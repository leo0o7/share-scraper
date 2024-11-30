use crate::shares::models::ElementRef;
use crate::shares::parsers::DefaultParse;
use crate::shares::selectors::select_for_prop;
use serde::{Deserialize, Serialize};

use crate::generate_from_element;
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub isin: String,
    pub performance_1_mese: f64,
    pub performance_6_mesi: f64,
    pub performance_1_anno: f64,
}

generate_from_element!(PerformanceMetrics, {
    performance_1_mese: f64,
    performance_6_mesi: f64,
    performance_1_anno: f64,
});
