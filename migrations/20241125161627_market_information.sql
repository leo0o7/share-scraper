CREATE TABLE IF NOT EXISTS market_information (
  isin VARCHAR(12) PRIMARY KEY,
  super_sector VARCHAR(50),
  mercato_segmento VARCHAR(50),
  capitalizzazione_di_mercato DOUBLE PRECISION,
  lotto_minimo DOUBLE PRECISION,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);

