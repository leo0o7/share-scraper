use super::gen_macro::*;
use crate::generate_scrapable_struct;
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
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
