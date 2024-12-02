use crate::shares::models::ElementRef;
use crate::shares::models::ScrapableStruct;
use crate::shares::parsers::SafeParse;
use crate::shares::selectors::select_for_prop;
use crate::shares::ShareIsin;
use serde::{Deserialize, Serialize};

use crate::generate_scrapable_struct;

#[derive(Default, Debug, Serialize, Deserialize)]
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
