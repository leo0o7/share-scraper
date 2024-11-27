CREATE TABLE IF NOT EXISTS performance_metrics (
  isin VARCHAR(12) PRIMARY KEY,
  performance_1_mese DOUBLE PRECISION,
  performance_6_mesi DOUBLE PRECISION,
  performance_1_anno DOUBLE PRECISION,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);
