use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::{extract::State, routing::get, Router};
use db::isins::query_all_isins;
use db::shares::{query_share_with, ShareQuery};
use scraper::shares::Share;
use sqlx::PgPool;
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() {
    let log_file = std::fs::File::create("../server.log").expect("Can't create log file");

    let file_logger = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false);
    let stdout_logger = tracing_subscriber::fmt::layer().with_ansi(true);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(file_logger)
        .with(stdout_logger)
        .init();

    info!("Starting server...");

    let db = match db::connect().await {
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

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();

    info!("Started server at http://127.0.0.1:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
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
