/// Application name used for configuration and cache directories.
pub const APP_NAME: &str = "twitch-indicator";

/// Configuration file name.
pub const CONFIG_FILE: &str = "config.toml";

/// Twitch OAuth authorization URL.
pub const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";

/// OAuth scopes required for the application.
pub const OAUTH_SCOPES: &[&str] = &["user:read:follows"];

/// Local redirect URI for OAuth callback.
pub const REDIRECT_URI: &str = "https://localhost:17563";

/// Port for the OAuth callback server.
pub const REDIRECT_PORT: u16 = 17563;

/// Default refresh interval in minutes.
pub const DEFAULT_REFRESH_INTERVAL_MINUTES: u64 = 2;

/// Default notification timeout in milliseconds.
pub const DEFAULT_NOTIFICATION_TIMEOUT_MS: u32 = 5000;
