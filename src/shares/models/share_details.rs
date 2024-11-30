use crate::shares::models::ElementRef;
use crate::shares::parsers::DefaultParse;
use crate::shares::selectors::select_for_prop;
use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::generate_from_element;

#[derive(Derivative, Debug, Serialize, Deserialize)]
#[derivative(Default)]
pub struct ShareDetails {
    pub isin: String,
    pub id_strumento: f64,
    #[derivative(Default(value = "\"N/A\".to_string()"))]
    pub codice_alfanumerico: String,
}

generate_from_element!(ShareDetails, {
    id_strumento: f64,
    codice_alfanumerico: String,
});
