// Error handling module
// Defines error types and HTTP response conversion

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// API errors that can occur during request processing
#[derive(Error, Debug)]
pub enum ApiError {
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Invalid model name
    #[error("Invalid model: {0}")]
    #[allow(dead_code)]
    InvalidModel(String),

    /// Error from Kiro API
    #[error("Kiro API error: {status} - {message}")]
    KiroApiError { status: u16, message: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    ConfigError(String),

    /// Request validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::AuthError(msg) => (StatusCode::UNAUTHORIZED, "auth_error", msg),
            ApiError::InvalidModel(msg) => (StatusCode::BAD_REQUEST, "invalid_model", msg),
            ApiError::KiroApiError { status, message } => {
                let status_code =
                    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status_code, "kiro_api_error", message)
            }
            ApiError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "config_error", msg),
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, "validation_error", msg),
            ApiError::Internal(err) => {
                // Log internal errors
                tracing::error!("Internal error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": {
                "message": message,
                "type": error_type,
            }
        }));

        (status, body).into_response()
    }
}

/// Result type alias for API operations
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = ApiError::AuthError("Invalid token".to_string());
        assert_eq!(err.to_string(), "Authentication failed: Invalid token");

        let err = ApiError::InvalidModel("gpt-4".to_string());
        assert_eq!(err.to_string(), "Invalid model: gpt-4");

        let err = ApiError::KiroApiError {
            status: 429,
            message: "Rate limit exceeded".to_string(),
        };
        assert_eq!(err.to_string(), "Kiro API error: 429 - Rate limit exceeded");
    }

    #[test]
    fn test_config_error_message() {
        let err = ApiError::ConfigError("Missing API key".to_string());
        assert_eq!(err.to_string(), "Configuration error: Missing API key");
    }

    #[test]
    fn test_validation_error_message() {
        let err = ApiError::ValidationError("Invalid JSON".to_string());
        assert_eq!(err.to_string(), "Validation error: Invalid JSON");
    }

    #[test]
    fn test_internal_error_message() {
        let err = ApiError::Internal(anyhow::anyhow!("Something went wrong"));
        assert_eq!(err.to_string(), "Internal error: Something went wrong");
    }

    #[tokio::test]
    async fn test_error_response_conversion() {
        let err = ApiError::AuthError("Invalid token".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let err = ApiError::InvalidModel("gpt-4".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = ApiError::KiroApiError {
            status: 429,
            message: "Rate limit".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_config_error_response() {
        let err = ApiError::ConfigError("Bad config".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_validation_error_response() {
        let err = ApiError::ValidationError("Missing field".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_internal_error_response() {
        let err = ApiError::Internal(anyhow::anyhow!("Unexpected error"));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_kiro_api_error_invalid_status() {
        // Test with an invalid status code (must be >= 1000 to be invalid)
        // HTTP status codes 100-999 are valid
        let err = ApiError::KiroApiError {
            status: 1000, // Invalid HTTP status (out of range)
            message: "Unknown error".to_string(),
        };
        let response = err.into_response();
        // Invalid status codes fall back to 500 Internal Server Error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_kiro_api_error_various_statuses() {
        // Test 400 Bad Request
        let err = ApiError::KiroApiError {
            status: 400,
            message: "Bad request".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test 403 Forbidden
        let err = ApiError::KiroApiError {
            status: 403,
            message: "Forbidden".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Test 404 Not Found
        let err = ApiError::KiroApiError {
            status: 404,
            message: "Not found".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Test 500 Internal Server Error
        let err = ApiError::KiroApiError {
            status: 500,
            message: "Server error".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test 503 Service Unavailable
        let err = ApiError::KiroApiError {
            status: 503,
            message: "Service unavailable".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
