use std::fmt::{Display, Formatter};
use std::hash::Hash;

use crate::shares::parsers::SafeParse;
use chrono::NaiveDateTime;
use html_scraper::{ElementRef, Selector};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::errors::{ScraperResult, ScrapingError};

static ISIN_SHARE_NAME_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("span.t-text").unwrap());

// derive for HashSet and other
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Isin {
    pub country: String,
    pub nna: String,
    pub check: u8,
}

impl TryFrom<String> for Isin {
    type Error = IsinError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 12 {
            return Err(IsinError::InvalidLength(value.len()));
        }

        let country = value[0..2].to_string();
        let nna = value[2..11].to_string();
        let check: u8 = value[11..12]
            .parse()
            .map_err(|_| IsinError::InvalidCheckDigit(value[11..12].to_string()))?;

        Ok(Isin {
            country,
            nna,
            check,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum IsinError {
    InvalidLength(usize),
    InvalidCheckDigit(String),
}

impl Isin {
    pub fn new(isin_str: String) -> Option<Isin> {
        Self::try_from(isin_str.to_string()).ok()
    }
}

impl Display for Isin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.country, self.nna, self.check)
    }
}

#[derive(Debug, sqlx::FromRow, Eq, Clone, Serialize, Deserialize)]
pub struct ShareIsin {
    pub share_name: String,
    #[sqlx(try_from = "String")]
    pub isin: Isin,
    pub updated_at: NaiveDateTime,
}

// don't include "updated_at" for HashSet
impl PartialEq for ShareIsin {
    fn eq(&self, other: &Self) -> bool {
        self.share_name == other.share_name && self.isin == other.isin
    }
}
impl Hash for ShareIsin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.share_name.hash(state);
        self.isin.hash(state);
    }
}

impl ShareIsin {
    pub fn new(name: String, isin_str: String) -> Option<Self> {
        if !name.is_empty() {
            Isin::new(isin_str).map(|isin| ShareIsin {
                share_name: name.to_string(),
                isin,
                updated_at: chrono::offset::Utc::now().naive_utc(),
            })
        } else {
            None
        }
    }

    pub fn from_element(isin_element: ElementRef) -> ScraperResult<Self> {
        debug!("Attempting to create ShareIsin from element");

        let share_link_attr = isin_element
            .attr("href")
            .ok_or(ScrapingError::InvalidPage)?;

        let isin_str = isin_token_from_href(share_link_attr).ok_or(ScrapingError::ParsingErr)?;
        debug!("ISIN string is {}", isin_str);

        let name: String = isin_element
            .select(&ISIN_SHARE_NAME_SELECTOR)
            .next()
            .and_then(|el| el.safe_parse())
            .ok_or(ScrapingError::InvalidPage)?;
        debug!("Name is {}", name);

        Self::new(name, isin_str.to_owned()).ok_or(ScrapingError::ParsingErr)
    }
}

// Borsa detail links put the ISIN in the last path segment and may append
// market suffixes or query params, for example `IT0005439861-EXGM.html?lang=it`.
pub(crate) fn isin_token_from_href(href: &str) -> Option<&str> {
    href.split('/')
        .next_back()
        .and_then(|s| s.split('.').next())
        .and_then(|s| s.split('-').next())
}
