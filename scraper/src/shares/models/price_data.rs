use super::gen_macro::*;
use crate::generate_scrapable_struct;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use super::{PriceDateReference, PriceDateTimeReference};

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceData {
    pub isin: String,
    pub fase_di_mercato: Option<String>,
    pub prezzo_ultimo_contratto: Option<f64>,
    pub var_percentuale: Option<f64>,
    pub var_assoluta: Option<f64>,
    pub pr_medio_progr: Option<f64>,
    pub data_ora_ultimo_contratto: Option<NaiveDateTime>,
    pub quantita_ultimo: Option<f64>,
    pub quantita_totale: Option<f64>,
    pub numero_contratti: Option<u64>,
    pub controvalore: Option<f64>,
    pub max_oggi: Option<f64>,
    pub max_anno: Option<PriceDateReference>,
    pub min_oggi: Option<f64>,
    pub min_anno: Option<PriceDateReference>,
    pub chiusura_precedente: Option<f64>,
    pub prezzo_riferimento: Option<PriceDateTimeReference>,
    pub prezzo_ufficiale: Option<PriceDateReference>,
    pub apertura_odierna: Option<f64>,
}

generate_scrapable_struct!(PriceData, {
    fase_di_mercato: String,
    prezzo_ultimo_contratto: f64,
    var_percentuale: f64,
    var_assoluta: f64,
    pr_medio_progr: f64,
    data_ora_ultimo_contratto: NaiveDateTime,
    quantita_ultimo: f64,
    quantita_totale: f64,
    numero_contratti: u64,
    controvalore: f64,
    max_oggi: f64,
    max_anno: PriceDateReference,
    min_oggi: f64,
    min_anno: PriceDateReference,
    chiusura_precedente: f64,
    prezzo_riferimento: PriceDateTimeReference,
    prezzo_ufficiale: PriceDateReference,
    apertura_odierna: f64,
});
