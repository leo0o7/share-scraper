use std::num::{ParseFloatError, ParseIntError};

use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use scraper::{element_ref::Select, ElementRef};

pub fn parse_float_or_default(element: ElementRef) -> f64 {
    element
        .text()
        .next()
        .and_then(|text| parse_float(text).ok())
        .unwrap_or_default()
}

pub fn parse_str_or_default(element: ElementRef) -> String {
    element.text().next().unwrap_or("N/A").to_owned()
}

pub fn parse_datetime_or_default(element: ElementRef) -> NaiveDateTime {
    element
        .text()
        .next()
        .and_then(|text| parse_datetime(&text).ok())
        .unwrap_or(NaiveDateTime::default())
}

pub fn parse_date_or_default(mut element: Select) -> NaiveDate {
    element
        .next()
        .and_then(|el| el.text().next())
        .and_then(|text| parse_date(&text).ok())
        .unwrap_or(NaiveDate::default())
}

pub fn parse_int_or_default(element: ElementRef) -> u64 {
    element
        .text()
        .next()
        .and_then(|text| parse_int(&text).ok())
        .unwrap_or(0)
}

fn parse_int(str: &str) -> Result<u64, ParseIntError> {
    str.trim().replace(".", "").parse()
}

pub fn parse_float(text: &str) -> Result<f64, ParseFloatError> {
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

pub fn parse_datetime(str: &str) -> Result<NaiveDateTime, chrono::ParseError> {
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

pub fn parse_date(str: &str) -> Result<NaiveDate, chrono::ParseError> {
    let fmt = "%d/%m/%y";
    NaiveDate::parse_from_str(str, fmt)
}
