// Authentication module
// Manages token lifecycle and credential loading

mod credentials;
mod manager;
mod refresh;
mod types;

pub use manager::AuthManager;

// Re-export for testing
#[cfg(test)]
pub use credentials::detect_auth_type;
