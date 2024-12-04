use scraper::ElementRef;

pub fn select_for_prop<'a>(prop: &str, wrapper: ElementRef<'a>) -> Option<ElementRef<'a>> {
    let row = match prop {
        "codice_isin" => "left_1",
        "id_strumento" => "left_2",
        "codice_alfanumerico" => "left_3",
        "super_sector" => "left_4",
        "mercato_segmento" => "left_5",
        "capitalizzazione_di_mercato" => "left_6",
        "lotto_minimo" => "left_7",
        "fase_di_mercato" => "left_8",
        "prezzo_ultimo_contratto" => "left_9",
        "var_percentuale" => "left_10",
        "var_assoluta" => "left_11",
        "pr_medio_progr" => "left_12",
        "data_ora_ultimo_contratto" => "left_13",
        "quantita_ultimo" => "left_14",
        "quantita_totale" => "left_15",
        "numero_contratti" => "left_16",
        "controvalore" => "right_1",
        "max_oggi" => "right_2",
        "max_anno" => "right_3",
        "min_oggi" => "right_4",
        "min_anno" => "right_5",
        "chiusura_precedente" => "right_6",
        "prezzo_riferimento" => "right_7",
        "prezzo_ufficiale" => "right_8",
        "apertura_odierna" => "right_9",
        "performance_1_mese" => "right_10",
        "performance_6_mesi" => "right_11",
        "performance_1_anno" => "right_12",
        _ => {
            return None;
        }
    };

    let tables_selector = scraper::Selector::parse("table:nth-of-type(1)").unwrap();

    let mut tables = wrapper.select(&tables_selector);

    let (table_idx, row_idx) = parse_row_str(row)?;

    if let Some(table) = tables.nth(table_idx) {
        let value_selector = scraper::Selector::parse("span.t-text.-right").unwrap();
        table.select(&value_selector).nth(row_idx)
    } else {
        None
    }
}

fn parse_row_str(raw: &str) -> Option<(usize, usize)> {
    let (side, row_str) = raw.split_once('_')?;
    let row = row_str.parse::<usize>().ok()?;

    match side {
        "left" => Some((0, row - 1)),
        "right" => Some((1, row - 1)),
        _ => None,
    }
}
