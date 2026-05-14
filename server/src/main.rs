use app_config::{load_config, LoggingConfig};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::{extract::State, routing::get, Router};
use db::isins::query_all_isins;
use db::shares::{query_share_with, ShareQuery};
use scraper::shares::Share;
use sqlx::PgPool;
use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config(env!("CARGO_MANIFEST_DIR"))?;
    init_logging(&config.logging)?;

    info!("Starting server...");

    let db = match db::connect(&config.database).await {
        Ok(pool) => pool,
        Err(e) => {
            error!("Error connecting to db: {}", e);
            panic!()
        }
    };

    let shared_state = Arc::new(AppState { db });

    let app = Router::new()
        .route("/all_isins", get(all_isins))
        .route("/all_shares", get(all_shares))
        .route("/share", get(query_share))
        .with_state(shared_state);

    let listener = TcpListener::bind(config.server.bind_address).await?;

    info!("Started server at http://{}", config.server.bind_address);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let log_file = File::create(&config.server_file_path)?;

    let file_logger = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false);
    let env_filter = EnvFilter::try_new(&config.level)?;

    if config.stdout {
        let stdout_logger = tracing_subscriber::fmt::layer().with_ansi(true);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_logger)
            .with(stdout_logger)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_logger)
            .init();
    }

    Ok(())
}

async fn all_isins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match query_all_isins(&state.db).await {
        Ok(isins) => Json(isins).into_response(),
        Err(err) => {
            error!("Error fetching all isins: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn all_shares(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match query_share_with(ShareQuery::empty(), &state.db).await {
        Ok(shares) => Json(shares).into_response(),
        Err(err) => {
            error!("Error fetching all shares: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn query_share(
    Query(query): Query<ShareQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match query_share_with(query, &state.db).await {
        Ok(shares) => Json::<Vec<Share>>(shares).into_response(),
        Err(db_err) => {
            error!("DB error: {}", db_err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
