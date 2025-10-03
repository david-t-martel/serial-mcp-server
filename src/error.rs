#[cfg(feature = "rest-api")]
use axum::{http::StatusCode, response::{IntoResponse, Response}};
use std::fmt;

#[cfg(feature = "rest-api")]
use serde_json::json;

/// A specialized `Result` type for REST handlers (only when the rest-api feature is enabled).
#[cfg(feature = "rest-api")]
pub type AppResult<T> = Result<T, AppError>;

/// Unified application error type.
///
/// Most variants are currently used by the legacy stdio or (future) REST interface.
/// The MCP path relies on `CallToolError` instead; we intentionally keep this
/// type lightweight and fully non-panicking for compatibility.
#[derive(Debug)]
#[allow(dead_code)] // Some variants may be unused depending on feature flags.
pub enum AppError {
    PortNotOpen,
    PortAlreadyOpen,
    InvalidPayload(String),
    SerialError(serialport::Error),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PortNotOpen => write!(f, "Operation requires an open serial port, but the port is closed."),
            Self::PortAlreadyOpen => write!(f, "Port is already open. Close it before trying to open it again."),
            Self::InvalidPayload(details) => write!(f, "The request payload is invalid: {details}"),
            Self::SerialError(e) => write!(f, "A serial port error occurred: {e}"),
            Self::IoError(e) => write!(f, "An I/O error occurred: {e}"),
            Self::SerdeError(e) => write!(f, "A serialization/deserialization error occurred: {e}"),
        }
    }
}

/// Allows Axum to convert `AppError` into an HTTP response (only when rest-api feature enabled).
#[cfg(feature = "rest-api")]
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match self {
            Self::PortNotOpen => (StatusCode::CONFLICT, "PortNotOpen", self.to_string()),
            Self::PortAlreadyOpen => (StatusCode::CONFLICT, "PortAlreadyOpen", self.to_string()),
            Self::InvalidPayload(_) => (StatusCode::BAD_REQUEST, "InvalidPayload", self.to_string()),
            Self::SerialError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SerialError", self.to_string()),
            Self::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IoError", self.to_string()),
            Self::SerdeError(_) => (StatusCode::BAD_REQUEST, "DeserializationError", self.to_string()),
        };

        let body = axum::Json(json!({
            "status": "error",
            "error": { "type": error_type, "message": error_message }
        }));
        (status, body).into_response()
    }
}

// Implement `From` conversions to allow the `?` operator to work seamlessly.
impl From<serialport::Error> for AppError {
    fn from(err: serialport::Error) -> Self {
        AppError::SerialError(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::SerdeError(err)
    }
}
