use crate::shares::models::ElementRef;
use crate::shares::parsers::DefaultParse;
use crate::shares::selectors::select_for_prop;
use chrono::NaiveDateTime;
use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::generate_from_element;

use super::{PriceDateReference, PriceDateTimeReference};

#[derive(Derivative, Debug, Serialize, Deserialize)]
#[derivative(Default)]
pub struct PriceData {
    pub isin: String,
    #[derivative(Default(value = "\"N/A\".to_string()"))]
    pub fase_di_mercato: String,
    pub prezzo_ultimo_contratto: f64,
    pub var_percentuale: f64,
    pub var_assoluta: f64,
    pub pr_medio_progr: f64,
    pub data_ora_ultimo_contratto: NaiveDateTime,
    pub quantita_ultimo: f64,
    pub quantita_totale: f64,
    pub numero_contratti: u64,
    pub controvalore: f64,
    pub max_oggi: f64,
    pub max_anno: PriceDateReference,
    pub min_oggi: f64,
    pub min_anno: PriceDateReference,
    pub chiusura_precedente: f64,
    pub prezzo_riferimento: PriceDateTimeReference,
    pub prezzo_ufficiale: PriceDateReference,
    pub apertura_odierna: f64,
}

generate_from_element!(PriceData, {
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
