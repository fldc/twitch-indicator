// Library interface for testing and potential reuse
// Only expose public API, keep internal modules private

mod api;
pub mod config;
mod constants;
pub mod errors;
mod gui;
mod services;

// Re-export public API types
pub use config::Config;
pub use errors::{AppError, ApiError, ConfigError, OAuthError, StreamError};

// For testing purposes only
#[cfg(test)]
pub use services::StreamService;
