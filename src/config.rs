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
    #[arg(short = 'H', long, env = "SERVER_HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Server port
    #[arg(short, long, env = "SERVER_PORT", default_value = "8000")]
    pub port: u16,

    /// Proxy API key for client authentication
    #[arg(short = 'k', long, env = "PROXY_API_KEY")]
    pub api_key: Option<String>,

    /// Path to kiro-cli SQLite database
    #[arg(short = 'd', long, env = "KIRO_CLI_DB_FILE")]
    pub db_file: Option<String>,

    /// AWS region for Kiro API
    #[arg(short = 'r', long, env = "KIRO_REGION", default_value = "us-east-1")]
    pub region: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Debug mode (off, errors, all)
    #[arg(long, env = "DEBUG_MODE", default_value = "off")]
    pub debug_mode: String,

    /// Enable fake reasoning/extended thinking
    #[arg(long, env = "FAKE_REASONING", default_value = "true")]
    pub fake_reasoning: bool,

    /// Max tokens for fake reasoning
    #[arg(long, env = "FAKE_REASONING_MAX_TOKENS", default_value = "4000")]
    pub fake_reasoning_max_tokens: u32,

    /// First token timeout in seconds
    #[arg(long, env = "FIRST_TOKEN_TIMEOUT", default_value = "15")]
    pub first_token_timeout: u64,

    /// HTTP request timeout in seconds
    #[arg(long, env = "HTTP_REQUEST_TIMEOUT", default_value = "300")]
    pub http_timeout: u64,

    /// HTTP max retries
    #[arg(long, env = "HTTP_MAX_RETRIES", default_value = "3")]
    pub http_retries: u32,
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
    AsReasoningContent, // Extract to reasoning_content field (OpenAI-compatible)
    Remove,             // Remove thinking block completely
    Pass,               // Pass through with original tags
    StripTags,          // Remove tags but keep content
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
            // Server settings (from CLI with defaults)
            server_host: args.host,
            server_port: args.port,

            // Authentication (CLI > ENV, required)
            proxy_api_key: args
                .api_key
                .or_else(|| std::env::var("PROXY_API_KEY").ok())
                .context("PROXY_API_KEY is required (use -k or set PROXY_API_KEY env var)")?,

            // Kiro credentials
            kiro_region: args.region,

            kiro_cli_db_file: args
                .db_file
                .map(|s| expand_tilde(&s))
                .or_else(|| std::env::var("KIRO_CLI_DB_FILE").ok().map(|s| expand_tilde(&s)))
                .context("KIRO_CLI_DB_FILE is required (use -d or set KIRO_CLI_DB_FILE env var)")?,

            // Timeouts
            streaming_timeout: std::env::var("STREAMING_READ_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            token_refresh_threshold: std::env::var("TOKEN_REFRESH_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),

            first_token_timeout: args.first_token_timeout,

            // HTTP client
            http_max_connections: std::env::var("HTTP_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),

            http_connect_timeout: std::env::var("HTTP_CONNECT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),

            http_request_timeout: args.http_timeout,

            http_max_retries: args.http_retries,

            // Debug
            debug_mode: parse_debug_mode(&args.debug_mode),

            log_level: args.log_level,

            // Converter settings
            tool_description_max_length: std::env::var("TOOL_DESCRIPTION_MAX_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),

            fake_reasoning_enabled: args.fake_reasoning,

            fake_reasoning_max_tokens: args.fake_reasoning_max_tokens,

            fake_reasoning_handling: parse_fake_reasoning_handling(
                &std::env::var("FAKE_REASONING_HANDLING").unwrap_or_default(),
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
