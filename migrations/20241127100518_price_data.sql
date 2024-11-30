CREATE TABLE IF NOT EXISTS price_data (
  isin VARCHAR(12) PRIMARY KEY,
  fase_di_mercato VARCHAR(50),
  prezzo_ultimo_contratto DOUBLE PRECISION,
  var_percentuale DOUBLE PRECISION,
  var_assoluta DOUBLE PRECISION,
  pr_medio_progr DOUBLE PRECISION,
  data_ora_ultimo_contratto DATE,
  quantita_ultimo DOUBLE PRECISION,
  quantita_totale DOUBLE PRECISION,
  numero_contratti INT,
  controvalore DOUBLE PRECISION,
  max_oggi DOUBLE PRECISION,
  max_anno DOUBLE PRECISION,
  max_anno_date DATE,
  min_oggi DOUBLE PRECISION,
  min_anno DOUBLE PRECISION,
  min_anno_date DATE,
  chiusura_precedente DOUBLE PRECISION,
  prezzo_riferimento DOUBLE PRECISION,
  data_ora_prezzo_rifermento DATE,
  prezzo_ufficiale DOUBLE PRECISION,
  data_prezzo_ufficiale DATE,
  apertura_odierna DOUBLE PRECISION,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);

