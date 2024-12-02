use std::num::{ParseFloatError, ParseIntError};

use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use scraper::ElementRef;

use super::models::{PriceDateReference, PriceDateTimeReference};

pub trait DefaultParse<T> {
    fn default_parse(&self) -> T;
}

pub trait SafeParse<T> {
    fn safe_parse(&self) -> Option<T>;
}

impl SafeParse<f64> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<f64> {
        self.text().next().and_then(|text| parse_float(text).ok())
    }
}

impl SafeParse<String> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<String> {
        self.text().next().map(|s| s.to_owned())
    }
}

impl SafeParse<NaiveDateTime> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<NaiveDateTime> {
        self.text()
            .next()
            .and_then(|text| parse_datetime(&text).ok())
    }
}

impl SafeParse<u64> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<u64> {
        self.text().next().and_then(|text| parse_int(&text).ok())
    }
}

impl SafeParse<PriceDateReference> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<PriceDateReference> {
        let price_date_str: String = self.default_parse();

        price_date_str.split_once(" - ").and_then(|tuple| {
            let price = parse_float(tuple.0).ok();
            let date = parse_date(tuple.1).ok();

            Some(PriceDateReference { price, date })
        })
    }
}

impl SafeParse<PriceDateTimeReference> for ElementRef<'_> {
    fn safe_parse(&self) -> Option<PriceDateTimeReference> {
        let price_datetime_str: String = self.default_parse();

        price_datetime_str.split_once("-").and_then(|tuple| {
            let price = parse_float(tuple.0.trim()).ok();
            let datetime = parse_datetime(tuple.1.trim()).ok();

            Some(PriceDateTimeReference { price, datetime })
        })
    }
}

impl DefaultParse<f64> for ElementRef<'_> {
    fn default_parse(&self) -> f64 {
        self.text()
            .next()
            .and_then(|text| parse_float(text).ok())
            .unwrap_or_default()
    }
}

impl DefaultParse<String> for ElementRef<'_> {
    fn default_parse(&self) -> String {
        self.text().next().unwrap_or("N/A").to_owned()
    }
}

impl DefaultParse<NaiveDateTime> for ElementRef<'_> {
    fn default_parse(&self) -> NaiveDateTime {
        self.text()
            .next()
            .and_then(|text| parse_datetime(&text).ok())
            .unwrap_or_default()
    }
}

impl DefaultParse<u64> for ElementRef<'_> {
    fn default_parse(&self) -> u64 {
        self.safe_parse().unwrap_or(0)
    }
}

impl DefaultParse<PriceDateReference> for ElementRef<'_> {
    fn default_parse(&self) -> PriceDateReference {
        self.safe_parse().unwrap_or_default()
        // let price_date_str: String = self.default_parse();
        //
        // price_date_str
        //     .split_once(" - ")
        //     .and_then(|tuple| {
        //         let price = parse_float(tuple.0).unwrap_or(0.0);
        //         let date = parse_date(tuple.1).unwrap_or(NaiveDate::default());
        //
        //         Some(PriceDateReference {
        //             price: Some(price),
        //             date: Some(date),
        //         })
        //     })
        //     .unwrap_or_default()
    }
}

impl DefaultParse<PriceDateTimeReference> for ElementRef<'_> {
    fn default_parse(&self) -> PriceDateTimeReference {
        let price_datetime_str: String = self.default_parse();

        price_datetime_str
            .split_once("-")
            .and_then(|tuple| {
                let price = parse_float(tuple.0.trim()).unwrap_or(0.0);
                let datetime = parse_datetime(tuple.1.trim()).unwrap_or(NaiveDateTime::default());

                Some(PriceDateTimeReference {
                    price: Some(price),
                    datetime: Some(datetime),
                })
            })
            .unwrap_or_default()
    }
}

fn parse_int(str: &str) -> Result<u64, ParseIntError> {
    str.trim().replace(".", "").parse()
}

fn parse_float(text: &str) -> Result<f64, ParseFloatError> {
    let dots_as_thousands_separator = Regex::new(r"^(\d{1,3})(\.?\d{3})*(,\d+)?$").unwrap();

    let cleaned = text
        .trim()
        .trim_start_matches("+")
        .trim_end_matches("%")
        .trim();

    let is_negative = cleaned.starts_with("-");

    let cleaned = cleaned.trim_start_matches("-");

    let normalized = if dots_as_thousands_separator.is_match(cleaned) {
        cleaned.replace(".", "").replace(",", ".")
    } else {
        cleaned.replace(",", "")
    };

    normalized
        .parse()
        .map(|val: f64| if is_negative { -val } else { val })
}

fn parse_datetime(str: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    // 29/11/24 16.07.46
    let fmt1 = "%d/%m/%y %H.%M.%S";
    // 29/11/24 - 16.07.46
    let fmt2 = "%d/%m/%y - %H.%M.%S";

    if let Ok(res) = NaiveDateTime::parse_from_str(str, fmt1) {
        return Ok(res);
    };

    // if first is invalid uses second format
    NaiveDateTime::parse_from_str(str, fmt2)
}

fn parse_date(str: &str) -> Result<NaiveDate, chrono::ParseError> {
    let fmt = "%d/%m/%y";
    NaiveDate::parse_from_str(str, fmt)
}
