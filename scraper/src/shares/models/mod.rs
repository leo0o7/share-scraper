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
use tracing::{info, warn};

pub trait ScrapableStruct {
    fn from_element(share_isin: &ShareIsin, table: ElementRef) -> Self;
    fn with_isin(share_isin: &ShareIsin) -> Self;
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Share {
    pub share_id: ShareIsin,
    pub share_details: ShareDetails,
    pub market_information: MarketInformation,
    pub price_data: PriceData,
    pub performance_metrics: PerformanceMetrics,
}

impl ScrapableStruct for Share {
    fn with_isin(share_isin: &ShareIsin) -> Self {
        warn!("Creating empty share");
        Share {
            share_id: share_isin.clone(),
            share_details: ShareDetails::with_isin(share_isin),
            market_information: MarketInformation::with_isin(share_isin),
            price_data: PriceData::with_isin(share_isin),
            performance_metrics: PerformanceMetrics::with_isin(share_isin),
        }
    }
    fn from_element(share_isin: &ShareIsin, table: ElementRef) -> Self {
        info!("Creating full share from elemnt");
        Share {
            share_id: share_isin.clone(),
            share_details: ShareDetails::from_element(share_isin, table),
            market_information: MarketInformation::from_element(share_isin, table),
            price_data: PriceData::from_element(share_isin, table),
            performance_metrics: PerformanceMetrics::from_element(share_isin, table),
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
                id_strumento: info.id_strumento,
                codice_alfanumerico: info.codice_alfanumerico,
            },
            market_information: MarketInformation {
                isin: info.isin.clone(),
                super_sector: info.super_sector,
                mercato_segmento: info.mercato_segmento,
                capitalizzazione_di_mercato: info.capitalizzazione_di_mercato,
                lotto_minimo: info.lotto_minimo,
            },
            price_data: PriceData {
                isin: info.isin.clone(),
                fase_di_mercato: info.fase_di_mercato,
                prezzo_ultimo_contratto: info.prezzo_ultimo_contratto,
                var_percentuale: info.var_percentuale,
                var_assoluta: info.var_assoluta,
                pr_medio_progr: info.pr_medio_progr,
                data_ora_ultimo_contratto: info.data_ora_ultimo_contratto,
                quantita_ultimo: info.quantita_ultimo,
                quantita_totale: info.quantita_totale,
                numero_contratti: info.numero_contratti.map(|v| v as u64),
                controvalore: info.controvalore,
                max_oggi: info.max_oggi,
                max_anno: Some(PriceDateReference {
                    price: info.max_anno,
                    date: info.max_anno_date,
                }),
                min_oggi: info.min_oggi,
                min_anno: Some(PriceDateReference {
                    price: info.min_anno,
                    date: info.min_anno_date,
                }),
                chiusura_precedente: info.chiusura_precedente,
                prezzo_riferimento: Some(PriceDateTimeReference {
                    price: info.prezzo_riferimento,
                    datetime: info.data_ora_prezzo_rifermento,
                }),
                prezzo_ufficiale: Some(PriceDateReference {
                    price: info.prezzo_ufficiale,
                    date: info.data_prezzo_ufficiale,
                }),
                apertura_odierna: info.apertura_odierna,
            },
            performance_metrics: PerformanceMetrics {
                isin: info.isin.clone(),
                performance_1_mese: info.performance_1_mese,
                performance_6_mesi: info.performance_6_mesi,
                performance_1_anno: info.performance_1_anno,
            },
        }
    }
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
