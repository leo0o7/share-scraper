use scraper::ElementRef;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Isin {
    pub country: String,
    pub nna: String,
    pub check: u8,
}

impl Isin {
    pub fn new(isin_str: &str) -> Option<Isin> {
        if isin_str.len() == 12 {
            let country = isin_str[0..2].to_string();
            let nna = isin_str[2..11].to_string();
            let check = isin_str[11..12].parse::<u8>().ok()?;

            Some(Isin {
                country,
                nna,
                check,
            })
        } else {
            None
        }
    }

    pub fn get_str(&self) -> String {
        format!("{}{}{}", self.country, self.nna, self.check)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShareIsin {
    pub name: String,
    pub isin: Isin,
}

impl ShareIsin {
    pub fn new(name: &str, isin_str: &str) -> Option<ShareIsin> {
        if name.is_empty() {
            None
        } else {
            Isin::new(isin_str).map(|isin| ShareIsin {
                name: name.to_string(),
                isin,
            })
        }
    }
    pub fn from_element(isin_element: ElementRef) -> Option<ShareIsin> {
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

            return ShareIsin::new(&name, &isin_str);
        };

        None
    }
}
