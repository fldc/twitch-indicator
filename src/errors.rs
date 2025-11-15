use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("OAuth error: {0}")]
    OAuth(#[from] OAuthError),

    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),

    #[error("GUI error: {0}")]
    Gui(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load configuration from {path}: {source}")]
    LoadFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse configuration: {0}")]
    ParseFailed(#[from] toml::de::Error),

    #[error("Failed to save configuration: {0}")]
    SaveFailed(std::io::Error),

    #[error("Configuration directory not found")]
    DirectoryNotFound,

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// API-related errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid response from API: {0}")]
    InvalidResponse(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("API returned error status {status}: {message}")]
    ApiErrorResponse { status: u16, message: String },

    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// OAuth-related errors
#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("Failed to generate TLS certificate: {0}")]
    TlsCertificateFailed(String),

    #[error("Failed to start OAuth server: {0}")]
    ServerStartFailed(String),

    #[error("Failed to bind to port {port}: {source}")]
    PortBindFailed { port: u16, source: std::io::Error },

    #[error("OAuth callback failed: {0}")]
    CallbackFailed(String),

    #[error("Failed to open browser: {0}")]
    BrowserOpenFailed(String),

    #[error("State mismatch: expected {expected}, got {actual}")]
    StateMismatch { expected: String, actual: String },

    #[error("Invalid token response: {0}")]
    InvalidTokenResponse(String),

    #[error("Authorization denied by user")]
    AuthorizationDenied,
}

/// Stream-related errors
#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Failed to launch program {program}: {source}")]
    ProgramLaunchFailed {
        program: String,
        source: std::io::Error,
    },

    #[error("Failed to open stream in browser: {0}")]
    BrowserOpenFailed(String),

    #[error("Invalid stream URL: {0}")]
    InvalidUrl(String),

    #[error("Channel name extraction failed for URL: {0}")]
    ChannelExtractionFailed(String),
}

/// Type alias for Results using AppError
pub type Result<T> = std::result::Result<T, AppError>;

// Convenience conversions
impl From<String> for OAuthError {
    fn from(s: String) -> Self {
        OAuthError::CallbackFailed(s)
    }
}

impl From<&str> for OAuthError {
    fn from(s: &str) -> Self {
        OAuthError::CallbackFailed(s.to_string())
    }
}
