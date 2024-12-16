DROP TABLE performance_metrics;

CREATE TABLE performance_metrics (
  isin VARCHAR(12) PRIMARY KEY,
  performance_1_mese DOUBLE PRECISION NULL,
  performance_6_mesi DOUBLE PRECISION NULL,
  performance_1_anno DOUBLE PRECISION NULL,
  updated_at TIMESTAMP NOT NULL,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);
