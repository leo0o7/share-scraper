use scraper::shares::Share;
use serde::{de, Deserialize, Deserializer};
use sqlx::{Pool, Postgres, QueryBuilder};
use std::{fmt, str::FromStr};
use tracing::{info, warn};

const INITIAL_SHARE_QUERY: &str = r#"
     SELECT
         si.isin,
         si.share_name,
         sd.id_strumento,
         sd.codice_alfanumerico,
         mi.super_sector,
         mi.mercato_segmento,
         mi.capitalizzazione_di_mercato,
         mi.lotto_minimo,
         pd.fase_di_mercato,
         pd.prezzo_ultimo_contratto,
         pd.var_percentuale,
         pd.var_assoluta,
         pd.pr_medio_progr,
         pd.data_ora_ultimo_contratto,
         pd.quantita_ultimo,
         pd.quantita_totale,
         pd.numero_contratti,
         pd.controvalore,
         pd.max_oggi,
         pd.max_anno,
         pd.max_anno_date,
         pd.min_oggi,
         pd.min_anno,
         pd.min_anno_date,
         pd.chiusura_precedente,
         pd.prezzo_riferimento,
         pd.data_ora_prezzo_rifermento,
         pd.prezzo_ufficiale,
         pd.data_prezzo_ufficiale,
         pd.apertura_odierna,
         pm.performance_1_mese,
         pm.performance_6_mesi,
         pm.performance_1_anno
     FROM share_isins si
     LEFT JOIN share_details sd ON si.isin = sd.isin
     LEFT JOIN market_information mi ON si.isin = mi.isin
     LEFT JOIN price_data pd ON si.isin = pd.isin
     LEFT JOIN performance_metrics pm ON si.isin = pm.isin
"#;

#[derive(Deserialize, Debug)]
pub struct ShareQuery {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub isin: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub lang: Option<String>,
}

impl ShareQuery {
    pub fn new(isin: Option<String>, name: Option<String>, lang: Option<String>) -> Self {
        Self { isin, name, lang }
    }
    pub fn empty() -> Self {
        Self {
            isin: None,
            name: None,
            lang: None,
        }
    }
    pub fn from_isin(isin: String) -> Self {
        Self {
            isin: Some(isin),
            name: None,
            lang: None,
        }
    }
    pub fn from_name(name: String) -> Self {
        Self {
            isin: None,
            name: Some(name),
            lang: None,
        }
    }
    pub fn from_lang(lang: String) -> Self {
        Self {
            isin: None,
            name: None,
            lang: Some(lang),
        }
    }
}

// from axum docs
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

pub async fn query_share_with(
    query: ShareQuery,
    pool: &Pool<Postgres>,
) -> Result<Vec<Share>, sqlx::Error> {
    info!("Querying shares with {:?}", query);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(INITIAL_SHARE_QUERY);

    let query_as = match (query.name, query.isin, query.lang) {
        (_, Some(isin), _) => query_builder
            .push(" WHERE si.isin = ")
            .push_bind(isin)
            .build_query_as(),
        (None, None, Some(lang)) => query_builder
            .push(" WHERE si.isin ILIKE ")
            .push_bind(lang)
            .push(" || '%'")
            .build_query_as(),
        (Some(name), None, Some(lang)) => query_builder
            .push(" WHERE si.isin ILIKE ")
            .push_bind(lang)
            .push(" || '%' AND si.share_name ILIKE '%' || ")
            .push_bind(name)
            .push(" || '%'")
            .build_query_as(),
        (Some(name), None, None) => query_builder
            .push(" WHERE si.share_name ILIKE '%' || ")
            .push_bind(name)
            .push(" || '%'")
            .build_query_as(),
        (None, None, None) => query_builder.build_query_as(),
    };

    let res = query_as.fetch_all(pool).await?;

    if res.is_empty() {
        warn!("Found no share");
    } else {
        info!("Got a total of {} shares", res.len());
    }

    Ok(res)
}
