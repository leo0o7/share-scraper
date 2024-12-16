use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Row};
use tracing::info;

use super::{
    gen_macro::*, market_information::MarketInformation, performance_metrics::PerformanceMetrics,
    price_data::PriceData, share_details::ShareDetails,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Share {
    pub share_id: ShareIsin,
    pub share_details: ShareDetails,
    pub market_information: MarketInformation,
    pub price_data: PriceData,
    pub performance_metrics: PerformanceMetrics,
    pub updated_at: NaiveDateTime,
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
            updated_at: chrono::offset::Utc::now().naive_utc(),
        }
    }

    fn from_selector(share_isin: &ShareIsin, selector: &PropertySelector) -> Self {
        info!("Creating full share from selector");
        Share {
            share_id: share_isin.clone(),
            share_details: ShareDetails::from_selector(share_isin, selector),
            market_information: MarketInformation::from_selector(share_isin, selector),
            price_data: PriceData::from_selector(share_isin, selector),
            performance_metrics: PerformanceMetrics::from_selector(share_isin, selector),
            updated_at: chrono::offset::Utc::now().naive_utc(),
        }
    }
}
impl FromRow<'_, sqlx::postgres::PgRow> for Share {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let share_id = ShareIsin::from_row(row)?;
        let share_details = ShareDetails::from_row(row)?;
        let market_information = MarketInformation::from_row(row)?;
        let price_data = PriceData::from_row(row)?;
        let performance_metrics = PerformanceMetrics::from_row(row)?;
        let updated_at = row.try_get("updated_at")?;

        Ok(Share {
            share_id,
            share_details,
            market_information,
            price_data,
            performance_metrics,
            updated_at,
        })
    }
}
