use scraper::ElementRef;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Isin {
    pub country: String,
    pub nna: String,
    pub check: u8,
}

impl TryFrom<String> for Isin {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == 12 {
            let country = value[0..2].to_string();
            let nna = value[2..11].to_string();
            match value[11..12].parse::<u8>() {
                Ok(check) => Ok(Isin {
                    country,
                    nna,
                    check,
                }),
                Err(_) => Err(format!("Isin can't be parsed from {}", value)),
            }
        } else {
            Err(format!(
                "Isin can't be parsed from a string of lenght {}",
                value.len()
            ))
        }
    }
}

impl Isin {
    pub fn new(isin_str: &str) -> Option<Isin> {
        Isin::try_from(isin_str.to_string()).ok()
    }

    pub fn get_str(&self) -> String {
        format!("{}{}{}", self.country, self.nna, self.check)
    }
}

#[derive(Debug, sqlx::FromRow, Clone, Serialize, Deserialize)]
pub struct ShareIsin {
    pub share_name: String,
    #[sqlx(try_from = "String")]
    pub isin: Isin,
}

impl ShareIsin {
    pub fn new(name: &str, isin_str: &str) -> Option<ShareIsin> {
        if name.is_empty() {
            None
        } else {
            Isin::new(isin_str).map(|isin| ShareIsin {
                share_name: name.to_string(),
                isin,
            })
        }
    }

    pub fn from_element(isin_element: ElementRef) -> Option<ShareIsin> {
        info!("Creating ShareIsin from element");
        let isin_share_name_selector = scraper::Selector::parse("span.t-text").unwrap();

        let share_link_attr = isin_element.value().attr("href");
        let share_name_element = isin_element.select(&isin_share_name_selector).next();

        if let (Some(link), Some(name_element)) = (share_link_attr, share_name_element) {
            let isin_str = link
                .split("/")
                .last()
                .and_then(|s| s.split(".").next())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let name = name_element.text().next().unwrap_or_default();

            return ShareIsin::new(name, &isin_str);
        };

        None
    }
}
