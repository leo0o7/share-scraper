INSERT INTO price_data (
    isin, fase_di_mercato, prezzo_ultimo_contratto, var_percentuale,
    var_assoluta, pr_medio_progr, data_ora_ultimo_contratto, quantita_ultimo,
    quantita_totale, numero_contratti, controvalore, max_oggi, max_anno,
    max_anno_date, min_oggi, min_anno, min_anno_date, chiusura_precedente,
    prezzo_riferimento, data_ora_prezzo_rifermento, prezzo_ufficiale,
    data_prezzo_ufficiale, apertura_odierna, updated_at
) VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
    $11, $12, $13, $14, $15, $16, $17, $18, $19,
    $20, $21, $22, $23, $24
)
ON CONFLICT (isin) DO UPDATE SET
fase_di_mercato = COALESCE(EXCLUDED.fase_di_mercato, price_data.fase_di_mercato),
prezzo_ultimo_contratto = COALESCE(EXCLUDED.prezzo_ultimo_contratto, price_data.prezzo_ultimo_contratto),
var_percentuale = COALESCE(EXCLUDED.var_percentuale, price_data.var_percentuale),
var_assoluta = COALESCE(EXCLUDED.var_assoluta, price_data.var_assoluta),
pr_medio_progr = COALESCE(EXCLUDED.pr_medio_progr, price_data.pr_medio_progr),
data_ora_ultimo_contratto = COALESCE(EXCLUDED.data_ora_ultimo_contratto, price_data.data_ora_ultimo_contratto),
quantita_ultimo = COALESCE(EXCLUDED.quantita_ultimo, price_data.quantita_ultimo),
quantita_totale = COALESCE(EXCLUDED.quantita_totale, price_data.quantita_totale),
numero_contratti = COALESCE(EXCLUDED.numero_contratti, price_data.numero_contratti),
controvalore = COALESCE(EXCLUDED.controvalore, price_data.controvalore),
max_oggi = COALESCE(EXCLUDED.max_oggi, price_data.max_oggi),
max_anno = COALESCE(EXCLUDED.max_anno, price_data.max_anno),
max_anno_date = COALESCE(EXCLUDED.max_anno_date, price_data.max_anno_date),
min_oggi = COALESCE(EXCLUDED.min_oggi, price_data.min_oggi),
min_anno = COALESCE(EXCLUDED.min_anno, price_data.min_anno),
min_anno_date = COALESCE(EXCLUDED.min_anno_date, price_data.min_anno_date),
chiusura_precedente = COALESCE(EXCLUDED.chiusura_precedente, price_data.chiusura_precedente),
prezzo_riferimento = COALESCE(EXCLUDED.prezzo_riferimento, price_data.prezzo_riferimento),
data_ora_prezzo_rifermento = COALESCE(EXCLUDED.data_ora_prezzo_rifermento, price_data.data_ora_prezzo_rifermento),
prezzo_ufficiale = COALESCE(EXCLUDED.prezzo_ufficiale, price_data.prezzo_ufficiale),
data_prezzo_ufficiale = COALESCE(EXCLUDED.data_prezzo_ufficiale, price_data.data_prezzo_ufficiale),
apertura_odierna = COALESCE(EXCLUDED.apertura_odierna, price_data.apertura_odierna),
updated_at = COALESCE(EXCLUDED.updated_at, price_data.updated_at)
