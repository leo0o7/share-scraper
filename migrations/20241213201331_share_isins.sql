DROP TABLE share_isins CASCADE;

CREATE TABLE share_isins (
  isin VARCHAR(12) PRIMARY KEY NOT NULL,
  share_name VARCHAR(50) NOT NULL,
  updated_at TIMESTAMP NOT NULL
);
