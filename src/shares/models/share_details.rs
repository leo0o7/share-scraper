use crate::shares::models::ElementRef;
use crate::shares::models::ScrapableStruct;
use crate::shares::parsers::SafeParse;
use crate::shares::selectors::select_for_prop;
use crate::shares::ShareIsin;
use serde::{Deserialize, Serialize};

use crate::generate_scrapable_struct;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ShareDetails {
    pub isin: String,
    pub id_strumento: Option<f64>,
    pub codice_alfanumerico: Option<String>,
}

generate_scrapable_struct!(ShareDetails, {
    id_strumento: f64,
    codice_alfanumerico: String,
});
