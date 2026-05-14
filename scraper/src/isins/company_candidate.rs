use std::collections::HashSet;

use html_scraper::{ElementRef, Html, Selector};
use once_cell::sync::Lazy;
use tracing::{debug, warn};

use crate::{
    metrics::{ScrapingMetrics, WithMetrics},
    shares::parsers::SafeParse,
};

use super::types::{isin_token_from_href, ShareIsin};

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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct CompanyCandidate {
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

pub(super) fn extract_company_elements<'a>(doc: &'a Html) -> Vec<ElementRef<'a>> {
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

pub(super) fn parse_elements(
    isin_elements: Vec<ElementRef<'_>>,
) -> WithMetrics<HashSet<ShareIsin>> {
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
