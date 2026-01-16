// HTTP client with retry logic and connection pooling

use anyhow::{Context, Result};
use reqwest::{Client, Request, Response};
use std::sync::Arc;
use std::time::Duration;

use crate::auth::AuthManager;
use crate::error::ApiError;

/// HTTP client for Kiro API with retry logic
pub struct KiroHttpClient {
    /// Shared HTTP client with connection pooling
    client: Client,

    /// Authentication manager
    auth_manager: Arc<AuthManager>,

    /// Maximum number of retries
    max_retries: u32,

    /// Base delay for exponential backoff (milliseconds)
    base_delay_ms: u64,
}

impl KiroHttpClient {
    /// Create a new HTTP client
    pub fn new(
        auth_manager: Arc<AuthManager>,
        max_connections: usize,
        connect_timeout: u64,
        request_timeout: u64,
        max_retries: u32,
    ) -> Result<Self> {
        let client = Client::builder()
            .pool_max_idle_per_host(max_connections)
            .connect_timeout(Duration::from_secs(connect_timeout))
            .timeout(Duration::from_secs(request_timeout))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            auth_manager,
            max_retries,
            base_delay_ms: 1000, // 1 second base delay
        })
    }

    /// Execute a request with retry logic
    /// Automatically handles:
    /// - 403: refreshes token and retries
    /// - 429: exponential backoff
    /// - 5xx: exponential backoff
    pub async fn request_with_retry(&self, request: Request) -> Result<Response, ApiError> {
        self.request_with_retry_internal(request, true).await
    }

    /// Execute a request without retries (for startup/initialization)
    /// Fails fast on any error
    pub async fn request_no_retry(&self, request: Request) -> Result<Response, ApiError> {
        self.request_with_retry_internal(request, false).await
    }

    /// Internal method that handles retry logic
    async fn request_with_retry_internal(
        &self,
        mut request: Request,
        enable_retry: bool,
    ) -> Result<Response, ApiError> {
        let max_retries = if enable_retry { self.max_retries } else { 0 };
        let mut attempt = 0;

        loop {
            // Clone the request for this attempt
            let req = request.try_clone().ok_or_else(|| {
                ApiError::Internal(anyhow::anyhow!("Request body is not cloneable"))
            })?;

            // Execute request
            let result = self.client.execute(req).await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    // Success
                    if status.is_success() {
                        return Ok(response);
                    }

                    // Handle specific error codes
                    match status.as_u16() {
                        // 403: Refresh token and retry
                        403 => {
                            if attempt < max_retries {
                                tracing::warn!("Received 403, refreshing token and retrying...");

                                // Refresh token
                                if let Err(e) = self.auth_manager.get_access_token().await {
                                    tracing::error!("Token refresh failed: {}", e);
                                    return Err(ApiError::AuthError(format!(
                                        "Token refresh failed: {}",
                                        e
                                    )));
                                }

                                // Update Authorization header in request
                                let token = self
                                    .auth_manager
                                    .get_access_token()
                                    .await
                                    .map_err(|e| ApiError::AuthError(e.to_string()))?;
                                request.headers_mut().insert(
                                    "Authorization",
                                    format!("Bearer {}", token).parse().unwrap(),
                                );

                                attempt += 1;
                                continue;
                            }
                        }

                        // 429 or 5xx: Exponential backoff
                        429 | 500..=599 => {
                            if attempt < max_retries {
                                let delay = self.calculate_backoff_delay(attempt);
                                tracing::warn!(
                                    "Received {}, retrying after {}ms (attempt {}/{})",
                                    status,
                                    delay,
                                    attempt + 1,
                                    max_retries
                                );

                                tokio::time::sleep(Duration::from_millis(delay)).await;
                                attempt += 1;
                                continue;
                            }
                        }

                        _ => {}
                    }

                    // Non-retryable error or max retries exceeded
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(ApiError::KiroApiError {
                        status: status.as_u16(),
                        message: error_text,
                    });
                }

                Err(e) => {
                    // Network error - retry with backoff
                    if attempt < max_retries {
                        let delay = self.calculate_backoff_delay(attempt);
                        tracing::warn!(
                            "Request failed: {}, retrying after {}ms (attempt {}/{})",
                            e,
                            delay,
                            attempt + 1,
                            max_retries
                        );

                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        attempt += 1;
                        continue;
                    }

                    return Err(ApiError::Internal(anyhow::anyhow!(
                        "HTTP request failed: {}",
                        e
                    )));
                }
            }
        }
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff_delay(&self, attempt: u32) -> u64 {
        // Exponential backoff: base_delay * 2^attempt
        // With jitter to avoid thundering herd
        let delay = self.base_delay_ms * 2_u64.pow(attempt);
        let jitter = (delay as f64 * 0.1 * rand::random()) as u64;
        delay + jitter
    }

    /// Get the underlying HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }
}

// Simple random number generation for jitter
mod rand {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    pub fn random() -> f64 {
        let state = RandomState::new();
        let mut hasher = state.build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        (hasher.finish() % 1000) as f64 / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        let auth_manager = Arc::new(
            AuthManager::new_for_testing("test-token".to_string(), "us-east-1".to_string(), 300)
                .unwrap(),
        );

        let client = KiroHttpClient::new(auth_manager, 20, 30, 300, 3).unwrap();

        // Test exponential backoff
        let delay0 = client.calculate_backoff_delay(0);
        let delay1 = client.calculate_backoff_delay(1);
        let delay2 = client.calculate_backoff_delay(2);

        // Each delay should be roughly double the previous (with jitter)
        assert!(delay0 >= 1000 && delay0 <= 1200); // ~1s with jitter
        assert!(delay1 >= 2000 && delay1 <= 2400); // ~2s with jitter
        assert!(delay2 >= 4000 && delay2 <= 4800); // ~4s with jitter
    }
}
