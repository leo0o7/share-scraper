INSERT INTO market_information (isin, super_sector, mercato_segmento, capitalizzazione_di_mercato, lotto_minimo, updated_at)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (isin) DO UPDATE SET
super_sector = COALESCE(EXCLUDED.super_sector, market_information.super_sector),
mercato_segmento = COALESCE(EXCLUDED.mercato_segmento, market_information.mercato_segmento),
capitalizzazione_di_mercato = COALESCE(EXCLUDED.capitalizzazione_di_mercato, market_information.capitalizzazione_di_mercato),
lotto_minimo = COALESCE(EXCLUDED.lotto_minimo, market_information.lotto_minimo),
updated_at = COALESCE(EXCLUDED.updated_at, market_information.updated_at)
