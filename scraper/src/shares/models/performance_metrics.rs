use super::gen_macro::*;
use crate::generate_scrapable_struct;
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub isin: String,
    pub performance_1_mese: Option<f64>,
    pub performance_6_mesi: Option<f64>,
    pub performance_1_anno: Option<f64>,
}

generate_scrapable_struct!(PerformanceMetrics, {
    performance_1_mese: f64,
    performance_6_mesi: f64,
    performance_1_anno: f64,
});
