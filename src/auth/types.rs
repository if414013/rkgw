// Authentication types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of authentication mechanism
#[derive(Debug, Clone, PartialEq)]
pub enum AuthType {
    /// Kiro IDE credentials (default)
    /// Uses https://prod.{region}.auth.desktop.kiro.dev/refreshToken
    #[allow(dead_code)]
    KiroDesktop,

    /// AWS SSO credentials from kiro-cli
    /// Uses https://oidc.{region}.amazonaws.com/token
    AwsSsoOidc,
}

/// Complete credential set
#[derive(Debug, Clone)]
pub struct Credentials {
    pub refresh_token: String,
    pub access_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub profile_arn: Option<String>,
    pub region: String,

    // AWS SSO OIDC specific fields
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub sso_region: Option<String>,
    #[allow(dead_code)]
    pub scopes: Option<Vec<String>>,
}

/// Token data from refresh response
#[derive(Debug, Clone)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub profile_arn: Option<String>,
}

/// Kiro Desktop refresh request
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroRefreshRequest {
    pub refresh_token: String,
}

/// Kiro Desktop refresh response
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroRefreshResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub profile_arn: Option<String>,
}

/// AWS SSO OIDC refresh response
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwsSsoOidcResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// SQLite token data
#[derive(Deserialize)]
pub struct SqliteTokenData {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub region: Option<String>,
    pub scopes: Option<Vec<String>>,
}

/// SQLite device registration data
#[derive(Deserialize)]
pub struct SqliteDeviceRegistration {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
}
