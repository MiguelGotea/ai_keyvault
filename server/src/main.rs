mod db;
mod error;
mod routes;

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
        server_master_key,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/api/devices/register", post(routes::register_device))
        .route(
            "/api/vault/sync",
            post(routes::upload_vault).get(routes::download_vault),
        )
        .route("/api/quota/lease", post(routes::acquire_lease))
        .route("/api/quota/readings", post(routes::post_quota_reading))
        .route(
            "/api/quota/readings/:account_id",
            get(routes::get_quota_readings),
        )
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Servidor AI KeyVault escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
