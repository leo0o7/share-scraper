use crate::isins::types::{Isin, ShareIsin};

use super::{
    parsers::{
        parse_date, parse_datetime, parse_datetime_or_default, parse_float, parse_float_or_default,
        parse_int_or_default, parse_str_or_default,
    },
    selectors::select_for_prop,
};
use chrono::{NaiveDate, NaiveDateTime};
use derivative::Derivative;
use scraper::ElementRef;
use serde::{Deserialize, Serialize};
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

#[derive(Derivative, Debug, Serialize, Deserialize)]
#[derivative(Default)]
pub struct ShareDetails {
    pub isin: String,
    pub id_strumento: f64,
    #[derivative(Default(value = "\"N/A\".to_string()"))]
    pub codice_alfanumerico: String,
}

impl ShareDetails {
    fn from_element(isin: &str, table: ElementRef) -> Option<ShareDetails> {
        let id_strumento = select_for_prop("id_strumento", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let codice_alfanumerico = select_for_prop("codice_alfanumerico", table)
            .map(parse_str_or_default)
            .unwrap_or_default();

        Some(ShareDetails {
            isin: isin.to_owned(),
            id_strumento,
            codice_alfanumerico,
        })
    }
}

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

impl MarketInformation {
    fn from_element(isin: &str, table: ElementRef) -> Option<MarketInformation> {
        let super_sector = select_for_prop("super_sector", table)
            .map(parse_str_or_default)
            .unwrap_or_default();

        let mercato_segmento = select_for_prop("mercato_segmento", table)
            .map(parse_str_or_default)
            .unwrap_or_default();

        let capitalizzazione_di_mercato = select_for_prop("capitalizzazione_di_mercato", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let lotto_minimo = select_for_prop("lotto_minimo", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        Some(MarketInformation {
            isin: isin.to_owned(),
            super_sector,
            mercato_segmento,
            capitalizzazione_di_mercato,
            lotto_minimo,
        })
    }
}

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

impl PriceData {
    fn from_element(isin: &str, table: ElementRef) -> Option<PriceData> {
        let fase_di_mercato = select_for_prop("fase_di_mercato", table)
            .map(parse_str_or_default)
            .unwrap_or_default();

        let prezzo_ultimo_contratto = select_for_prop("prezzo_ultimo_contratto", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let var_percentuale = select_for_prop("var_percentuale", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let var_assoluta = select_for_prop("var_assoluta", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let pr_medio_progr = select_for_prop("pr_medio_progr", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let data_ora_ultimo_contratto = select_for_prop("data_ora_ultimo_contratto", table)
            .map(parse_datetime_or_default)
            .unwrap_or_default();

        let quantita_ultimo = select_for_prop("quantita_ultimo", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let quantita_totale = select_for_prop("quantita_totale", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let numero_contratti = select_for_prop("numero_contratti", table)
            .map(parse_int_or_default)
            .unwrap_or_default() as u64;

        let controvalore = select_for_prop("controvalore", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let max_oggi = select_for_prop("max_oggi", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let max_anno = select_for_prop("max_anno", table)
            .and_then(PriceDateReference::from_element)
            .unwrap_or_default();

        let min_oggi = select_for_prop("min_oggi", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let min_anno = select_for_prop("min_anno", table)
            .and_then(PriceDateReference::from_element)
            .unwrap_or_default();

        let chiusura_precedente = select_for_prop("chiusura_precedente", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let prezzo_riferimento = select_for_prop("prezzo_riferimento", table)
            .and_then(PriceDateTimeReference::from_element)
            .unwrap_or_default();

        let prezzo_ufficiale = select_for_prop("prezzo_ufficiale", table)
            .and_then(PriceDateReference::from_element)
            .unwrap_or_default();

        let apertura_odierna = select_for_prop("apertura_odierna", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        Some(PriceData {
            isin: isin.to_owned(),
            fase_di_mercato,
            prezzo_ultimo_contratto,
            var_percentuale,
            var_assoluta,
            pr_medio_progr,
            data_ora_ultimo_contratto,
            quantita_ultimo,
            quantita_totale,
            numero_contratti,
            controvalore,
            max_oggi,
            max_anno,
            min_oggi,
            min_anno,
            chiusura_precedente,
            prezzo_riferimento,
            prezzo_ufficiale,
            apertura_odierna,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub isin: String,
    pub performance_1_mese: f64,
    pub performance_6_mesi: f64,
    pub performance_1_anno: f64,
}

impl PerformanceMetrics {
    fn from_element(isin: &str, table: ElementRef) -> Option<PerformanceMetrics> {
        let performance_1_mese = select_for_prop("performance_1_mese", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let performance_6_mesi = select_for_prop("performance_6_mesi", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        let performance_1_anno = select_for_prop("performance_1_anno", table)
            .map(parse_float_or_default)
            .unwrap_or_default();

        Some(PerformanceMetrics {
            isin: isin.to_owned(),
            performance_1_mese,
            performance_6_mesi,
            performance_1_anno,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PriceDateReference {
    pub price: f64,
    pub date: NaiveDate,
}

impl PriceDateReference {
    fn from_element(element: ElementRef) -> Option<PriceDateReference> {
        let price_date_str = parse_str_or_default(element);

        price_date_str.split_once(" - ").and_then(|tuple| {
            let price = parse_float(tuple.0).unwrap_or(0.0);
            let date = parse_date(tuple.1).unwrap_or(NaiveDate::default());

            Some(PriceDateReference { price, date })
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PriceDateTimeReference {
    pub price: f64,
    pub datetime: NaiveDateTime,
}

impl PriceDateTimeReference {
    fn from_element(element: ElementRef) -> Option<PriceDateTimeReference> {
        let price_datetime_str = parse_str_or_default(element);

        price_datetime_str.split_once("-").and_then(|tuple| {
            let price = parse_float(tuple.0.trim()).unwrap_or(0.0);
            let datetime = parse_datetime(tuple.1.trim()).unwrap_or(NaiveDateTime::default());

            Some(PriceDateTimeReference { price, datetime })
        })
    }
}
