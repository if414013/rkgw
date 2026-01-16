// Authentication module
// Manages token lifecycle and credential loading

mod types;
mod credentials;
mod manager;
mod refresh;

pub use manager::AuthManager;

// Re-export for testing
#[cfg(test)]
pub use credentials::detect_auth_type;
