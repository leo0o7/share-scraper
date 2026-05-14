use std::{collections::HashSet, future::Future};

use futures::{stream::FuturesUnordered, StreamExt};
use html_scraper::{ElementRef, Html, Selector};
use once_cell::sync::Lazy;
use tracing::{debug, info_span, warn, Instrument};
use types::{isin_token_from_href, ShareIsin};

use crate::{
    errors::ScraperResult,
    metrics::{ScrapingMetrics, WithMetrics},
    shares::parsers::SafeParse,
    ScraperRuntime,
};

pub mod types;

static DESKTOP_COMPANY_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("div[data-bb-view=\"list-aZ-stream\"] table.m-table.-firstlevel a.u-hidden.-xs")
        .unwrap()
});
static COMPANY_LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("div[data-bb-view=\"list-aZ-stream\"] table.m-table.-firstlevel a[href]")
        .unwrap()
});
static COMPANY_NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("span.t-text").unwrap());

pub async fn scrape_all_isins(runtime: &ScraperRuntime) -> WithMetrics<HashSet<ShareIsin>> {
    scrape_all_isins_with_fetcher(
        b'A'..=b'Z',
        runtime.isin_max_pages_per_letter(),
        |letter, page| async move { fetch_isins_page(runtime, letter as char, page).await },
    )
    .await
}

async fn scrape_all_isins_with_fetcher<I, F, Fut>(
    letters: I,
    max_pages: u8,
    fetch_page: F,
) -> WithMetrics<HashSet<ShareIsin>>
where
    I: IntoIterator<Item = u8>,
    F: Fn(u8, u8) -> Fut + Clone,
    Fut: Future<Output = ScraperResult<String>>,
{
    let mut metrics = ScrapingMetrics::empty();
    let mut tasks = FuturesUnordered::new();

    for letter in letters {
        let letter = letter as char;
        let fetch_page = fetch_page.clone();
        tasks.push(
            crawl_isins_for_letter_with_fetcher(letter, max_pages, move |page| {
                fetch_page(letter as u8, page)
            })
            .instrument(info_span!("scraping isins", letter = letter.to_string())),
        );
    }

    let mut res: HashSet<ShareIsin> = HashSet::new();

    while let Some(mut result) = tasks.next().await {
        res.extend(result.unmetric());
        metrics = metrics + result.metrics;
    }

    WithMetrics::new(res, metrics)
}

async fn crawl_isins_for_letter_with_fetcher<F, Fut>(
    letter: char,
    max_pages: u8,
    mut fetch_page: F,
) -> WithMetrics<HashSet<ShareIsin>>
where
    F: FnMut(u8) -> Fut,
    Fut: Future<Output = ScraperResult<String>>,
{
    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();
    let mut previous_candidates: Option<HashSet<CompanyCandidate>> = None;
    let mut repeated_page_found = false;

    for page in 1..=max_pages {
        debug!("Scraping ISINs at {} for letter {}", page, letter);

        match fetch_page(page).await {
            Ok(txt) => {
                let doc = Html::parse_document(&txt);
                let isin_elements = extract_company_elements(&doc);
                let current_candidates = isin_elements
                    .iter()
                    .map(|element| CompanyCandidate::from(*element))
                    .collect::<HashSet<_>>();

                if previous_candidates.as_ref() == Some(&current_candidates) {
                    debug!("Found repeated ISIN page {} for letter {}", page, letter);
                    repeated_page_found = true;
                    break;
                }

                previous_candidates = Some(current_candidates);

                let mut isins = parse_elements(isin_elements);
                res.extend(isins.unmetric());
                metrics = metrics + isins.metrics;
            }
            Err(e) => metrics.errors.update(e),
        }
    }

    if !repeated_page_found {
        warn!(
            letter = %letter,
            max_pages,
            "ISIN letter page cap reached before repeated page was detected; returning partial results"
        );
    }

    debug!("Found {} ISINs", res.len());

    WithMetrics::new(res, metrics)
}

async fn fetch_isins_page(
    runtime: &ScraperRuntime,
    letter: char,
    page: u8,
) -> ScraperResult<String> {
    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/listino-a-z.html?initial={}&page={}&lang=it",
        letter, page
    );

    runtime
        .get_page_text(url)
        .instrument(info_span!("fetching_page"))
        .await
}

#[cfg(test)]
fn parse_page(res_txt: String) -> WithMetrics<HashSet<ShareIsin>> {
    debug!("Parsing ISIN page");

    let doc = Html::parse_document(&res_txt);
    parse_elements(extract_company_elements(&doc))
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CompanyCandidate {
    href: String,
    name: Option<String>,
}

impl From<ElementRef<'_>> for CompanyCandidate {
    fn from(element: ElementRef<'_>) -> Self {
        CompanyCandidate {
            href: element.attr("href").unwrap_or_default().to_string(),
            name: parse_company_name(element),
        }
    }
}

fn extract_company_elements<'a>(doc: &'a Html) -> Vec<ElementRef<'a>> {
    let mut isin_elements: Vec<_> = doc
        .select(&DESKTOP_COMPANY_SELECTOR)
        .filter(is_company_detail_element)
        .collect();

    if isin_elements.is_empty() {
        let mut seen_hrefs = HashSet::new();
        isin_elements = doc
            .select(&COMPANY_LINK_SELECTOR)
            .filter(is_company_detail_element)
            .filter(|element| {
                element
                    .attr("href")
                    .map(|href| seen_hrefs.insert(href.to_string()))
                    .unwrap_or(false)
            })
            .collect();
    }

    isin_elements
}

fn parse_company_name(element: ElementRef<'_>) -> Option<String> {
    element
        .select(&COMPANY_NAME_SELECTOR)
        .next()
        .and_then(|el| el.safe_parse())
}

fn parse_elements(isin_elements: Vec<ElementRef<'_>>) -> WithMetrics<HashSet<ShareIsin>> {
    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();

    isin_elements.into_iter().for_each(|element| {
        metrics.total += 1;
        match ShareIsin::from_element(element) {
            Ok(share_isin) => {
                metrics.successful += 1;
                res.insert(share_isin);
            }
            Err(error) => {
                let candidate = CompanyCandidate::from(element);
                let token = isin_token_from_href(&candidate.href).unwrap_or("<missing>");
                let company_name = candidate.name.as_deref().unwrap_or("<missing>");

                warn!(
                    href = %candidate.href,
                    token = %token,
                    company_name = %company_name,
                    error = ?error,
                    "ISIN creation failed"
                );
                metrics.errors.update(error)
            }
        }
    });
    debug!("Metrics for parsing: {:?}", metrics);

    WithMetrics::new(res, metrics)
}

fn is_company_detail_element(element: &ElementRef<'_>) -> bool {
    element.attr("href").is_some_and(is_company_detail_href)
}

fn is_company_detail_href(href: &str) -> bool {
    href.starts_with("/borsa/azioni/")
        && href.contains("/scheda/")
        && isin_token_from_href(href).is_some()
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::errors::ScrapingError;
    use tokio::sync::{mpsc, Notify};

    use super::{parse_page, scrape_all_isins_with_fetcher};

    #[test]
    fn parses_company_href_with_market_suffix_as_website_isin_token() {
        let html = r#"
            <div data-bb-view="list-aZ-stream">
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-hidden -xs" href="/borsa/azioni/euronext-growth-milan/scheda/IT0005439861-EXGM.html?lang=it">
                                <span class="t-text">Example Share</span>
                            </a>
                        </td>
                    </tr>
                </table>
            </div>
        "#;

        let mut parsed = parse_page(html.to_string());
        let isins = parsed.unmetric();

        assert_eq!(parsed.metrics.total, 1);
        assert_eq!(parsed.metrics.successful, 1);
        assert_eq!(parsed.metrics.errors.parsing_error, 0);

        let share = isins.iter().next().expect("expected parsed share isin");
        assert_eq!(share.share_name, "Example Share");
        assert_eq!(share.isin.to_string(), "IT0005439861");
    }

    #[test]
    fn prefers_desktop_company_links_and_ignores_listing_controls() {
        let html = r#"
            <div data-bb-view="list-aZ-stream">
                <a href="/borsa/azioni/listino-a-z.html?initial=A">A</a>
                <a href="/borsa/azioni/listino-a-z.html?initial=A&page=2">2</a>
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-visible -xs" href="/borsa/azioni/scheda/IT0003128367.html">
                                <span class="t-text">Mobile Name Should Not Win</span>
                            </a>
                            <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT0003128367.html">
                                <span class="t-text">Enel</span>
                            </a>
                            <a href="/borsa/azioni/scheda/IT0003128367.html?add-to-portfolio=true">Portfolio</a>
                            <a href="/borsa/azioni/scheda/IT0000072618.html">
                                <span class="t-text">Fallback Link Should Not Be Used</span>
                            </a>
                        </td>
                    </tr>
                </table>
            </div>
        "#;

        let mut parsed = parse_page(html.to_string());
        let isins = parsed.unmetric();

        assert_eq!(parsed.metrics.total, 1);
        assert_eq!(parsed.metrics.successful, 1);
        assert_eq!(parsed.metrics.errors.parsing_error, 0);
        assert_eq!(isins.len(), 1);

        let share = isins.iter().next().expect("expected parsed share isin");
        assert_eq!(share.share_name, "Enel");
        assert_eq!(share.isin.to_string(), "IT0003128367");
    }

    #[test]
    fn falls_back_to_unique_company_links_when_desktop_links_are_absent() {
        let html = r#"
            <div data-bb-view="list-aZ-stream">
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-visible -xs" href="/borsa/azioni/scheda/IT0000072618.html">
                                <span class="t-text">Intesa Sanpaolo</span>
                            </a>
                            <a class="u-visible -xs" href="/borsa/azioni/scheda/IT0000072618.html">
                                <span class="t-text">Intesa Sanpaolo Duplicate</span>
                            </a>
                            <a class="u-visible -xs" href="/borsa/azioni/scheda/IT0005439861-EXGM.html">
                                <span class="t-text">Example Share</span>
                            </a>
                        </td>
                    </tr>
                </table>
            </div>
        "#;

        let mut parsed = parse_page(html.to_string());
        let isins = parsed.unmetric();

        assert_eq!(parsed.metrics.total, 2);
        assert_eq!(parsed.metrics.successful, 2);
        assert_eq!(parsed.metrics.errors.parsing_error, 0);
        assert_eq!(isins.len(), 2);
        assert!(isins.iter().any(|share| {
            share.share_name == "Intesa Sanpaolo" && share.isin.to_string() == "IT0000072618"
        }));
        assert!(isins.iter().any(|share| {
            share.share_name == "Example Share" && share.isin.to_string() == "IT0005439861"
        }));
    }

    #[tokio::test]
    async fn scrape_all_crawls_letters_concurrently_and_stops_before_repeated_or_capped_pages() {
        fn page_html(letter: u8) -> String {
            let index = letter - b'A' + 1;
            let isin = format!("IT{index:010}");
            let name = format!("Share {}", letter as char);
            let malformed = if letter == b'A' {
                r#"
                <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT000000000X.html">
                    <span class="t-text">Malformed</span>
                </a>
                "#
            } else {
                ""
            };

            format!(
                r#"
                <div data-bb-view="list-aZ-stream">
                    <table class="m-table -firstlevel">
                        <tr>
                            <td>
                                <a class="u-hidden -xs" href="/borsa/azioni/scheda/{isin}.html">
                                    <span class="t-text">{name}</span>
                                </a>
                                {malformed}
                            </td>
                        </tr>
                    </table>
                </div>
                "#
            )
        }

        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let release_first_pages = Arc::new(Notify::new());

        let scrape_task = tokio::spawn({
            let release_first_pages = Arc::clone(&release_first_pages);
            async move {
                scrape_all_isins_with_fetcher(b'A'..=b'Z', 2, move |letter, page| {
                    let started_tx = started_tx.clone();
                    let release_first_pages = Arc::clone(&release_first_pages);

                    async move {
                        started_tx.send((letter, page)).unwrap();

                        if page == 1 {
                            release_first_pages.notified().await;
                        }

                        Ok::<_, ScrapingError>(page_html(letter))
                    }
                })
                .await
            }
        });

        let mut first_starts = Vec::new();
        for _ in b'A'..=b'Z' {
            first_starts.push(
                tokio::time::timeout(Duration::from_secs(1), started_rx.recv())
                    .await
                    .expect("timed out waiting for letter crawl to start")
                    .expect("start channel closed before every letter started"),
            );
        }

        assert!(first_starts.iter().all(|(_, page)| *page == 1));

        release_first_pages.notify_waiters();

        let mut scraped = scrape_task.await.expect("scrape task should finish");
        let isins = scraped.unmetric();

        assert_eq!(isins.len(), 26);
        assert_eq!(scraped.metrics.total, 27);
        assert_eq!(scraped.metrics.successful, 26);
        assert_eq!(scraped.metrics.errors.parsing_error, 1);

        let mut capped =
            scrape_all_isins_with_fetcher(b'A'..=b'A', 1, move |letter, page| async move {
                assert_eq!(page, 1);
                Ok::<_, ScrapingError>(page_html(letter))
            })
            .await;
        let capped_isins = capped.unmetric();

        assert_eq!(capped_isins.len(), 1);
        assert_eq!(capped.metrics.total, 2);
        assert_eq!(capped.metrics.successful, 1);
        assert_eq!(capped.metrics.errors.parsing_error, 1);
    }
}
