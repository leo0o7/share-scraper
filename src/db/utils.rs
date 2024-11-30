use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::{query, query_as, Pool, Postgres};

use crate::{
    isins::types::{DBShareIsin, ShareIsin},
    shares::types::{Share, ShareFullInfo},
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
    let db_res = query_as!(DBShareIsin, "SELECT * FROM share_isins")
        .fetch_all(pool)
        .await?;

    let share_isins: Vec<ShareIsin> = db_res.into_iter().filter_map(ShareIsin::from_db).collect();

    Ok(share_isins)
}

pub async fn insert_all_shares(shares: Vec<Share>, pool: &Pool<Postgres>) {
    println!("Total Shares found: {}", shares.len());

    let mut tasks = FuturesUnordered::new();

    for share in shares {
        tasks.push(insert_share(share, pool));
    }

    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            eprintln!("Unable to insert Share, {e}");
        }
    }
}

pub async fn insert_share(share: Share, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let share_details = share.share_details;
    let market_information = share.market_information;
    let price_data = share.price_data;
    let performance_metrics = share.performance_metrics;

    query!(
        r#"
        INSERT INTO share_details (isin, id_strumento, codice_alfanumerico)
        VALUES ($1, $2, $3)
        ON CONFLICT (isin) DO UPDATE SET 
        id_strumento = EXCLUDED.id_strumento,
        codice_alfanumerico = EXCLUDED.codice_alfanumerico
        "#,
        share_details.isin,
        share_details.id_strumento,
        share_details.codice_alfanumerico
    )
    .execute(&mut *tx)
    .await?;

    query!(
        r#"
        INSERT INTO market_information (isin, super_sector, mercato_segmento, capitalizzazione_di_mercato, lotto_minimo)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (isin) DO UPDATE SET
        super_sector = EXCLUDED.super_sector,
        mercato_segmento = EXCLUDED.mercato_segmento,
        capitalizzazione_di_mercato = EXCLUDED.capitalizzazione_di_mercato,
        lotto_minimo = EXCLUDED.lotto_minimo
        "#,
        market_information.isin,
        market_information.super_sector,
        market_information.mercato_segmento,
        market_information.capitalizzazione_di_mercato,
        market_information.lotto_minimo
    )
    .execute(&mut *tx)
    .await?;

    query!(
        r#"
        INSERT INTO price_data (
            isin,
            fase_di_mercato,
            prezzo_ultimo_contratto,
            var_percentuale,
            var_assoluta,
            pr_medio_progr,
            data_ora_ultimo_contratto,
            quantita_ultimo,
            quantita_totale,
            numero_contratti,
            controvalore,
            max_oggi,
            max_anno,
            max_anno_date,
            min_oggi,
            min_anno,
            min_anno_date,
            chiusura_precedente,
            prezzo_riferimento,
            data_ora_prezzo_rifermento,
            prezzo_ufficiale,
            data_prezzo_ufficiale,
            apertura_odierna
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23
        )
        ON CONFLICT (isin) DO UPDATE SET
        fase_di_mercato = EXCLUDED.fase_di_mercato,
        prezzo_ultimo_contratto = EXCLUDED.prezzo_ultimo_contratto,
        var_percentuale = EXCLUDED.var_percentuale,
        var_assoluta = EXCLUDED.var_assoluta,
        pr_medio_progr = EXCLUDED.pr_medio_progr,
        data_ora_ultimo_contratto = EXCLUDED.data_ora_ultimo_contratto,
        quantita_ultimo = EXCLUDED.quantita_ultimo,
        quantita_totale = EXCLUDED.quantita_totale,
        numero_contratti = EXCLUDED.numero_contratti,
        controvalore = EXCLUDED.controvalore,
        max_oggi = EXCLUDED.max_oggi,
        max_anno = EXCLUDED.max_anno,
        max_anno_date = EXCLUDED.max_anno_date,
        min_oggi = EXCLUDED.min_oggi,
        min_anno = EXCLUDED.min_anno,
        min_anno_date = EXCLUDED.min_anno_date,
        chiusura_precedente = EXCLUDED.chiusura_precedente,
        prezzo_riferimento = EXCLUDED.prezzo_riferimento,
        data_ora_prezzo_rifermento = EXCLUDED.data_ora_prezzo_rifermento,
        prezzo_ufficiale = EXCLUDED.prezzo_ufficiale,
        data_prezzo_ufficiale = EXCLUDED.data_prezzo_ufficiale,
        apertura_odierna = EXCLUDED.apertura_odierna
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
        price_data.numero_contratti as i64,
        price_data.controvalore,
        price_data.max_oggi,
        price_data.max_anno.price,
        price_data.max_anno.date,
        price_data.min_oggi,
        price_data.min_anno.price,
        price_data.min_anno.date,
        price_data.chiusura_precedente,
        price_data.prezzo_riferimento.price,
        price_data.prezzo_riferimento.datetime,
        price_data.prezzo_ufficiale.price,
        price_data.prezzo_ufficiale.date,
        price_data.apertura_odierna
    )
    .execute(&mut *tx)
    .await?;

    query!(
        r#"
        INSERT INTO performance_metrics (isin, performance_1_mese, performance_6_mesi, performance_1_anno)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (isin) DO UPDATE SET
        performance_1_mese = EXCLUDED.performance_1_mese,
        performance_6_mesi = EXCLUDED.performance_6_mesi,
        performance_1_anno = EXCLUDED.performance_1_anno
        "#,
        performance_metrics.isin, 
        performance_metrics.performance_1_mese,
        performance_metrics.performance_6_mesi,
        performance_metrics.performance_1_anno
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn get_share_by_isin(isin: &str, pool: &Pool<Postgres>) -> Result<Option<ShareFullInfo>, sqlx::Error> {
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
