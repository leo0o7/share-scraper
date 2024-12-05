use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::{query, query_as, Pool, Postgres};
use tracing::{error, info, info_span, Instrument};

use scraper::{
    isins::types::{DBShareIsin, ShareIsin},
    shares::models::share::{Share, ShareFullInfo},
};

pub async fn insert_all_isins(isins: Vec<ShareIsin>, pool: &Pool<Postgres>) {
    println!("Total ISINs found: {}", isins.len());

    let mut tasks = FuturesUnordered::new();

    for isin in isins {
        tasks.push(insert_isin(isin, pool));
    }

    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            eprint!("Unable to insert ISIN, {e}");
        }
    }
}

pub async fn insert_isin(isin: ShareIsin, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let _ = query!(
        "INSERT INTO share_isins (isin, share_name) VALUES ($1, $2)",
        isin.isin.get_str(),
        isin.share_name
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn query_all_isins(pool: &Pool<Postgres>) -> Result<Vec<ShareIsin>, sqlx::Error> {
    info!("Querying all isins from db");
    let db_res = query_as!(DBShareIsin, "SELECT * FROM share_isins")
        .fetch_all(pool)
        .await?;

    let share_isins: Vec<ShareIsin> = db_res.into_iter().filter_map(ShareIsin::from_db).collect();
    info!("Got a total of {} from db", share_isins.len());

    Ok(share_isins)
}

pub async fn insert_all_shares(shares: Vec<Share>, pool: &Pool<Postgres>) {
    info!("Inserting a total of {} Shares", shares.len());

    let mut tasks = FuturesUnordered::new();

    for share in shares {
        tasks.push(insert_share(share, pool));
    }

    let mut curr_num = 0;
    let share_num = tasks.len();
    while let Some(res) = tasks.next().instrument(info_span!("inserting_share")).await {
        curr_num += 1;
        info!("Inserting share {}/{}", curr_num, share_num);
        if let Err(e) = res {
            error!("Unable to insert Share, {}", e);
        }
    }
}

pub async fn insert_share(share: Share, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let share_details = share.share_details;
    let market_information = share.market_information;
    let price_data = share.price_data;
    let performance_metrics = share.performance_metrics;

    info!(
        "Inserting ShareDetails for {}",
        share.share_id.isin.get_str()
    );
    query!(
        r#"
        INSERT INTO share_details (isin, id_strumento, codice_alfanumerico)
        VALUES ($1, $2, $3)
        ON CONFLICT (isin) DO UPDATE SET
        id_strumento = COALESCE(EXCLUDED.id_strumento, share_details.id_strumento),
        codice_alfanumerico = COALESCE(EXCLUDED.codice_alfanumerico, share_details.codice_alfanumerico)
        "#,
        share_details.isin,
        share_details.id_strumento,
        share_details.codice_alfanumerico
    )
    .execute(&mut *tx)
    .await?;

    info!(
        "Inserting MarketInformation for {}",
        share.share_id.isin.get_str()
    );
    query!(
        r#"
        INSERT INTO market_information (isin, super_sector, mercato_segmento, capitalizzazione_di_mercato, lotto_minimo)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (isin) DO UPDATE SET
        super_sector = COALESCE(EXCLUDED.super_sector, market_information.super_sector),
        mercato_segmento = COALESCE(EXCLUDED.mercato_segmento, market_information.mercato_segmento),
        capitalizzazione_di_mercato = COALESCE(EXCLUDED.capitalizzazione_di_mercato, market_information.capitalizzazione_di_mercato),
        lotto_minimo = COALESCE(EXCLUDED.lotto_minimo, market_information.lotto_minimo)
        "#,
        market_information.isin,
        market_information.super_sector,
        market_information.mercato_segmento,
        market_information.capitalizzazione_di_mercato,
        market_information.lotto_minimo
    )
    .execute(&mut *tx)
    .await?;

    info!("Inserting PriceData for {}", share.share_id.isin.get_str());
    query!(
        r#"
        INSERT INTO price_data (
            isin, fase_di_mercato, prezzo_ultimo_contratto, var_percentuale,
            var_assoluta, pr_medio_progr, data_ora_ultimo_contratto, quantita_ultimo,
            quantita_totale, numero_contratti, controvalore, max_oggi, max_anno,
            max_anno_date, min_oggi, min_anno, min_anno_date, chiusura_precedente,
            prezzo_riferimento, data_ora_prezzo_rifermento, prezzo_ufficiale,
            data_prezzo_ufficiale, apertura_odierna
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23
        )
        ON CONFLICT (isin) DO UPDATE SET
        fase_di_mercato = COALESCE(EXCLUDED.fase_di_mercato, price_data.fase_di_mercato),
        prezzo_ultimo_contratto = COALESCE(EXCLUDED.prezzo_ultimo_contratto, price_data.prezzo_ultimo_contratto),
        var_percentuale = COALESCE(EXCLUDED.var_percentuale, price_data.var_percentuale),
        var_assoluta = COALESCE(EXCLUDED.var_assoluta, price_data.var_assoluta),
        pr_medio_progr = COALESCE(EXCLUDED.pr_medio_progr, price_data.pr_medio_progr),
        data_ora_ultimo_contratto = COALESCE(EXCLUDED.data_ora_ultimo_contratto, price_data.data_ora_ultimo_contratto),
        quantita_ultimo = COALESCE(EXCLUDED.quantita_ultimo, price_data.quantita_ultimo),
        quantita_totale = COALESCE(EXCLUDED.quantita_totale, price_data.quantita_totale),
        numero_contratti = COALESCE(EXCLUDED.numero_contratti, price_data.numero_contratti),
        controvalore = COALESCE(EXCLUDED.controvalore, price_data.controvalore),
        max_oggi = COALESCE(EXCLUDED.max_oggi, price_data.max_oggi),
        max_anno = COALESCE(EXCLUDED.max_anno, price_data.max_anno),
        max_anno_date = COALESCE(EXCLUDED.max_anno_date, price_data.max_anno_date),
        min_oggi = COALESCE(EXCLUDED.min_oggi, price_data.min_oggi),
        min_anno = COALESCE(EXCLUDED.min_anno, price_data.min_anno),
        min_anno_date = COALESCE(EXCLUDED.min_anno_date, price_data.min_anno_date),
        chiusura_precedente = COALESCE(EXCLUDED.chiusura_precedente, price_data.chiusura_precedente),
        prezzo_riferimento = COALESCE(EXCLUDED.prezzo_riferimento, price_data.prezzo_riferimento),
        data_ora_prezzo_rifermento = COALESCE(EXCLUDED.data_ora_prezzo_rifermento, price_data.data_ora_prezzo_rifermento),
        prezzo_ufficiale = COALESCE(EXCLUDED.prezzo_ufficiale, price_data.prezzo_ufficiale),
        data_prezzo_ufficiale = COALESCE(EXCLUDED.data_prezzo_ufficiale, price_data.data_prezzo_ufficiale),
        apertura_odierna = COALESCE(EXCLUDED.apertura_odierna, price_data.apertura_odierna)
        "#,
        price_data.isin,
        price_data.fase_di_mercato,
        price_data.prezzo_ultimo_contratto,
        price_data.var_percentuale,
        price_data.var_assoluta,
        price_data.pr_medio_progr,
        price_data.data_ora_ultimo_contratto,
        price_data.quantita_ultimo,
        price_data.quantita_totale,
        price_data.numero_contratti.map(|i| i as i32),
        price_data.controvalore,
        price_data.max_oggi,
        price_data.max_anno.as_ref().and_then(|e| e.price),
        price_data.max_anno.as_ref().and_then(|e| e.date),
        price_data.min_oggi,
        price_data.min_anno.as_ref().and_then(|e| e.price),
        price_data.min_anno.as_ref().and_then(|e| e.date),
        price_data.chiusura_precedente,
        price_data.prezzo_riferimento.as_ref().and_then(|e| e.price),
        price_data.prezzo_riferimento.as_ref().and_then(|e| e.datetime),
        price_data.prezzo_ufficiale.as_ref().and_then(|e| e.price),
        price_data.prezzo_ufficiale.as_ref().and_then(|e| e.date),
        price_data.apertura_odierna
    )
    .execute(&mut *tx)
    .await?;

    info!(
        "Inserting PerformanceMetrics for {}",
        share.share_id.isin.get_str()
    );
    query!(
        r#"
        INSERT INTO performance_metrics (isin, performance_1_mese, performance_6_mesi, performance_1_anno)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (isin) DO UPDATE SET
        performance_1_mese = COALESCE(EXCLUDED.performance_1_mese, performance_metrics.performance_1_mese),
        performance_6_mesi = COALESCE(EXCLUDED.performance_6_mesi, performance_metrics.performance_6_mesi),
        performance_1_anno = COALESCE(EXCLUDED.performance_1_anno, performance_metrics.performance_1_anno)
        "#,
        performance_metrics.isin,
        performance_metrics.performance_1_mese,
        performance_metrics.performance_6_mesi,
        performance_metrics.performance_1_anno
    )
    .execute(&mut *tx)
    .await?;

    match tx.commit().await {
        Ok(_) => {
            info!(
                "Successfully commited transition for {}",
                share.share_id.isin.get_str()
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "Error commiting transition for {}",
                share.share_id.isin.get_str()
            );
            Err(e)
        }
    }
}

pub async fn get_share_by_isin(
    isin: &str,
    pool: &Pool<Postgres>,
) -> Result<Option<ShareFullInfo>, sqlx::Error> {
    query_as!(
        ShareFullInfo,
        r#"
         SELECT
             si.isin,
             si.share_name,
             sd.id_strumento,
             sd.codice_alfanumerico,
             mi.super_sector,
             mi.mercato_segmento,
             mi.capitalizzazione_di_mercato,
             mi.lotto_minimo,
             pd.fase_di_mercato,
             pd.prezzo_ultimo_contratto,
             pd.var_percentuale,
             pd.var_assoluta,
             pd.pr_medio_progr,
             pd.data_ora_ultimo_contratto,
             pd.quantita_ultimo,
             pd.quantita_totale,
             pd.numero_contratti,
             pd.controvalore,
             pd.max_oggi,
             pd.max_anno,
             pd.max_anno_date,
             pd.min_oggi,
             pd.min_anno,
             pd.min_anno_date,
             pd.chiusura_precedente,
             pd.prezzo_riferimento,
             pd.data_ora_prezzo_rifermento,
             pd.prezzo_ufficiale,
             pd.data_prezzo_ufficiale,
             pd.apertura_odierna,
             pm.performance_1_mese,
             pm.performance_6_mesi,
             pm.performance_1_anno
         FROM share_isins si
         LEFT JOIN share_details sd ON si.isin = sd.isin
         LEFT JOIN market_information mi ON si.isin = mi.isin
         LEFT JOIN price_data pd ON si.isin = pd.isin
         LEFT JOIN performance_metrics pm ON si.isin = pm.isin
         WHERE si.isin = $1
         "#,
        isin
    )
    .fetch_optional(pool)
    .await
}
