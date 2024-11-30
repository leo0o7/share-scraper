use crate::shares::models::ElementRef;
use crate::shares::parsers::DefaultParse;
use crate::shares::selectors::select_for_prop;
use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::generate_from_element;
#[derive(Derivative, Debug, Serialize, Deserialize)]
#[derivative(Default)]
pub struct MarketInformation {
    pub isin: String,
    #[derivative(Default(value = "\"N/A\".to_string()"))]
    pub super_sector: String,
    #[derivative(Default(value = "\"N/A\".to_string()"))]
    pub mercato_segmento: String,
    pub capitalizzazione_di_mercato: f64,
    pub lotto_minimo: f64,
}

generate_from_element!(MarketInformation, {
    super_sector: String ,
    mercato_segmento: String,
    capitalizzazione_di_mercato: f64,
    lotto_minimo: f64,
});
