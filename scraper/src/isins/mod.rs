use std::{collections::HashSet, future::Future};

use futures::{stream::FuturesUnordered, StreamExt};
use html_scraper::Html;
use tracing::{debug, info_span, warn, Instrument};
use types::ShareIsin;

use crate::{
    errors::ScraperResult,
    metrics::{ScrapingMetrics, WithMetrics},
    ScraperRuntime,
};

mod company_candidate;
pub mod types;

use company_candidate::{
    company_page_signature, extract_company_elements, parse_elements, CompanyPageSignature,
};

struct IsinCrawler<I, F> {
    letters: I,
    max_pages: u8,
    fetch_page: F,
}

impl<I, F> IsinCrawler<I, F> {
    fn new(letters: I, max_pages: u8, fetch_page: F) -> Self {
        Self {
            letters,
            max_pages,
            fetch_page,
        }
    }
}

impl<I, F, Fut> IsinCrawler<I, F>
where
    I: IntoIterator<Item = u8>,
    F: Fn(u8, u8) -> Fut + Clone,
    Fut: Future<Output = ScraperResult<String>>,
{
    #[cfg(test)]
    async fn crawl(self) -> WithMetrics<HashSet<ShareIsin>> {
        self.crawl_observing(NoopIsinCrawlProgress).await
    }

    async fn crawl_observing<P>(self, progress: P) -> WithMetrics<HashSet<ShareIsin>>
    where
        P: IsinCrawlProgress,
    {
        let mut metrics = ScrapingMetrics::empty();
        let mut tasks = FuturesUnordered::new();

        for letter in self.letters {
            let letter = letter as char;
            let fetch_page = self.fetch_page.clone();
            let progress = progress.clone();
            tasks.push(
                crawl_isins_for_letter_with_fetcher(
                    letter,
                    self.max_pages,
                    move |page| fetch_page(letter as u8, page),
                    progress,
                )
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
}

pub async fn scrape_all_isins(runtime: &ScraperRuntime) -> WithMetrics<HashSet<ShareIsin>> {
    scrape_all_isins_with_progress(runtime, NoopIsinCrawlProgress).await
}

pub trait IsinCrawlProgress: Clone {
    fn page_scraped(
        &self,
        letter: char,
        page: u8,
        isins_found: u64,
        result: ScraperResult<()>,
        parsing_errors: u64,
    );

    fn letter_completed(&self, letter: char);
}

#[derive(Clone)]
pub struct NoopIsinCrawlProgress;

impl IsinCrawlProgress for NoopIsinCrawlProgress {
    fn page_scraped(
        &self,
        _letter: char,
        _page: u8,
        _isins_found: u64,
        _result: ScraperResult<()>,
        _parsing_errors: u64,
    ) {
    }

    fn letter_completed(&self, _letter: char) {}
}

pub async fn scrape_all_isins_with_progress<P>(
    runtime: &ScraperRuntime,
    progress: P,
) -> WithMetrics<HashSet<ShareIsin>>
where
    P: IsinCrawlProgress,
{
    IsinCrawler::new(
        b'A'..=b'Z',
        runtime.isin_max_pages_per_letter(),
        |letter, page| async move { fetch_isins_page(runtime, letter as char, page).await },
    )
    .crawl_observing(progress)
    .await
}

async fn crawl_isins_for_letter_with_fetcher<F, Fut, P>(
    letter: char,
    max_pages: u8,
    mut fetch_page: F,
    progress: P,
) -> WithMetrics<HashSet<ShareIsin>>
where
    F: FnMut(u8) -> Fut,
    Fut: Future<Output = ScraperResult<String>>,
    P: IsinCrawlProgress,
{
    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();
    let mut seen_signatures: HashSet<CompanyPageSignature> = HashSet::new();
    let mut repeated_page_found = false;

    for page in 1..=max_pages {
        debug!("Scraping ISINs at {} for letter {}", page, letter);

        match fetch_page(page).await {
            Ok(txt) => {
                let doc = Html::parse_document(&txt);
                let isin_elements = extract_company_elements(&doc);
                let current_signature = company_page_signature(&isin_elements);

                if !seen_signatures.insert(current_signature) {
                    progress.page_scraped(letter, page, 0, Ok(()), 0);
                    debug!("Found repeated ISIN page {} for letter {}", page, letter);
                    repeated_page_found = true;
                    break;
                }

                let mut isins = parse_elements(isin_elements);
                let isins_found = isins.metrics.successful as u64;
                let parsing_errors = isins.metrics.errors.parsing_error as u64;
                progress.page_scraped(letter, page, isins_found, Ok(()), parsing_errors);
                res.extend(isins.unmetric());
                metrics = metrics + isins.metrics;
            }
            Err(e) => {
                progress.page_scraped(letter, page, 0, Err(e.clone()), 0);
                metrics.errors.update(e);
            }
        }
    }

    progress.letter_completed(letter);

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

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::errors::ScrapingError;
    use tokio::sync::{mpsc, Notify};

    use super::{parse_page, IsinCrawler};

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
                IsinCrawler::new(b'A'..=b'Z', 2, move |letter, page| {
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
                .crawl()
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

        let mut capped = IsinCrawler::new(b'A'..=b'A', 1, move |letter, page| async move {
            assert_eq!(page, 1);
            Ok::<_, ScrapingError>(page_html(letter))
        })
        .crawl()
        .await;
        let capped_isins = capped.unmetric();

        assert_eq!(capped_isins.len(), 1);
        assert_eq!(capped.metrics.total, 2);
        assert_eq!(capped.metrics.successful, 1);
        assert_eq!(capped.metrics.errors.parsing_error, 1);
    }

    #[tokio::test]
    async fn repeated_page_detection_uses_stable_isin_signature() {
        fn page_html(name: &str) -> String {
            format!(
                r#"
                <div data-bb-view="list-aZ-stream">
                    <table class="m-table -firstlevel">
                        <tr>
                            <td>
                                <a class="u-hidden -xs" href="/borsa/azioni/scheda/IT0003128367-MTAA.html?lang=it">
                                    <span class="t-text">{name}</span>
                                </a>
                            </td>
                        </tr>
                    </table>
                </div>
                "#
            )
        }

        let mut scraped = IsinCrawler::new(b'S'..=b'S', 2, move |_letter, page| async move {
            let name = if page == 1 {
                "Stable Name"
            } else {
                "Changed Name"
            };

            Ok::<_, ScrapingError>(page_html(name))
        })
        .crawl()
        .await;
        let isins = scraped.unmetric();

        assert_eq!(isins.len(), 1);
        assert_eq!(scraped.metrics.total, 1);
        assert_eq!(scraped.metrics.successful, 1);

        let share = isins.iter().next().expect("expected parsed share isin");
        assert_eq!(share.share_name, "Stable Name");
        assert_eq!(share.isin.to_string(), "IT0003128367");
    }

    #[tokio::test]
    async fn repeated_page_detection_stops_on_non_adjacent_repeats() {
        fn page_html(isin: &str) -> String {
            format!(
                r#"
                <div data-bb-view="list-aZ-stream">
                    <table class="m-table -firstlevel">
                        <tr>
                            <td>
                                <a class="u-hidden -xs" href="/borsa/azioni/scheda/{isin}-MTAA.html?lang=it">
                                    <span class="t-text">{isin}</span>
                                </a>
                            </td>
                        </tr>
                    </table>
                </div>
                "#
            )
        }

        let mut scraped = IsinCrawler::new(b'C'..=b'C', 3, move |_letter, page| async move {
            let isin = match page {
                1 | 3 => "IT0003128367",
                2 => "IT0000072618",
                _ => unreachable!(),
            };

            Ok::<_, ScrapingError>(page_html(isin))
        })
        .crawl()
        .await;
        let isins = scraped.unmetric();

        assert_eq!(isins.len(), 2);
        assert_eq!(scraped.metrics.total, 2);
        assert_eq!(scraped.metrics.successful, 2);
    }
}
