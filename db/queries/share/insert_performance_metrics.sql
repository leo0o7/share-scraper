INSERT INTO performance_metrics (isin, performance_1_mese, performance_6_mesi, performance_1_anno, updated_at)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (isin) DO UPDATE SET
performance_1_mese = COALESCE(EXCLUDED.performance_1_mese, performance_metrics.performance_1_mese),
performance_6_mesi = COALESCE(EXCLUDED.performance_6_mesi, performance_metrics.performance_6_mesi),
performance_1_anno = COALESCE(EXCLUDED.performance_1_anno, performance_metrics.performance_1_anno),
updated_at = COALESCE(EXCLUDED.updated_at, performance_metrics.updated_at)
