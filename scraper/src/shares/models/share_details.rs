use super::gen_macro::*;
use crate::generate_scrapable_struct;
use serde::{Deserialize, Serialize};

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
