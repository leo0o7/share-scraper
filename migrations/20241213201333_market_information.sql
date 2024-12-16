DROP TABLE market_information;

CREATE TABLE market_information (
  isin VARCHAR(12) PRIMARY KEY,
  super_sector VARCHAR(50) NULL,
  mercato_segmento VARCHAR(50) NULL,
  capitalizzazione_di_mercato DOUBLE PRECISION NULL,
  lotto_minimo DOUBLE PRECISION NULL,
  updated_at TIMESTAMP NOT NULL,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);
