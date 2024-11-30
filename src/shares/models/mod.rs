mod gen_macro;
mod market_information;
mod performance_metrics;
mod price_data;
mod share_details;

use crate::isins::types::{Isin, ShareIsin};

use chrono::{NaiveDate, NaiveDateTime};
use market_information::MarketInformation;
use performance_metrics::PerformanceMetrics;
use price_data::PriceData;
use scraper::ElementRef;
use serde::{Deserialize, Serialize};
use share_details::ShareDetails;
use sqlx::prelude::FromRow;

#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
pub struct Share {
    pub share_id: ShareIsin,
    pub share_details: ShareDetails,
    pub market_information: MarketInformation,
    pub price_data: PriceData,
    pub performance_metrics: PerformanceMetrics,
}

impl Share {
    pub fn from_element(share_isin: &ShareIsin, table: ElementRef) -> Share {
        let isin = &share_isin.isin.get_str();
        let share_details = ShareDetails::from_element(isin, table).unwrap_or_default();
        let market_information = MarketInformation::from_element(isin, table).unwrap_or_default();
        let price_data = PriceData::from_element(isin, table).unwrap_or_default();
        let performance_metrics = PerformanceMetrics::from_element(isin, table).unwrap_or_default();

        Share {
            share_id: share_isin.to_owned(),
            share_details,
            market_information,
            price_data,
            performance_metrics,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct ShareFullInfo {
    // Share Isins Table
    pub isin: String,
    pub share_name: Option<String>,

    // Share Details Table
    pub id_strumento: Option<f64>,
    pub codice_alfanumerico: Option<String>,

    // Market Information Table
    pub super_sector: Option<String>,
    pub mercato_segmento: Option<String>,
    pub capitalizzazione_di_mercato: Option<f64>,
    pub lotto_minimo: Option<f64>,

    // Price Data Table
    pub fase_di_mercato: Option<String>,
    pub prezzo_ultimo_contratto: Option<f64>,
    pub var_percentuale: Option<f64>,
    pub var_assoluta: Option<f64>,
    pub pr_medio_progr: Option<f64>,
    pub data_ora_ultimo_contratto: Option<NaiveDateTime>,
    pub quantita_ultimo: Option<f64>,
    pub quantita_totale: Option<f64>,
    pub numero_contratti: Option<i32>,
    pub controvalore: Option<f64>,
    pub max_oggi: Option<f64>,
    pub max_anno: Option<f64>,
    pub max_anno_date: Option<NaiveDate>,
    pub min_oggi: Option<f64>,
    pub min_anno: Option<f64>,
    pub min_anno_date: Option<NaiveDate>,
    pub chiusura_precedente: Option<f64>,
    pub prezzo_riferimento: Option<f64>,
    pub data_ora_prezzo_rifermento: Option<NaiveDateTime>,
    pub prezzo_ufficiale: Option<f64>,
    pub data_prezzo_ufficiale: Option<NaiveDate>,
    pub apertura_odierna: Option<f64>,

    // Performance Metrics Table
    pub performance_1_mese: Option<f64>,
    pub performance_6_mesi: Option<f64>,
    pub performance_1_anno: Option<f64>,
}

impl From<ShareFullInfo> for Share {
    fn from(info: ShareFullInfo) -> Self {
        Share {
            share_id: ShareIsin {
                isin: Isin::new(&info.isin).unwrap_or_default(),
                share_name: info.share_name.unwrap_or_default(),
            },
            share_details: ShareDetails {
                isin: info.isin.clone(),
                id_strumento: info.id_strumento.unwrap_or_default(),
                codice_alfanumerico: info.codice_alfanumerico.unwrap_or_default(),
            },
            market_information: MarketInformation {
                isin: info.isin.clone(),
                super_sector: info.super_sector.unwrap_or_default(),
                mercato_segmento: info.mercato_segmento.unwrap_or_default(),
                capitalizzazione_di_mercato: info.capitalizzazione_di_mercato.unwrap_or_default(),
                lotto_minimo: info.lotto_minimo.unwrap_or_default(),
            },
            price_data: PriceData {
                isin: info.isin.clone(),
                fase_di_mercato: info.fase_di_mercato.unwrap_or_default(),
                prezzo_ultimo_contratto: info.prezzo_ultimo_contratto.unwrap_or_default(),
                var_percentuale: info.var_percentuale.unwrap_or_default(),
                var_assoluta: info.var_assoluta.unwrap_or_default(),
                pr_medio_progr: info.pr_medio_progr.unwrap_or_default(),
                data_ora_ultimo_contratto: info.data_ora_ultimo_contratto.unwrap_or_default(),
                quantita_ultimo: info.quantita_ultimo.unwrap_or_default(),
                quantita_totale: info.quantita_totale.unwrap_or_default(),
                numero_contratti: info.numero_contratti.unwrap_or_default() as u64,
                controvalore: info.controvalore.unwrap_or_default(),
                max_oggi: info.max_oggi.unwrap_or_default(),
                max_anno: PriceDateReference {
                    price: info.max_anno.unwrap_or_default(),
                    date: info.max_anno_date.unwrap_or_default(),
                },
                min_oggi: info.min_oggi.unwrap_or_default(),
                min_anno: PriceDateReference {
                    price: info.min_anno.unwrap_or_default(),
                    date: info.min_anno_date.unwrap_or_default(),
                },
                chiusura_precedente: info.chiusura_precedente.unwrap_or_default(),
                prezzo_riferimento: PriceDateTimeReference {
                    price: info.prezzo_riferimento.unwrap_or_default(),
                    datetime: info.data_ora_prezzo_rifermento.unwrap_or_default(),
                },
                prezzo_ufficiale: PriceDateReference {
                    price: info.prezzo_ufficiale.unwrap_or_default(),
                    date: info.data_prezzo_ufficiale.unwrap_or_default(),
                },
                apertura_odierna: info.apertura_odierna.unwrap_or_default(),
            },
            performance_metrics: PerformanceMetrics {
                isin: info.isin.clone(),
                performance_1_mese: info.performance_1_mese.unwrap_or_default(),
                performance_6_mesi: info.performance_6_mesi.unwrap_or_default(),
                performance_1_anno: info.performance_1_anno.unwrap_or_default(),
            },
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PriceDateReference {
    pub price: f64,
    pub date: NaiveDate,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PriceDateTimeReference {
    pub price: f64,
    pub datetime: NaiveDateTime,
}
