use crate::shares::models::ElementRef;
use crate::shares::models::ScrapableStruct;
use crate::shares::parsers::SafeParse;
use crate::shares::selectors::select_for_prop;
use crate::shares::ShareIsin;
use serde::{Deserialize, Serialize};

use crate::generate_scrapable_struct;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MarketInformation {
    pub isin: String,
    pub super_sector: Option<String>,
    pub mercato_segmento: Option<String>,
    pub capitalizzazione_di_mercato: Option<f64>,
    pub lotto_minimo: Option<f64>,
}

generate_scrapable_struct!(MarketInformation, {
    super_sector: String ,
    mercato_segmento: String,
    capitalizzazione_di_mercato: f64,
    lotto_minimo: f64,
});
