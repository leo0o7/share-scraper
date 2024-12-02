CREATE TABLE IF NOT EXISTS share_details (
  isin VARCHAR(12) PRIMARY KEY,
  id_strumento DOUBLE PRECISION NULL,
  codice_alfanumerico VARCHAR(50) NULL,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);

