// Configuration module
// Loads and validates configuration from CLI args, environment variables, and defaults

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

/// Kiro Gateway - Rust Implementation
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Server host address
    #[arg(short = 'H', long, env = "SERVER_HOST")]
    pub host: Option<String>,

    /// Server port
    #[arg(short, long, env = "SERVER_PORT")]
    pub port: Option<u16>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "LOG_LEVEL")]
    pub log_level: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Config {
    // Server settings
    pub server_host: String,
    pub server_port: u16,

    // Authentication
    pub proxy_api_key: String,

    // Kiro credentials
    pub kiro_region: String,
    pub kiro_cli_db_file: PathBuf,

    // Timeouts
    #[allow(dead_code)]
    pub streaming_timeout: u64,
    pub token_refresh_threshold: u64,
    pub first_token_timeout: u64,

    // HTTP client
    pub http_max_connections: usize,
    pub http_connect_timeout: u64,
    pub http_request_timeout: u64,
    pub http_max_retries: u32,

    // Debug
    pub debug_mode: DebugMode,
    pub log_level: String,

    // Converter settings
    pub tool_description_max_length: usize,
    pub fake_reasoning_enabled: bool,
    pub fake_reasoning_max_tokens: u32,
    #[allow(dead_code)]
    pub fake_reasoning_handling: FakeReasoningHandling,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FakeReasoningHandling {
    AsReasoningContent,  // Extract to reasoning_content field (OpenAI-compatible)
    Remove,              // Remove thinking block completely
    Pass,                // Pass through with original tags
    StripTags,           // Remove tags but keep content
}

#[derive(Clone, Debug, PartialEq)]
pub enum DebugMode {
    Off,
    Errors,
    All,
}

impl Config {
    /// Load configuration from all sources with priority: CLI > ENV > defaults
    pub fn load() -> Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        // Parse CLI arguments
        let args = CliArgs::parse();

        // Build config with priority handling
        let config = Config {
            // Server settings (CLI > ENV > default)
            server_host: args
                .host
                .or_else(|| std::env::var("SERVER_HOST").ok())
                .unwrap_or_else(|| "0.0.0.0".to_string()),

            server_port: args
                .port
                .or_else(|| std::env::var("SERVER_PORT").ok().and_then(|s| s.parse().ok()))
                .unwrap_or(8000),

            // Authentication (required)
            proxy_api_key: std::env::var("PROXY_API_KEY")
                .context("PROXY_API_KEY environment variable is required")?,

            // Kiro credentials
            kiro_region: std::env::var("KIRO_REGION").unwrap_or_else(|_| "us-east-1".to_string()),

            kiro_cli_db_file: std::env::var("KIRO_CLI_DB_FILE")
                .map(|s| expand_tilde(&s))
                .context("KIRO_CLI_DB_FILE environment variable is required")?,

            // Timeouts
            streaming_timeout: std::env::var("STREAMING_READ_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            token_refresh_threshold: std::env::var("TOKEN_REFRESH_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            first_token_timeout: std::env::var("FIRST_TOKEN_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(15),

            // HTTP client
            http_max_connections: std::env::var("HTTP_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),

            http_connect_timeout: std::env::var("HTTP_CONNECT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),

            http_request_timeout: std::env::var("HTTP_REQUEST_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            http_max_retries: std::env::var("HTTP_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),

            // Debug
            debug_mode: parse_debug_mode(&std::env::var("DEBUG_MODE").unwrap_or_default()),

            log_level: args
                .log_level
                .or_else(|| std::env::var("LOG_LEVEL").ok())
                .unwrap_or_else(|| "info".to_string()),

            // Converter settings
            tool_description_max_length: std::env::var("TOOL_DESCRIPTION_MAX_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),

            // Fake reasoning - enabled by default (like Python)
            // FAKE_REASONING env var: empty/"true"/"1"/"yes" = enabled, "false"/"0"/"no"/"disabled"/"off" = disabled
            fake_reasoning_enabled: {
                let raw = std::env::var("FAKE_REASONING").unwrap_or_default().to_lowercase();
                // Default is true - only disable if explicitly set to false/0/no/disabled/off
                !matches!(raw.as_str(), "false" | "0" | "no" | "disabled" | "off")
            },

            fake_reasoning_max_tokens: std::env::var("FAKE_REASONING_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4000),

            fake_reasoning_handling: parse_fake_reasoning_handling(
                &std::env::var("FAKE_REASONING_HANDLING").unwrap_or_default()
            ),
        };

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate that the SQLite database file exists
        if !self.kiro_cli_db_file.exists() {
            anyhow::bail!(
                "KIRO_CLI_DB_FILE does not exist: {}",
                self.kiro_cli_db_file.display()
            );
        }

        Ok(())
    }
}

/// Expand tilde (~) in file paths to user's home directory
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Parse debug mode from string
fn parse_debug_mode(s: &str) -> DebugMode {
    match s.to_lowercase().as_str() {
        "errors" => DebugMode::Errors,
        "all" => DebugMode::All,
        _ => DebugMode::Off,
    }
}

/// Parse fake reasoning handling mode from string
fn parse_fake_reasoning_handling(s: &str) -> FakeReasoningHandling {
    match s.to_lowercase().as_str() {
        "remove" => FakeReasoningHandling::Remove,
        "pass" => FakeReasoningHandling::Pass,
        "strip_tags" => FakeReasoningHandling::StripTags,
        _ => FakeReasoningHandling::AsReasoningContent, // default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let path = expand_tilde("~/test/file.txt");
        assert!(path.to_string_lossy().contains("test/file.txt"));
        assert!(!path.to_string_lossy().starts_with("~"));

        let path = expand_tilde("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_tilde_relative_path() {
        let path = expand_tilde("relative/path");
        assert_eq!(path, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_tilde_just_tilde() {
        // Just "~" without slash should not expand
        let path = expand_tilde("~");
        assert_eq!(path, PathBuf::from("~"));
    }

    #[test]
    fn test_parse_debug_mode() {
        assert_eq!(parse_debug_mode("off"), DebugMode::Off);
        assert_eq!(parse_debug_mode("errors"), DebugMode::Errors);
        assert_eq!(parse_debug_mode("all"), DebugMode::All);
        assert_eq!(parse_debug_mode("invalid"), DebugMode::Off);
        assert_eq!(parse_debug_mode(""), DebugMode::Off);
    }

    #[test]
    fn test_parse_debug_mode_case_insensitive() {
        assert_eq!(parse_debug_mode("ERRORS"), DebugMode::Errors);
        assert_eq!(parse_debug_mode("Errors"), DebugMode::Errors);
        assert_eq!(parse_debug_mode("ALL"), DebugMode::All);
        assert_eq!(parse_debug_mode("All"), DebugMode::All);
        assert_eq!(parse_debug_mode("OFF"), DebugMode::Off);
    }

    #[test]
    fn test_parse_fake_reasoning_handling() {
        assert_eq!(
            parse_fake_reasoning_handling(""),
            FakeReasoningHandling::AsReasoningContent
        );
        assert_eq!(
            parse_fake_reasoning_handling("remove"),
            FakeReasoningHandling::Remove
        );
        assert_eq!(
            parse_fake_reasoning_handling("pass"),
            FakeReasoningHandling::Pass
        );
        assert_eq!(
            parse_fake_reasoning_handling("strip_tags"),
            FakeReasoningHandling::StripTags
        );
    }

    #[test]
    fn test_parse_fake_reasoning_handling_case_insensitive() {
        assert_eq!(
            parse_fake_reasoning_handling("REMOVE"),
            FakeReasoningHandling::Remove
        );
        assert_eq!(
            parse_fake_reasoning_handling("Remove"),
            FakeReasoningHandling::Remove
        );
        assert_eq!(
            parse_fake_reasoning_handling("PASS"),
            FakeReasoningHandling::Pass
        );
        assert_eq!(
            parse_fake_reasoning_handling("STRIP_TAGS"),
            FakeReasoningHandling::StripTags
        );
    }

    #[test]
    fn test_parse_fake_reasoning_handling_default() {
        // Unknown values should default to AsReasoningContent
        assert_eq!(
            parse_fake_reasoning_handling("unknown"),
            FakeReasoningHandling::AsReasoningContent
        );
        assert_eq!(
            parse_fake_reasoning_handling("invalid"),
            FakeReasoningHandling::AsReasoningContent
        );
    }

    #[test]
    fn test_debug_mode_equality() {
        assert_eq!(DebugMode::Off, DebugMode::Off);
        assert_eq!(DebugMode::Errors, DebugMode::Errors);
        assert_eq!(DebugMode::All, DebugMode::All);
        assert_ne!(DebugMode::Off, DebugMode::Errors);
        assert_ne!(DebugMode::Errors, DebugMode::All);
    }

    #[test]
    fn test_fake_reasoning_handling_equality() {
        assert_eq!(
            FakeReasoningHandling::AsReasoningContent,
            FakeReasoningHandling::AsReasoningContent
        );
        assert_eq!(FakeReasoningHandling::Remove, FakeReasoningHandling::Remove);
        assert_eq!(FakeReasoningHandling::Pass, FakeReasoningHandling::Pass);
        assert_eq!(
            FakeReasoningHandling::StripTags,
            FakeReasoningHandling::StripTags
        );
        assert_ne!(FakeReasoningHandling::Remove, FakeReasoningHandling::Pass);
    }
}
