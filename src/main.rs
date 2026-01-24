use anyhow::Result;
use std::sync::Arc;

mod auth;
mod cache;
mod config;
mod converters;
mod dashboard;
mod error;
mod http_client;
mod metrics;
mod middleware;
mod models;
mod resolver;
mod routes;
mod streaming;
mod thinking_parser;
mod tokenizer;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if interactive setup is needed (no .env and missing required values)
    if config::needs_interactive_setup() {
        let interactive_config = config::run_interactive_setup()?;

        // Set environment variables from interactive config so Config::load() can use them
        std::env::set_var("PROXY_API_KEY", &interactive_config.proxy_api_key);
        std::env::set_var("KIRO_CLI_DB_FILE", &interactive_config.kiro_cli_db_file);
        std::env::set_var("KIRO_REGION", &interactive_config.kiro_region);
        std::env::set_var("SERVER_PORT", &interactive_config.server_port);
    }

    // Load configuration first (for log level)
    let config = config::Config::load()?;
    config.validate()?;

    // Initialize logging with a configured level
    let log_level = config.log_level.to_lowercase();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("🚀 Kiro Gateway starting...");
    tracing::info!(
        "Server configured: {}:{}",
        config.server_host,
        config.server_port
    );
    tracing::debug!("Debug mode: {:?}", config.debug_mode);

    // Initialize authentication manager
    tracing::info!("Initializing authentication...");
    let auth_manager = Arc::new(auth::AuthManager::new(
        config.kiro_cli_db_file.clone(),
        config.token_refresh_threshold,
    )?);

    // Test authentication by getting a token
    match auth_manager.get_access_token().await {
        Ok(token) => {
            tracing::info!(
                "✅ Authentication successful (token: {}...)",
                &token[..20.min(token.len())]
            );
        }
        Err(e) => {
            tracing::error!("❌ Authentication failed: {}", e);
            tracing::warn!(
                "Server will start but API requests will fail without valid credentials"
            );
        }
    }

    // Initialize HTTP client
    let http_client = Arc::new(http_client::KiroHttpClient::new(
        auth_manager.clone(),
        config.http_max_connections,
        config.http_connect_timeout,
        config.http_request_timeout,
        config.http_max_retries,
    )?);
    tracing::info!("✅ HTTP client initialized with connection pooling");

    // Initialize model cache
    tracing::info!("Initializing model cache...");
    let model_cache = cache::ModelCache::new(3600); // 1 hour TTL

    // Load models from Kiro API at startup - fail fast on errors
    tracing::info!("Loading models from Kiro API...");
    let models = match load_models_from_kiro(&http_client, &auth_manager, &config).await {
        Ok(models) => models,
        Err(e) => {
            tracing::error!("❌ Failed to load models from Kiro API: {}", e);
            tracing::error!("");
            tracing::error!("🔧 Troubleshooting steps:");
            tracing::error!("   1. Check your network connection");
            tracing::error!("   2. Verify your Kiro CLI credentials are valid");
            tracing::error!("   3. Try logging out and logging back in:");
            tracing::error!("");
            tracing::error!("      kiro-cli logout");
            tracing::error!("      kiro-cli login");
            tracing::error!("");
            anyhow::bail!("Startup failed: Unable to connect to CodeWhisperer API");
        }
    };

    tracing::info!("📊 Models from Kiro API:");
    for model in &models {
        tracing::info!("{}", serde_json::to_string_pretty(model).unwrap_or_default());
    }

    model_cache.update(models);
    tracing::info!(
        "✅ Loaded {} models from Kiro API",
        model_cache.get_all_model_ids().len()
    );

    // Add hidden models to cache
    add_hidden_models(&model_cache);
    tracing::info!("✅ Added hidden models to cache");

    let resolver = resolver::ModelResolver::new(
        model_cache.clone(),
        std::collections::HashMap::new(),
    );
    tracing::info!("✅ Model resolver initialized");

    let metrics = Arc::new(metrics::MetricsCollector::new());
    tracing::info!("✅ Metrics collector initialized");

    let app_state = routes::AppState {
        proxy_api_key: config.proxy_api_key.clone(),
        model_cache: model_cache.clone(),
        auth_manager: auth_manager.clone(),
        http_client: http_client.clone(),
        resolver,
        config: Arc::new(config.clone()),
        metrics,
    };

    // Build the application with routes and middleware
    let app = build_app(app_state);

    // Bind to configured host and port
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Print startup banner
    print_startup_banner(&config);

    // Start server with graceful shutdown
    tracing::info!("🚀 Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("👋 Server shutdown complete");

    Ok(())
}

/// Load models from Kiro API (no retries - fail fast during startup)
async fn load_models_from_kiro(
    http_client: &http_client::KiroHttpClient,
    auth_manager: &auth::AuthManager,
    _config: &config::Config,
) -> anyhow::Result<Vec<serde_json::Value>> {
    // Get access token
    let access_token = auth_manager.get_access_token().await?;
    let region = auth_manager.get_region().await;

    // Build request to list models - use Q API endpoint, not CodeWhisperer
    // Correct endpoint: https://q.{region}.amazonaws.com/ListAvailableModels
    let url = format!("https://q.{}.amazonaws.com/ListAvailableModels", region);

    // Build request with query parameters
    // Note: profileArn is only needed for Kiro Desktop auth, not AWS SSO OIDC
    let req_builder = http_client
        .client()
        .get(&url)
        .query(&[("origin", "AI_EDITOR")])
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json");

    // Add profileArn only if using Kiro Desktop auth (not AWS SSO)
    // For AWS SSO OIDC (kiro-cli), profileArn is not needed
    // TODO: Detect auth type and conditionally add profileArn

    let req = req_builder.build()?;

    // Execute request WITHOUT retries (fail fast during startup)
    let response = http_client.request_no_retry(req).await?;

    // Parse response
    let body = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;

    // Extract models from response
    if let Some(models) = json.get("models").and_then(|v| v.as_array()) {
        Ok(models.clone())
    } else {
        Ok(vec![])
    }
}

/// Add hidden models to cache
fn add_hidden_models(cache: &cache::ModelCache) {
    // Add commonly used model aliases that may not be in the API response
    let hidden_models = vec![
        (
            "claude-3-5-sonnet-20241022",
            "CLAUDE_3_5_SONNET_20241022_V2_0",
        ),
        (
            "claude-3-5-sonnet-20240620",
            "CLAUDE_3_5_SONNET_20240620_V1_0",
        ),
        (
            "claude-3-5-haiku-20241022",
            "CLAUDE_3_5_HAIKU_20241022_V1_0",
        ),
        ("claude-3-opus-20240229", "CLAUDE_3_OPUS_20240229_V1_0"),
        ("claude-3-sonnet-20240229", "CLAUDE_3_SONNET_20240229_V1_0"),
        ("claude-3-haiku-20240307", "CLAUDE_3_HAIKU_20240307_V1_0"),
        ("claude-sonnet-4", "CLAUDE_SONNET_4_20250514_V1_0"),
        ("claude-sonnet-4-20250514", "CLAUDE_SONNET_4_20250514_V1_0"),
        (
            "anthropic.claude-sonnet-4-v1",
            "CLAUDE_SONNET_4_20250514_V1_0",
        ),
    ];

    for (display_name, internal_id) in hidden_models {
        cache.add_hidden_model(display_name, internal_id);
    }
}

/// Build the application with all routes and middleware
fn build_app(state: routes::AppState) -> axum::Router {
    use axum::Router;

    // Health check routes (no auth required)
    let health_routes = routes::health_routes();

    // OpenAI API routes (with auth)
    let openai_routes = routes::openai_routes(state.clone());

    // Anthropic API routes (with auth)
    let anthropic_routes = routes::anthropic_routes(state.clone());

    // Combine all routes
    Router::new()
        .merge(health_routes)
        .merge(openai_routes)
        .merge(anthropic_routes)
        // Apply middleware stack: CORS → Debug → (Auth is per-route)
        .layer(middleware::cors_layer())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::debug_middleware,
        ))
}

/// Print startup banner
fn print_startup_banner(config: &config::Config) {
    let banner = r#"
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║              🚀 Kiro Gateway - Rust Edition              ║
║                                                           ║
║  OpenAI & Anthropic compatible proxy for Kiro API        ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
"#;

    println!("{}", banner);
    println!("  Version:     {}", env!("CARGO_PKG_VERSION"));
    println!(
        "  Server:      http://{}:{}",
        config.server_host, config.server_port
    );
    println!("  Region:      {}", config.kiro_region);
    println!("  Debug Mode:  {:?}", config.debug_mode);
    println!("  Log Level:   {}", config.log_level);
    println!(
        "  Fake Reasoning: {} (max_tokens: {})",
        if config.fake_reasoning_enabled {
            "enabled"
        } else {
            "disabled"
        },
        config.fake_reasoning_max_tokens
    );
    println!();
}

/// Handle graceful shutdown signal
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal, initiating graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown...");
        },
    }
}
