#![allow(dead_code)]

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

use crate::api::models::*;
use crate::api::oauth::OAuthFlow;
use crate::config::Config;

const TWITCH_API_BASE: &str = "https://api.twitch.tv/helix";
const TWITCH_VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";

pub struct TwitchClient {
    client: Client,
    client_id: String,
    access_token: Option<String>,
    config: Arc<RwLock<Config>>,
}

impl TwitchClient {
    pub fn new(client_id: String, config: Arc<RwLock<Config>>) -> Self {
        Self {
            client: Client::new(),
            client_id,
            access_token: None,
            config,
        }
    }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    pub async fn load_token_from_config(&mut self) -> Result<()> {
        let config = self.config.read().await;
        if let Some(ref token) = config.twitch.access_token {
            self.access_token = Some(token.clone());
            debug!("Loaded access token from config");
        }
        Ok(())
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        let mut oauth_flow = OAuthFlow::new(self.client_id.clone());
        let token_response = oauth_flow.authenticate().await?;

        self.access_token = Some(token_response.access_token.clone());

        {
            let mut config = self.config.write().await;
            config.twitch.access_token = Some(token_response.access_token);
            config
                .save_default()
                .await
                .context("Failed to save token to config")?;
        }

        Ok(())
    }

    pub async fn validate_token(&self) -> Result<TokenValidation> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow!("No access token available"))?;

        let response = self
            .client
            .get(TWITCH_VALIDATE_URL)
            .header("Authorization", format!("OAuth {token}"))
            .send()
            .await
            .context("Failed to validate token")?;

        if !response.status().is_success() {
            return Err(anyhow!("Token validation failed: {}", response.status()));
        }

        let validation: TokenValidation = response
            .json()
            .await
            .context("Failed to parse token validation response")?;

        debug!("Token validated for user: {}", validation.login);
        Ok(validation)
    }

    pub async fn get_user(&self) -> Result<User> {
        let response = self
            .make_api_request("users", &[])
            .await
            .context("Failed to get user info")?;

        let users: TwitchResponse<User> = response
            .json()
            .await
            .context("Failed to parse user response")?;

        users
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No user data returned"))
    }

    pub async fn get_followed_channels(&self, user_id: &str) -> Result<Vec<FollowedChannel>> {
        let mut all_channels = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut params = vec![("user_id", user_id), ("first", "100")];

            if let Some(ref cursor_val) = cursor {
                params.push(("after", cursor_val));
            }

            let response = self
                .make_api_request("channels/followed", &params)
                .await
                .context("Failed to get followed channels")?;

            let channels_response: TwitchResponse<FollowedChannel> = response
                .json()
                .await
                .context("Failed to parse followed channels response")?;

            all_channels.extend(channels_response.data);

            cursor = channels_response.pagination.and_then(|p| p.cursor);

            if cursor.is_none() {
                break;
            }
        }

        debug!("Retrieved {} followed channels", all_channels.len());
        Ok(all_channels)
    }

    pub async fn get_followed_streams(&self, user_id: &str) -> Result<Vec<Stream>> {
        let params = vec![("user_id", user_id), ("first", "100")];

        let response = self
            .make_api_request("streams/followed", &params)
            .await
            .context("Failed to get followed streams")?;

        let streams_response: TwitchResponse<Stream> = response
            .json()
            .await
            .context("Failed to parse followed streams response")?;

        debug!("Retrieved {} live streams", streams_response.data.len());
        Ok(streams_response.data)
    }

    pub async fn get_streams_by_user_ids(&self, user_ids: &[String]) -> Result<Vec<Stream>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut params = vec![("first", "100")];
        for user_id in user_ids {
            params.push(("user_id", user_id));
        }

        let response = self
            .make_api_request("streams", &params)
            .await
            .context("Failed to get streams by user IDs")?;

        let streams_response: TwitchResponse<Stream> = response
            .json()
            .await
            .context("Failed to parse streams response")?;

        debug!(
            "Retrieved {} streams by user IDs",
            streams_response.data.len()
        );
        Ok(streams_response.data)
    }

    pub async fn get_users_by_ids(&self, user_ids: &[String]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut params = Vec::new();
        for user_id in user_ids {
            params.push(("id", user_id.as_str()));
        }

        let response = self
            .make_api_request("users", &params)
            .await
            .context("Failed to get users by IDs")?;

        let users_response: TwitchResponse<User> = response
            .json()
            .await
            .context("Failed to parse users response")?;

        debug!("Retrieved {} users by IDs", users_response.data.len());
        Ok(users_response.data)
    }

    pub async fn download_profile_image(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to download profile image")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to download image: {}", response.status()));
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read image bytes")?;

        Ok(bytes.to_vec())
    }

    async fn make_api_request(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow!("No access token available for API request"))?;

        let mut url = format!("{TWITCH_API_BASE}/{endpoint}");

        if !params.is_empty() {
            url.push('?');
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&query_string);
        }

        debug!("Making API request: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Client-ID", &self.client_id)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context("Failed to make API request")?;

        match response.status() {
            StatusCode::OK => Ok(response),
            StatusCode::UNAUTHORIZED => {
                error!("API request failed: Unauthorized (401)");
                Err(anyhow!("Authentication failed - token may be expired"))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("API request failed: Rate limit exceeded (429)");
                Err(anyhow!("Rate limit exceeded"))
            }
            status => {
                error!("API request failed with status: {}", status);
                let error_text = response.text().await.unwrap_or_default();
                Err(anyhow!("API request failed ({}): {}", status, error_text))
            }
        }
    }
}
