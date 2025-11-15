mod server;
mod tls;

use anyhow::{Context, Result};
use tokio::sync::oneshot;
use tracing::info;

use crate::api::models::TokenResponse;
use crate::constants::{OAUTH_SCOPES, REDIRECT_URI, TWITCH_AUTH_URL};

/// Handles the OAuth 2.0 implicit flow for Twitch authentication.
/// Uses a temporary HTTPS server to receive the access token via browser redirect.
pub struct OAuthFlow {
    client_id: String,
}

impl OAuthFlow {
    pub fn new(client_id: String) -> Self {
        Self { client_id }
    }

    /// Initiates the OAuth authentication flow.
    /// Opens the user's browser for authorization and waits for the callback.
    pub async fn authenticate(&mut self) -> Result<TokenResponse> {
        let state = uuid::Uuid::new_v4().to_string();
        let auth_url = self.build_auth_url(&state);

        info!("Opening browser for authorization: {}", auth_url);

        let (sender, receiver) = oneshot::channel();

        server::start_callback_server(state, sender).await?;

        webbrowser::open(&auth_url).context("Failed to open browser")?;

        receiver.await.context("Failed to receive OAuth callback")?
    }

    /// Builds the Twitch authorization URL with required parameters.
    fn build_auth_url(&self, state: &str) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=token&scope={}&state={}&force_verify=true",
            TWITCH_AUTH_URL,
            self.client_id,
            urlencoding::encode(REDIRECT_URI),
            OAUTH_SCOPES.join(" "),
            state
        )
    }
}
