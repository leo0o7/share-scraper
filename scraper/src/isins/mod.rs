use std::{collections::HashSet, future::Future};

use futures::{stream::FuturesUnordered, StreamExt};
use html_scraper::{ElementRef, Html};
use tracing::{debug, info_span, warn, Instrument};
use types::{isin_token_from_href, ShareIsin};

use crate::{
    errors::{ScraperResult, ScrapingError},
    metrics::{ScrapingMetrics, WithMetrics},
    shares::parsers::SafeParse,
    ScraperRuntime,
};

pub mod types;

const ISIN_PAGES_PER_LETTER: u8 = 9;

pub async fn scrape_all_isins(runtime: &ScraperRuntime) -> WithMetrics<HashSet<ShareIsin>> {
    let mut metrics = ScrapingMetrics::empty();
    let mut tasks = FuturesUnordered::new();

    for letter in b'A'..=b'Z' {
        let letter = letter as char;
        tasks.push(
            crawl_isins_for_letter(runtime, letter, ISIN_PAGES_PER_LETTER)
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

async fn crawl_isins_for_letter(
    runtime: &ScraperRuntime,
    letter: char,
    max_pages: u8,
) -> WithMetrics<HashSet<ShareIsin>> {
    crawl_isins_for_letter_with_fetcher(letter, max_pages, |page| async move {
        fetch_isins_page(runtime, letter, page).await
    })
    .await
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

    for page in 1..=max_pages {
        debug!("Scraping ISINs at {} for letter {}", page, letter);

        match fetch_page(page).await {
            Ok(txt) => {
                let candidates = extract_company_candidates(&txt);
                let current_candidates = candidates.iter().cloned().collect::<HashSet<_>>();

                if previous_candidates.as_ref() == Some(&current_candidates) {
                    debug!("Found repeated ISIN page {} for letter {}", page, letter);
                    break;
                }

                previous_candidates = Some(current_candidates);

                let mut isins = parse_candidates(candidates);
                res.extend(isins.unmetric());
                metrics = metrics + isins.metrics;
            }
            Err(e) => metrics.errors.update(e),
        }
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

    parse_candidates(extract_company_candidates(&res_txt))
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CompanyCandidate {
    href: String,
    name: Option<String>,
}

fn extract_company_candidates(res_txt: &str) -> Vec<CompanyCandidate> {
    let doc = Html::parse_document(res_txt);
    let desktop_company_selector = html_scraper::Selector::parse(
        "div[data-bb-view=\"list-aZ-stream\"] table.m-table.-firstlevel a.u-hidden.-xs",
    )
    .unwrap();
    let company_link_selector = html_scraper::Selector::parse(
        "div[data-bb-view=\"list-aZ-stream\"] table.m-table.-firstlevel a[href]",
    )
    .unwrap();

    let mut isin_elements: Vec<_> = doc
        .select(&desktop_company_selector)
        .filter(is_company_detail_element)
        .collect();

    if isin_elements.is_empty() {
        let mut seen_hrefs = HashSet::new();
        isin_elements = doc
            .select(&company_link_selector)
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
        .into_iter()
        .map(company_candidate_from_element)
        .collect()
}

fn company_candidate_from_element(element: ElementRef<'_>) -> CompanyCandidate {
    let name_selector = html_scraper::Selector::parse("span.t-text").unwrap();
    let name = element
        .select(&name_selector)
        .next()
        .and_then(|el| el.safe_parse());

    CompanyCandidate {
        href: element.attr("href").unwrap_or_default().to_string(),
        name,
    }
}

fn parse_candidates(candidates: Vec<CompanyCandidate>) -> WithMetrics<HashSet<ShareIsin>> {
    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();

    candidates.into_iter().for_each(|candidate| {
        metrics.total += 1;
        match share_isin_from_candidate(&candidate) {
            Ok(share_isin) => {
                metrics.successful += 1;
                res.insert(share_isin);
            }
            Err(error) => {
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

fn share_isin_from_candidate(candidate: &CompanyCandidate) -> ScraperResult<ShareIsin> {
    let isin_str = isin_token_from_href(&candidate.href).ok_or(ScrapingError::ParsingErr)?;
    let name = candidate.name.clone().ok_or(ScrapingError::InvalidPage)?;

    ShareIsin::new(name, isin_str.to_owned()).ok_or(ScrapingError::ParsingErr)
}

fn is_company_detail_element(element: &ElementRef<'_>) -> bool {
    element.attr("href").is_some_and(is_company_detail_href)
}

fn is_company_detail_href(href: &str) -> bool {
    href.starts_with("/borsa/azioni/scheda/")
        && href.ends_with(".html")
        && isin_token_from_href(href).is_some()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque, future::ready, rc::Rc};

    use crate::errors::ScrapingError;

    use super::{crawl_isins_for_letter_with_fetcher, parse_page};

    #[test]
    fn parses_company_href_with_market_suffix_as_website_isin_token() {
        let html = r#"
            <div data-bb-view="list-aZ-stream">
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT0005439861-EXGM.html">
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
    async fn stops_on_repeated_letter_page_without_counting_sentinel_records_or_metrics() {
        let repeated_page = r#"
            <div data-bb-view="list-aZ-stream">
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT0003128367.html">
                                <span class="t-text">Enel</span>
                            </a>
                            <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT000000000X.html">
                                <span class="t-text">Malformed</span>
                            </a>
                        </td>
                    </tr>
                </table>
            </div>
        "#;
        let later_page = r#"
            <div data-bb-view="list-aZ-stream">
                <table class="m-table -firstlevel">
                    <tr>
                        <td>
                            <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT0000072618.html">
                                <span class="t-text">Intesa Sanpaolo</span>
                            </a>
                        </td>
                    </tr>
                </table>
            </div>
        "#;
        let pages = Rc::new(RefCell::new(VecDeque::from([
            repeated_page.to_string(),
            repeated_page.to_string(),
            later_page.to_string(),
        ])));
        let fetch_count = Rc::new(RefCell::new(0));

        let mut crawled = crawl_isins_for_letter_with_fetcher('E', 3, {
            let pages = Rc::clone(&pages);
            let fetch_count = Rc::clone(&fetch_count);
            move |_| {
                *fetch_count.borrow_mut() += 1;
                let page = pages
                    .borrow_mut()
                    .pop_front()
                    .expect("expected fixture page");
                ready(Ok::<_, ScrapingError>(page))
            }
        })
        .await;

        let isins = crawled.unmetric();

        assert_eq!(*fetch_count.borrow(), 2);
        assert_eq!(crawled.metrics.total, 2);
        assert_eq!(crawled.metrics.successful, 1);
        assert_eq!(crawled.metrics.errors.parsing_error, 1);
        assert_eq!(isins.len(), 1);
        assert!(isins
            .iter()
            .any(|share| share.share_name == "Enel" && share.isin.to_string() == "IT0003128367"));
        assert!(!isins
            .iter()
            .any(|share| share.share_name == "Intesa Sanpaolo"));
    }
}
