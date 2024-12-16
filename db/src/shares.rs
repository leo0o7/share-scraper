use chrono::TimeDelta;
use futures::{stream::FuturesUnordered, StreamExt};
use scraper::{isins::types::ShareIsin, shares::Share};
use serde::Deserialize;
use sqlx::query_file;
use sqlx::{postgres::types::PgInterval, query_as, Pool, Postgres, QueryBuilder};
use tracing::{error, info, info_span, warn, Instrument};

use crate::metrics::InsertionMetrics;
use crate::utils::empty_string_as_none;

// IMPORTANT:
// share queries are found at:
// db/queries/share/*.sql
// excluding the following constants:

// can't be a file because it's used in QueryBuilder
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
        pm.performance_1_anno,
        GREATEST(
            si.updated_at,
            sd.updated_at,
            mi.updated_at,
            pd.updated_at,
            pm.updated_at
        ) AS updated_at
    FROM share_isins si
    LEFT JOIN share_details sd ON si.isin = sd.isin
    LEFT JOIN market_information mi ON si.isin = mi.isin
    LEFT JOIN price_data pd ON si.isin = pd.isin
    LEFT JOIN performance_metrics pm ON si.isin = pm.isin
"#;
// can't be a file because the query_as macro doesn't work properly with my types
// and query_file_as uses the macro and not the function
const SHARE_ISINS_WITH_INTERVAL: &str = r#"
    WITH latest_updates AS (
        SELECT 
            si.isin,
            GREATEST(
                COALESCE(sd.updated_at, '1970-01-01'::TIMESTAMP),
                COALESCE(mi.updated_at, '1970-01-01'::TIMESTAMP),
                COALESCE(pd.updated_at, '1970-01-01'::TIMESTAMP),
                COALESCE(pm.updated_at, '1970-01-01'::TIMESTAMP)
            ) AS last_update
        FROM share_isins si
        LEFT JOIN share_details sd ON si.isin = sd.isin
        LEFT JOIN market_information mi ON si.isin = mi.isin
        LEFT JOIN price_data pd ON si.isin = pd.isin
        LEFT JOIN performance_metrics pm ON si.isin = pm.isin
    )
    SELECT 
        si.isin,
        si.share_name,
        lu.last_update as updated_at
    FROM share_isins si
    JOIN latest_updates lu ON si.isin = lu.isin
    WHERE lu.last_update <= NOW() - $1::INTERVAL
"#;

#[derive(Deserialize, Debug, Default)]
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
        Self::default()
    }
    pub fn builder() -> ShareQueryBuilder {
        ShareQueryBuilder::default()
    }
}

#[derive(Default)]
pub struct ShareQueryBuilder {
    pub name: Option<String>,
    pub isin: Option<String>,
    pub lang: Option<String>,
}
impl ShareQueryBuilder {
    pub fn name(mut self, name: String) -> ShareQueryBuilder {
        self.name = Some(name);
        self
    }
    pub fn isin(mut self, isin: String) -> ShareQueryBuilder {
        self.isin = Some(isin);
        self
    }
    pub fn lang(mut self, lang: String) -> ShareQueryBuilder {
        self.lang = Some(lang);
        self
    }

    pub fn build(self) -> ShareQuery {
        ShareQuery {
            name: self.name,
            isin: self.isin,
            lang: self.lang,
        }
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

pub async fn get_shares_to_refresh(
    pool: &Pool<Postgres>,
    min_duration: TimeDelta,
) -> Result<Vec<ShareIsin>, sqlx::Error> {
    query_as(SHARE_ISINS_WITH_INTERVAL)
        .bind(PgInterval {
            months: 0,
            days: 0,
            microseconds: min_duration.num_microseconds().unwrap_or_default(),
        })
        .fetch_all(pool)
        .await
}

pub async fn insert_all_shares(shares: Vec<Share>, pool: &Pool<Postgres>) -> InsertionMetrics {
    let share_num = shares.len() as i32;
    let mut tasks = FuturesUnordered::new();

    info!("Inserting a total of {} Shares", share_num);

    for share in shares {
        tasks.push(insert_share(share, pool).instrument(info_span!("inserting_share")));
    }

    let mut curr_idx = 0;
    let mut successful_inserts = 0;

    while let Some(res) = tasks.next().await {
        curr_idx += 1;
        info!("Inserting share {}/{}", curr_idx, share_num);

        if let Err(e) = res {
            error!("Unable to insert Share, {}", e);
        } else {
            successful_inserts += 1;
        }
    }

    InsertionMetrics {
        total: share_num,
        successful: successful_inserts,
    }
}

pub async fn insert_share(share: Share, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let isin = share.share_id.isin;

    let share_details = share.share_details;
    let market_information = share.market_information;
    let price_data = share.price_data;
    let performance_metrics = share.performance_metrics;

    info!("Inserting ShareDetails for {}", isin);
    query_file!(
        "./queries/share/insert_details.sql",
        share_details.isin,
        share_details.id_strumento,
        share_details.codice_alfanumerico,
        share_details.updated_at
    )
    .execute(&mut *tx)
    .await?;

    info!("Inserting MarketInformation for {}", isin);
    query_file!(
        "./queries/share/insert_market_info.sql",
        market_information.isin,
        market_information.super_sector,
        market_information.mercato_segmento,
        market_information.capitalizzazione_di_mercato,
        market_information.lotto_minimo,
        market_information.updated_at
    )
    .execute(&mut *tx)
    .await?;

    info!("Inserting PriceData for {}", isin);
    query_file!(
        "./queries/share/insert_price_data.sql",
        price_data.isin,
        price_data.fase_di_mercato,
        price_data.prezzo_ultimo_contratto,
        price_data.var_percentuale,
        price_data.var_assoluta,
        price_data.pr_medio_progr,
        price_data.data_ora_ultimo_contratto,
        price_data.quantita_ultimo,
        price_data.quantita_totale,
        price_data.numero_contratti.map(|i| i as i32),
        price_data.controvalore,
        price_data.max_oggi,
        price_data.max_anno.as_ref().and_then(|e| e.price),
        price_data.max_anno.as_ref().and_then(|e| e.date),
        price_data.min_oggi,
        price_data.min_anno.as_ref().and_then(|e| e.price),
        price_data.min_anno.as_ref().and_then(|e| e.date),
        price_data.chiusura_precedente,
        price_data.prezzo_riferimento.as_ref().and_then(|e| e.price),
        price_data
            .prezzo_riferimento
            .as_ref()
            .and_then(|e| e.datetime),
        price_data.prezzo_ufficiale.as_ref().and_then(|e| e.price),
        price_data.prezzo_ufficiale.as_ref().and_then(|e| e.date),
        price_data.apertura_odierna,
        price_data.updated_at
    )
    .execute(&mut *tx)
    .await?;

    info!("Inserting PerformanceMetrics for {}", isin);
    query_file!(
        "./queries/share/insert_performance_metrics.sql",
        performance_metrics.isin,
        performance_metrics.performance_1_mese,
        performance_metrics.performance_6_mesi,
        performance_metrics.performance_1_anno,
        performance_metrics.updated_at
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await.inspect_err(|e| {
        error!("Error commiting transition for {}: {}", isin, e);
    })?;

    info!("Successfully commited transition for {}", isin);
    Ok(())
}
