use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Base de datos error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("No autorizado: {0}")]
    #[allow(dead_code)]
    Unauthorized(String),

    #[error("Conflicto o bloqueo activo: {0}")]
    #[allow(dead_code)]
    Conflict(String),

    #[error("Petición inválida: {0}")]
    BadRequest(String),

    #[error("Error interno del servidor: {0}")]
    Internal(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ServerError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error de BD: {}", e),
            ),
            ServerError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ServerError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
