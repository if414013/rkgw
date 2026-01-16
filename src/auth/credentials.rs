// Credential loading from SQLite database

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::Path;

use super::types::{AuthType, Credentials, SqliteDeviceRegistration, SqliteTokenData};

/// Load credentials from SQLite database (kiro-cli)
pub fn load_from_sqlite(path: &Path) -> Result<Credentials> {
    let conn = rusqlite::Connection::open(path)
        .with_context(|| format!("Failed to open SQLite database: {}", path.display()))?;

    // Load token data (try both key formats)
    let token_json: String = conn
        .query_row(
            "SELECT value FROM auth_kv WHERE key = ?",
            ["kirocli:odic:token"],
            |row| row.get(0),
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT value FROM auth_kv WHERE key = ?",
                ["codewhisperer:odic:token"],
                |row| row.get(0),
            )
        })
        .context("Failed to load token data from SQLite")?;

    let token_data: SqliteTokenData =
        serde_json::from_str(&token_json).context("Failed to parse token data from SQLite")?;

    // Load device registration (try both key formats)
    let registration_json: String = conn
        .query_row(
            "SELECT value FROM auth_kv WHERE key = ?",
            ["kirocli:odic:device-registration"],
            |row| row.get(0),
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT value FROM auth_kv WHERE key = ?",
                ["codewhisperer:odic:device-registration"],
                |row| row.get(0),
            )
        })
        .context("Failed to load device registration from SQLite")?;

    let registration: SqliteDeviceRegistration = serde_json::from_str(&registration_json)
        .context("Failed to parse device registration from SQLite")?;

    let refresh_token = token_data
        .refresh_token
        .context("SQLite token data must contain refresh_token")?;

    let expires_at = token_data.expires_at.and_then(|s| parse_datetime(&s).ok());

    // SSO region is used for OIDC token refresh only
    // API region stays as us-east-1 (CodeWhisperer is only available there)
    let sso_region = token_data.region.or(registration.region);

    Ok(Credentials {
        refresh_token,
        access_token: token_data.access_token,
        expires_at,
        profile_arn: None,
        region: "us-east-1".to_string(), // CodeWhisperer API region
        client_id: registration.client_id,
        client_secret: registration.client_secret,
        sso_region,
        scopes: token_data.scopes,
    })
}

/// Detect authentication type based on credentials
pub fn detect_auth_type(creds: &Credentials) -> AuthType {
    if creds.client_id.is_some() && creds.client_secret.is_some() {
        tracing::info!("Detected auth type: AWS SSO OIDC (kiro-cli)");
        AuthType::AwsSsoOidc
    } else {
        tracing::warn!("Missing client_id or client_secret - this should not happen with kiro-cli");
        AuthType::AwsSsoOidc
    }
}

/// Parse datetime from various ISO 8601 formats
fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    // Handle Z suffix
    let normalized = if s.ends_with('Z') {
        s.replace('Z', "+00:00")
    } else {
        s.to_string()
    };

    DateTime::parse_from_rfc3339(&normalized)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("Failed to parse datetime: {}", s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        // Test with Z suffix
        let dt = parse_datetime("2025-01-12T10:30:00Z").unwrap();
        assert_eq!(dt.to_rfc3339(), "2025-01-12T10:30:00+00:00");

        // Test with timezone
        let dt = parse_datetime("2025-01-12T10:30:00+00:00").unwrap();
        assert_eq!(dt.to_rfc3339(), "2025-01-12T10:30:00+00:00");
    }

    #[test]
    fn test_detect_auth_type() {
        let creds = Credentials {
            refresh_token: "token".to_string(),
            access_token: None,
            expires_at: None,
            profile_arn: None,
            region: "us-east-1".to_string(),
            client_id: Some("client".to_string()),
            client_secret: Some("secret".to_string()),
            sso_region: None,
            scopes: None,
        };
        assert_eq!(detect_auth_type(&creds), AuthType::AwsSsoOidc);
    }
}
