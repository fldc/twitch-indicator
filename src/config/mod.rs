#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, error, info};

const APP_NAME: &str = "twitch-indicator";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub twitch: TwitchConfig,
    pub notifications: NotificationConfig,
    pub ui: UiConfig,
    pub general: GeneralConfig,
    pub stream_open: StreamOpenConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub refresh_interval_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub show_game: bool,
    pub show_viewer_count: bool,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_selected_channels_on_top: bool,
    pub dark_theme: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub autostart: bool,
    pub minimize_to_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOpenConfig {
    pub program: Option<String>,
    pub arguments: Vec<String>,
    pub extra_command: Option<String>,
    pub extra_arguments: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            twitch: TwitchConfig {
                client_id: "pdnu3rmmjndvi58vd5f19l5rxqvu6c".to_string(),
                redirect_uri: "https://localhost:17563".to_string(),
                access_token: None,
                refresh_token: None,
                refresh_interval_minutes: 2,
            },
            notifications: NotificationConfig {
                enabled: true,
                show_game: true,
                show_viewer_count: true,
                timeout_ms: 5000,
            },
            ui: UiConfig {
                show_selected_channels_on_top: true,
                dark_theme: true,
            },
            general: GeneralConfig {
                autostart: false,
                minimize_to_tray: true,
            },
            stream_open: StreamOpenConfig {
                program: None,
                arguments: vec![],
                extra_command: None,
                extra_arguments: vec![],
            },
        }
    }
}

impl Config {
    pub async fn load_or_create(config_path: Option<String>) -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        let config_file = match config_path {
            Some(path) => PathBuf::from(path),
            None => config_dir.join(CONFIG_FILE),
        };

        if config_file.exists() {
            debug!("Loading config from: {:?}", config_file);
            let content = fs::read_to_string(&config_file)
                .await
                .with_context(|| format!("Failed to read config file: {config_file:?}"))?;

            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {config_file:?}"))?;

            info!("Configuration loaded successfully");
            Ok(config)
        } else {
            info!("Config file not found, creating default configuration");
            let config = Config::default();
            config.save(&config_file).await?;
            Ok(config)
        }
    }

    pub async fn save(&self, config_file: &PathBuf) -> Result<()> {
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create config directory: {parent:?}"))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(config_file, content)
            .await
            .with_context(|| format!("Failed to write config file: {config_file:?}"))?;

        debug!("Configuration saved to: {:?}", config_file);
        Ok(())
    }

    pub async fn save_default(&self) -> Result<()> {
        let config_dir = Self::get_config_dir()?;
        let config_file = config_dir.join(CONFIG_FILE);
        self.save(&config_file).await
    }

    pub fn get_config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join(APP_NAME))
            .context("Failed to get config directory")
    }

    pub fn get_cache_dir() -> Result<PathBuf> {
        dirs::cache_dir()
            .map(|dir| dir.join(APP_NAME))
            .context("Failed to get cache directory")
    }

    pub fn update_tokens(&mut self, access_token: String, refresh_token: Option<String>) {
        self.twitch.access_token = Some(access_token);
        if let Some(refresh_token) = refresh_token {
            self.twitch.refresh_token = Some(refresh_token);
        }
    }

    pub fn clear_tokens(&mut self) {
        self.twitch.access_token = None;
        self.twitch.refresh_token = None;
    }

    pub fn is_authenticated(&self) -> bool {
        self.twitch.access_token.is_some()
    }

    pub fn open_stream_url(&self, url: &str) -> Result<()> {
        let channel_name = Self::extract_channel_name(url);

        if let Some(program) = &self.stream_open.program {
            if !program.trim().is_empty() {
                let mut args = self.stream_open.arguments.clone();
                args.push(url.to_string());

                std::process::Command::new(program)
                    .args(&args)
                    .spawn()
                    .with_context(|| format!("Failed to launch {program} with URL: {url}"))?;

                info!("Opened stream with {}: {} (args: {:?})", program, url, args);
            } else {
                webbrowser::open(url)
                    .with_context(|| format!("Failed to open URL in default browser: {url}"))?;

                info!("Opened stream in default browser: {}", url);
            }
        } else {
            webbrowser::open(url)
                .with_context(|| format!("Failed to open URL in default browser: {url}"))?;

            info!("Opened stream in default browser: {}", url);
        }

        if let Some(extra_program) = &self.stream_open.extra_command {
            if !extra_program.trim().is_empty() && !channel_name.is_empty() {
                let mut extra_args = self.stream_open.extra_arguments.clone();
                extra_args.push(channel_name.clone());

                match std::process::Command::new(extra_program)
                    .args(&extra_args)
                    .spawn()
                {
                    Ok(_) => {
                        info!(
                            "Started extra command {}: {} (args: {:?})",
                            extra_program, channel_name, extra_args
                        );
                    }
                    Err(e) => {
                        error!("Failed to launch extra command {}: {}", extra_program, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_channel_name(url: &str) -> String {
        if let Some(pos) = url.find("twitch.tv/") {
            let after_domain = &url[pos + 10..];
            let end_pos = after_domain
                .find(&['/', '?', '#'][..])
                .unwrap_or(after_domain.len());
            let channel = &after_domain[..end_pos];
            if !channel.is_empty() {
                return channel.to_string();
            }
        }
        String::new()
    }
}
