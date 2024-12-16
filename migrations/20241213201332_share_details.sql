DROP TABLE share_details;

CREATE TABLE share_details (
  isin VARCHAR(12) PRIMARY KEY,
  id_strumento DOUBLE PRECISION NULL,
  codice_alfanumerico VARCHAR(50) NULL,
  updated_at TIMESTAMP NOT NULL,
  FOREIGN KEY (isin) REFERENCES share_isins(isin)
);
