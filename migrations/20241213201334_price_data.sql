DROP TABLE price_data;

CREATE TABLE price_data (
  isin VARCHAR(12) PRIMARY KEY,
  fase_di_mercato VARCHAR(50) NULL,
  prezzo_ultimo_contratto DOUBLE PRECISION NULL,
  var_percentuale DOUBLE PRECISION NULL,
  var_assoluta DOUBLE PRECISION NULL,
  pr_medio_progr DOUBLE PRECISION NULL,
  data_ora_ultimo_contratto TIMESTAMP NULL,
  quantita_ultimo DOUBLE PRECISION NULL,
  quantita_totale DOUBLE PRECISION NULL,
  numero_contratti INT NULL,
  controvalore DOUBLE PRECISION NULL,
  max_oggi DOUBLE PRECISION NULL,
  max_anno DOUBLE PRECISION NULL,
  max_anno_date DATE NULL,
  min_oggi DOUBLE PRECISION NULL,
  min_anno DOUBLE PRECISION NULL,
  min_anno_date DATE NULL,
  chiusura_precedente DOUBLE PRECISION NULL,
  prezzo_riferimento DOUBLE PRECISION NULL,
  data_ora_prezzo_rifermento TIMESTAMP NULL,
  prezzo_ufficiale DOUBLE PRECISION NULL,
  data_prezzo_ufficiale DATE NULL,
  apertura_odierna DOUBLE PRECISION NULL,
  updated_at TIMESTAMP NOT NULL,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);
