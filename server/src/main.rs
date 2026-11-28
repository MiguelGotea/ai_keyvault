mod db;
mod error;
mod routes;
mod stats;

use axum::{
    routing::{get, post},
    Router,
};
use routes::AppState;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info,ai_keyvault_server=debug".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/keyvault.db?mode=rwc".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3010".into())
        .parse()
        .unwrap_or(3010);
    let server_master_key =
        std::env::var("SERVER_MASTER_KEY").unwrap_or_else(|_| "default_master_key_for_dev".into());

    info!(
        "Iniciando conexión a base de datos SQLite: {}",
        database_url
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Inicializando tablas y esquemas de BD...");
    db::init_db(&pool).await?;

    let state = Arc::new(AppState {
        db: pool,
        _server_master_key: server_master_key,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/api/devices", get(routes::list_devices))
        .route("/api/devices/revoke", post(routes::revoke_device))
        .route("/api/quota/lease", post(routes::acquire_lease))
        .route("/api/quota/readings", post(routes::post_quota_reading))
        .route(
            "/api/quota/readings/:account_id",
            get(routes::get_quota_readings),
        )
        .route("/api/chats", post(routes::post_chat_sessions))
        .route("/api/chats/:account_id", get(routes::get_chat_sessions))
        .route("/api/pings", post(routes::post_ping))
        .route("/api/pings/needs", get(routes::get_pings_needs))
        .route("/api/pings/:account_id", get(routes::get_pings))
        .route("/api/switch", post(routes::post_switch))
        .route("/api/switch/history", get(routes::get_switch_history))
        .route(
            "/api/alerts",
            post(routes::post_alert).get(routes::get_alert_log),
        )
        .route("/api/stats/compare", get(routes::get_stats_compare))
        .route(
            "/api/stats/summary/:account_id",
            get(routes::get_stats_summary),
        )
        .route(
            "/api/stats/heatmap/:account_id",
            get(routes::get_stats_heatmap),
        )
        .route(
            "/api/stats/weekly/:account_id",
            get(routes::get_stats_weekly),
        )
        .route(
            "/api/stats/export/:account_id",
            get(routes::get_stats_export),
        )
        .route("/api/auth/register", post(routes::auth_register))
        .route("/api/auth/params", post(routes::auth_params))
        .route("/api/auth/login", post(routes::auth_login))
        .route("/api/auth/vault", post(routes::auth_upload_vault))
        .route(
            "/api/auth/vault/download",
            post(routes::auth_download_vault),
        )
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Servidor AI KeyVault escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
