use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::credentials;
use super::refresh;
use super::types::{AuthType, Credentials};

/// Authentication manager
/// Manages token lifecycle with automatic refresh and thread-safe access
pub struct AuthManager {
    /// Current credentials
    credentials: Arc<RwLock<Credentials>>,

    /// Current access token
    access_token: Arc<RwLock<Option<String>>>,

    /// Token expiration time
    expires_at: Arc<RwLock<Option<DateTime<Utc>>>>,

    /// Authentication type
    auth_type: AuthType,

    /// HTTP client for refresh requests
    client: Client,

    /// Path to SQLite database (for reload on 400 error)
    sqlite_db: Option<PathBuf>,

    /// Token refresh threshold in seconds (default: 300 = 5 minutes)
    refresh_threshold: i64,
}

impl AuthManager {
    /// Create a new AuthManager for testing (no SQLite required)
    /// Available in test builds and integration tests
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_testing(
        access_token: String,
        region: String,
        refresh_threshold: u64,
    ) -> Result<Self> {
        use super::types::Credentials;

        let credentials = Credentials {
            refresh_token: "test-refresh-token".to_string(),
            access_token: Some(access_token.clone()),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            profile_arn: None,
            region: region.clone(),
            client_id: Some("test-client-id".to_string()),
            client_secret: Some("test-client-secret".to_string()),
            sso_region: Some(region),
            scopes: None,
        };

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            credentials: Arc::new(RwLock::new(credentials)),
            access_token: Arc::new(RwLock::new(Some(access_token))),
            expires_at: Arc::new(RwLock::new(Some(Utc::now() + Duration::hours(1)))),
            auth_type: AuthType::AwsSsoOidc,
            client,
            sqlite_db: None,
            refresh_threshold: refresh_threshold as i64,
        })
    }

    /// Create a new AuthManager from SQLite database
    pub fn new(sqlite_db: PathBuf, refresh_threshold: u64) -> Result<Self> {
        // Load credentials from SQLite database
        tracing::info!("Loading credentials from SQLite: {}", sqlite_db.display());
        let credentials = credentials::load_from_sqlite(&sqlite_db)?;

        // Detect authentication type (should always be AWS SSO OIDC for kiro-cli)
        let auth_type = credentials::detect_auth_type(&credentials);

        // Extract initial token data
        let access_token = credentials.access_token.clone();
        let expires_at = credentials.expires_at;

        // Create HTTP client with timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            credentials: Arc::new(RwLock::new(credentials)),
            access_token: Arc::new(RwLock::new(access_token)),
            expires_at: Arc::new(RwLock::new(expires_at)),
            auth_type,
            client,
            sqlite_db: Some(sqlite_db),
            refresh_threshold: refresh_threshold as i64,
        })
    }

    /// Check if token is expiring soon (within threshold)
    async fn is_token_expiring_soon(&self) -> bool {
        let expires_at = self.expires_at.read().await;

        match *expires_at {
            None => true, // No expiration info, assume refresh needed
            Some(exp) => {
                let now = Utc::now();
                let threshold = now + Duration::seconds(self.refresh_threshold);
                exp <= threshold
            }
        }
    }

    /// Check if token is actually expired (not just expiring soon)
    async fn is_token_expired(&self) -> bool {
        let expires_at = self.expires_at.read().await;

        match *expires_at {
            None => true, // No expiration info, assume expired
            Some(exp) => Utc::now() >= exp,
        }
    }

    /// Refresh the access token
    async fn refresh_token(&self) -> Result<()> {
        tracing::debug!("Refreshing access token...");

        // Get mutable access to credentials
        let mut creds = self.credentials.write().await;

        // Perform refresh with retry logic
        let token_data = refresh::refresh_with_retry(
            &self.client,
            self.auth_type.clone(),
            &mut creds,
            self.sqlite_db.as_deref(),
        )
        .await?;

        // Update stored token data
        {
            let mut access_token = self.access_token.write().await;
            *access_token = Some(token_data.access_token.clone());
        }

        {
            let mut expires_at = self.expires_at.write().await;
            *expires_at = Some(token_data.expires_at);
        }

        // Update credentials with new refresh token if provided
        if let Some(ref new_refresh_token) = token_data.refresh_token {
            creds.refresh_token = new_refresh_token.clone();
        }

        if let Some(ref new_profile_arn) = token_data.profile_arn {
            creds.profile_arn = Some(new_profile_arn.clone());
        }

        Ok(())
    }

    /// Get a valid access token, refreshing if necessary
    /// Thread-safe method that ensures only one refresh occurs at a time
    pub async fn get_access_token(&self) -> Result<String> {
        // Check if refresh is needed
        if self.is_token_expiring_soon().await {
            // Attempt refresh
            if let Err(e) = self.refresh_token().await {
                tracing::error!("Token refresh failed: {}", e);

                // Graceful degradation: if token isn't actually expired yet, use it
                if !self.is_token_expired().await {
                    tracing::warn!(
                        "Using existing token despite refresh failure (not yet expired)"
                    );
                    let token = self.access_token.read().await;
                    if let Some(ref t) = *token {
                        return Ok(t.clone());
                    }
                }

                return Err(e).context("Failed to refresh token and no valid token available");
            }
        }

        // Return current token
        let token = self.access_token.read().await;
        token.as_ref().cloned().context("No access token available")
    }

    /// Get the region
    pub async fn get_region(&self) -> String {
        let creds = self.credentials.read().await;
        creds.region.clone()
    }

    /// Get the profile ARN
    pub async fn get_profile_arn(&self) -> Option<String> {
        let creds = self.credentials.read().await;
        creds.profile_arn.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_expiration_check() {
        let creds = Credentials {
            refresh_token: "test".to_string(),
            access_token: Some("token".to_string()),
            expires_at: Some(Utc::now() + Duration::seconds(600)), // 10 minutes from now
            profile_arn: None,
            region: "us-east-1".to_string(),
            client_id: None,
            client_secret: None,
            sso_region: None,
            scopes: None,
        };

        let manager = AuthManager {
            credentials: Arc::new(RwLock::new(creds)),
            access_token: Arc::new(RwLock::new(Some("token".to_string()))),
            expires_at: Arc::new(RwLock::new(Some(Utc::now() + Duration::seconds(600)))),
            auth_type: AuthType::KiroDesktop,
            client: Client::new(),
            sqlite_db: None,
            refresh_threshold: 300,
        };

        // Token expires in 10 minutes, threshold is 5 minutes - should not need refresh
        assert!(!manager.is_token_expiring_soon().await);

        // Update to expire in 2 minutes - should need refresh
        {
            let mut expires_at = manager.expires_at.write().await;
            *expires_at = Some(Utc::now() + Duration::seconds(120));
        }
        assert!(manager.is_token_expiring_soon().await);
    }

    #[tokio::test]
    async fn test_token_expired_check() {
        let manager = AuthManager {
            credentials: Arc::new(RwLock::new(Credentials {
                refresh_token: "test".to_string(),
                access_token: None,
                expires_at: None,
                profile_arn: None,
                region: "us-east-1".to_string(),
                client_id: None,
                client_secret: None,
                sso_region: None,
                scopes: None,
            })),
            access_token: Arc::new(RwLock::new(None)),
            expires_at: Arc::new(RwLock::new(Some(Utc::now() - Duration::seconds(60)))),
            auth_type: AuthType::KiroDesktop,
            client: Client::new(),
            sqlite_db: None,
            refresh_threshold: 300,
        };

        // Token expired 1 minute ago
        assert!(manager.is_token_expired().await);
    }
}
