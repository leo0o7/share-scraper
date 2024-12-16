INSERT INTO share_details (isin, id_strumento, codice_alfanumerico, updated_at)
VALUES ($1, $2, $3, $4)
ON CONFLICT (isin) DO UPDATE SET
id_strumento = COALESCE(EXCLUDED.id_strumento, share_details.id_strumento),
codice_alfanumerico = COALESCE(EXCLUDED.codice_alfanumerico, share_details.codice_alfanumerico),
updated_at = COALESCE(EXCLUDED.updated_at, share_details.updated_at)
