use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// A specialized `Result` type for this application's API handlers.
pub type AppResult<T> = Result<T, AppError>;

/// The single, unified error type for the entire application.
///
/// This enum represents all possible failures that our service can encounter.
/// By implementing `IntoResponse`, we can return this enum directly from our
/// handlers and have Axum automatically convert it into a well-formed JSON
/// error response.
#[derive(Debug)]
pub enum AppError {
    PortNotOpen,
    PortAlreadyOpen,
    InvalidPayload(String),
    SerialError(serialport::Error),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

/// Provides user-friendly, human-readable error messages.
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::PortNotOpen => write!(f, "Operation requires an open serial port, but the port is closed."),
            AppError::PortAlreadyOpen => write!(f, "Port is already open. Close it before trying to open it again."),
            AppError::InvalidPayload(details) => write!(f, "The request payload is invalid: {}", details),
            AppError::SerialError(e) => write!(f, "A serial port error occurred: {}", e),
            AppError::IoError(e) => write!(f, "An I/O error occurred: {}", e),
            AppError::SerdeError(e) => write!(f, "A serialization/deserialization error occurred: {}", e),
        }
    }
}

/// Allows Axum to convert `AppError` into an HTTP response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match self {
            AppError::PortNotOpen => (StatusCode::CONFLICT, "PortNotOpen", self.to_string()),
            AppError::PortAlreadyOpen => (StatusCode::CONFLICT, "PortAlreadyOpen", self.to_string()),
            AppError::InvalidPayload(_) => (StatusCode::BAD_REQUEST, "InvalidPayload", self.to_string()),
            AppError::SerialError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SerialError", self.to_string()),
            AppError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IoError", self.to_string()),
            AppError::SerdeError(_) => (StatusCode::BAD_REQUEST, "DeserializationError", self.to_string()),
        };

        let body = Json(json!({
            "status": "error",
            "error": {
                "type": error_type,
                "message": error_message,
            }
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
