mod gen_macro;
mod market_information;
mod performance_metrics;
mod price_data;
pub mod share;
mod share_details;

use crate::isins::types::ShareIsin;

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use super::property_selector::PropertySelector;

pub trait ScrapableStruct {
    fn from_selector(share_isin: &ShareIsin, selector: &PropertySelector) -> Self;
    fn with_isin(share_isin: &ShareIsin) -> Self;
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct PriceDateReference {
    pub price: Option<f64>,
    pub date: Option<NaiveDate>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct PriceDateTimeReference {
    pub price: Option<f64>,
    pub datetime: Option<NaiveDateTime>,
}
