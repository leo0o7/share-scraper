use super::gen_macro::*;
use crate::generate_scrapable_struct;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::prelude::FromRow;
use sqlx::Row;

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

impl FromRow<'_, PgRow> for PriceData {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            isin: row.try_get("isin")?,
            fase_di_mercato: row.try_get("fase_di_mercato")?,
            prezzo_ultimo_contratto: row.try_get("prezzo_ultimo_contratto")?,
            var_percentuale: row.try_get("var_percentuale")?,
            var_assoluta: row.try_get("var_assoluta")?,
            pr_medio_progr: row.try_get("pr_medio_progr")?,
            data_ora_ultimo_contratto: row.try_get("data_ora_ultimo_contratto")?,
            quantita_ultimo: row.try_get("quantita_ultimo")?,
            quantita_totale: row.try_get("quantita_totale")?,
            numero_contratti: row
                .try_get::<Option<i32>, _>("numero_contratti")?
                .map(|v| v as u64),
            controvalore: row.try_get("controvalore")?,
            max_oggi: row.try_get("max_oggi")?,
            max_anno: Some(PriceDateReference {
                price: row.try_get("max_anno")?,
                date: row.try_get("max_anno_date")?,
            }),
            min_oggi: row.try_get("min_oggi")?,
            min_anno: Some(PriceDateReference {
                price: row.try_get("min_anno")?,
                date: row.try_get("min_anno_date")?,
            }),
            chiusura_precedente: row.try_get("chiusura_precedente")?,
            prezzo_riferimento: Some(PriceDateTimeReference {
                price: row.try_get("prezzo_riferimento")?,
                datetime: row.try_get("data_ora_prezzo_rifermento")?,
            }),
            prezzo_ufficiale: Some(PriceDateReference {
                price: row.try_get("prezzo_ufficiale")?,
                date: row.try_get("data_prezzo_ufficiale")?,
            }),
            apertura_odierna: row.try_get("apertura_odierna")?,
        })
    }
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
