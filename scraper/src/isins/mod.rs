use std::collections::HashSet;

use futures::{stream::FuturesUnordered, StreamExt};
use html_scraper::Html;
use tracing::{debug, info_span, warn, Instrument};
use types::{isin_token_from_href, ShareIsin};

use crate::{
    metrics::{ScrapingMetrics, WithMetrics},
    ScraperRuntime,
};

pub mod types;

pub async fn scrape_all_isins(runtime: &ScraperRuntime) -> WithMetrics<HashSet<ShareIsin>> {
    let mut metrics = ScrapingMetrics::empty();
    let mut tasks = FuturesUnordered::new();

    // WARN: it currently scrapes some ISINs multiple times
    //  if the last page for ISINs is X, scraping page X + Y still shows the ISINs at page X.
    // I need to use crawling for this...
    // TODO: use crawling to check pages
    for letter in b'A'..=b'Z' {
        for page in 1..=9 {
            let letter = letter as char;
            tasks.push(
                scrape_isins_at_page(runtime, letter, page).instrument(info_span!(
                    "scraping isins",
                    letter = letter.to_string(),
                    page = page
                )),
            );
        }
    }

    let mut res: HashSet<ShareIsin> = HashSet::new();

    while let Some(mut result) = tasks.next().await {
        res.extend(result.unmetric());
        metrics = metrics + result.metrics;
    }

    WithMetrics::new(res, metrics)
}

async fn scrape_isins_at_page(
    runtime: &ScraperRuntime,
    letter: char,
    page: u8,
) -> WithMetrics<HashSet<ShareIsin>> {
    debug!("Scraping ISINs at {} for letter {}", page, letter);

    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/listino-a-z.html?initial={}&page={}&lang=it",
        letter, page
    );

    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();

    let res_txt = runtime
        .get_page_text(url)
        .instrument(info_span!("fetching_page"))
        .await;

    match res_txt {
        Ok(txt) => {
            let mut isins = parse_page(txt);
            res.extend(isins.unmetric());
            metrics = metrics + isins.metrics;
        }
        Err(e) => metrics.errors.update(e),
    }

    debug!("Found {} ISINs", res.len());

    WithMetrics::new(res, metrics)
}

fn parse_page(res_txt: String) -> WithMetrics<HashSet<ShareIsin>> {
    debug!("Parsing ISIN page");

    let doc = Html::parse_document(&res_txt);
    let isin_element_selector = html_scraper::Selector::parse(
        "div[data-bb-view=\"list-aZ-stream\"] table.m-table.-firstlevel a.u-hidden.-xs",
    )
    .unwrap();

    let mut res: HashSet<ShareIsin> = HashSet::new();
    let mut metrics = ScrapingMetrics::empty();

    let all_isin_elements = doc.select(&isin_element_selector);

    all_isin_elements.for_each(|e| {
        metrics.total += 1;
        match ShareIsin::from_element(e) {
            Ok(result) => {
                metrics.successful += 1;
                res.insert(result);
            }
            Err(error) => {
                let href = e.attr("href").unwrap_or("<missing>");
                let token = isin_token_from_href(href).unwrap_or("<missing>");
                let company_name = e.text().collect::<Vec<_>>().join(" ");
                let company_name = company_name.trim();

                warn!(
                    href = %href,
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

#[cfg(test)]
mod tests {
    use super::parse_page;

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
}
