use once_cell::sync::Lazy;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use tracing::warn;

static TABLE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("table").unwrap());
static ROW_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("tr").unwrap());
static STRONG_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("strong").unwrap());
static VALUE_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("span.t-text.-right").unwrap());

static MAPPINGS: Lazy<Vec<(&'static str, Vec<&'static str>)>> = Lazy::new(|| {
    vec![
        ("id_strumento", vec!["id strumento"]),
        ("codice_alfanumerico", vec!["codice alfanumerico"]),
        ("super_sector", vec!["super sector"]),
        ("mercato_segmento", vec!["mercato/segmento"]),
        (
            "capitalizzazione_di_mercato",
            vec!["capitalizzazione di mercato"],
        ),
        ("lotto_minimo", vec!["lotto minimo"]),
        ("fase_di_mercato", vec!["fase di mercato"]),
        ("prezzo_ultimo_contratto", vec!["prezzo ultimo contratto"]),
        ("var_percentuale", vec!["var %"]),
        ("var_assoluta", vec!["var assoluta"]),
        ("pr_medio_progr", vec!["pr medio progr"]),
        (
            "data_ora_ultimo_contratto",
            vec!["data - ora ultimo contratto:"],
        ),
        ("quantita_ultimo", vec!["quantità ultimo"]),
        ("quantita_totale", vec!["quantità totale"]),
        ("numero_contratti", vec!["numero contratti"]),
        ("controvalore", vec!["controvalore"]),
        ("max_oggi", vec!["max oggi"]),
        ("max_anno", vec!["max anno"]),
        ("min_oggi", vec!["min oggi"]),
        ("min_anno", vec!["min anno"]),
        (
            "chiusura_precedente",
            vec!["chiusura precedente/pre-chiusura/chiusura:"],
        ),
        ("prezzo_riferimento", vec!["prezzo di riferimento"]),
        ("prezzo_ufficiale", vec!["prezzo ufficiale"]),
        ("apertura_odierna", vec!["apertura odierna:"]),
        ("performance_1_mese", vec!["performance 1 mese"]),
        ("performance_6_mesi", vec!["performance 6 mesi"]),
        ("performance_1_anno", vec!["performance 1 anno"]),
    ]
});

pub struct PropertySelector<'a> {
    index: HashMap<String, ElementRef<'a>>,
    prop_mapping: HashMap<&'static str, String>,
}

impl<'a> PropertySelector<'a> {
    pub fn new(document: &'a Html) -> Self {
        let mut index = HashMap::new();
        let mut prop_mapping = HashMap::new();

        for (rust_prop, search_terms) in MAPPINGS.iter() {
            for table in document.select(&TABLE_SELECTOR) {
                for row in table.select(&ROW_SELECTOR) {
                    if let Some(strong_elem) = row.select(&STRONG_SELECTOR).next() {
                        let text = strong_elem.text().collect::<String>().to_lowercase();

                        if search_terms.iter().any(|term| text.contains(term)) {
                            index.insert(text.clone(), row);
                            prop_mapping.insert(*rust_prop, text.clone());
                        }
                    }
                }
            }
        }

        Self {
            index,
            prop_mapping,
        }
    }

    pub fn get_property(&self, prop: &str) -> Option<ElementRef<'a>> {
        let indexed_text = self.prop_mapping.get(prop)?;

        let row = match self.index.get(indexed_text) {
            Some(row) => row,
            None => {
                warn!("No element found for {}", prop);
                return None;
            }
        };

        match row.select(&VALUE_SELECTOR).next() {
            Some(el) => Some(el),
            None => {
                warn!("No element found for {}", prop);
                None
            }
        }
    }
}
