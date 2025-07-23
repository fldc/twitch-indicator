use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::api::{Stream, TwitchClient, User};
use crate::config::Config;
use crate::gui::notifications::NotificationManager;
use crate::gui::settings::SettingsWindow;
use crate::gui::tray::SystemTray;

pub struct TwitchIndicator {
    config: Arc<RwLock<Config>>,
    twitch_client: TwitchClient,
    notification_manager: NotificationManager,
    current_streams: Vec<Stream>,
    authenticated_user: Option<User>,
}

impl TwitchIndicator {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let config_read = config.read().await;

        let mut twitch_client =
            TwitchClient::new(config_read.twitch.client_id.clone(), config.clone());

        drop(config_read);
        twitch_client.load_token_from_config().await?;

        let config_read = config.read().await;
        let notification_manager = NotificationManager::new(config_read.notifications.clone());

        drop(config_read);

        Ok(Self {
            config,
            twitch_client,
            notification_manager,
            current_streams: Vec::new(),
            authenticated_user: None,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        info!("Starting Twitch Indicator application");

        if !self.is_authenticated().await {
            info!("User not authenticated, starting authentication flow");
            self.authenticate().await?;
        } else {
            info!("User already authenticated, validating token");
            if let Err(e) = self.validate_and_refresh_token().await {
                warn!("Token validation failed: {}, re-authenticating", e);
                self.authenticate().await?;
            }
        }

        self.authenticated_user = Some(
            self.twitch_client
                .get_user()
                .await
                .context("Failed to get authenticated user info")?,
        );

        if let Some(ref user) = self.authenticated_user {
            info!("Authenticated as: {} ({})", user.display_name, user.login);
        }

        let tray = SystemTray::new(self.config.clone()).context("Failed to create system tray")?;

        self.run_with_tray(tray).await
    }

    async fn run_with_tray(mut self, mut tray: SystemTray) -> Result<()> {
        if let Err(e) = self.update_streams().await {
            error!("Initial stream update failed: {}", e);
        }

        tray.update_streams(self.current_streams.clone())?;

        let tooltip = self.create_tooltip();
        tray.set_tooltip(&tooltip)?;

        let config_for_menu = self.config.clone();

        let update_handle = tokio::spawn(async move {
            self.periodic_update_loop().await;
        });

        let menu_handler = move |action: String| match action.as_str() {
            "settings" => {
                info!("Settings requested - opening GTK configuration");

                let config = config_for_menu.clone();
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Ok(mut gtk_settings) =
                            crate::gui::gtk_settings::GtkSettingsWindow::new(config).await
                        {
                            if let Err(e) = gtk_settings.show_sync() {
                                eprintln!("Failed to show GTK settings: {e}");
                            }
                        }
                    });
                });
            }
            "refresh" => {
                info!("Manual refresh requested");
            }
            _ => {
                debug!("Unknown menu action: {}", action);
            }
        };

        let tray_result = tray.run(menu_handler).await;

        update_handle.abort();

        tray_result
    }

    async fn periodic_update_loop(&mut self) {
        let config_read = self.config.read().await;
        let refresh_interval =
            Duration::from_secs(config_read.twitch.refresh_interval_minutes * 60);
        drop(config_read);

        let mut interval_timer = interval(refresh_interval);

        loop {
            interval_timer.tick().await;

            if let Err(e) = self.update_streams().await {
                error!("Failed to update streams: {}", e);

                let error_msg = e.to_string();
                if error_msg.contains("Authentication failed")
                    || error_msg.contains("token may be expired")
                    || error_msg.contains("Unauthorized")
                {
                    warn!("Authentication error detected, attempting re-authentication");
                    if let Err(auth_err) = self.authenticate().await {
                        error!("Re-authentication failed: {}", auth_err);
                    } else if let Ok(user_info) = self.twitch_client.get_user().await {
                        self.authenticated_user = Some(user_info);
                        info!("Re-authentication completed successfully");
                    }
                }
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn update_streams(&mut self) -> Result<()> {
        debug!("Updating streams");

        let user = self
            .authenticated_user
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No authenticated user"))?;

        let new_streams = self
            .twitch_client
            .get_followed_streams(&user.id)
            .await
            .context("Failed to get followed streams")?;

        debug!("Retrieved {} live streams", new_streams.len());

        self.notification_manager.notify_new_streams(&new_streams)?;

        self.notification_manager.update_live_streams(&new_streams);

        self.current_streams = new_streams;

        info!(
            "Stream update completed: {} live streams",
            self.current_streams.len()
        );
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        let config = self.config.read().await;
        config.is_authenticated()
    }

    async fn authenticate(&mut self) -> Result<()> {
        info!("Starting Twitch authentication");

        self.twitch_client
            .authenticate()
            .await
            .context("Authentication failed")?;

        info!("Authentication completed successfully");
        Ok(())
    }

    async fn validate_and_refresh_token(&mut self) -> Result<()> {
        match self.twitch_client.validate_token().await {
            Ok(validation) => {
                debug!("Token valid for user: {}", validation.login);
                Ok(())
            }
            Err(e) => {
                info!("Token validation failed, re-authentication required");
                Err(e)
            }
        }
    }

    fn create_tooltip(&self) -> String {
        if let Some(ref user) = self.authenticated_user {
            format!(
                "Twitch Indicator - {} ({} live streams)",
                user.display_name,
                self.current_streams.len()
            )
        } else {
            "Twitch Indicator - Not authenticated".to_string()
        }
    }

    pub async fn export_settings(&self, path: &str) -> Result<()> {
        info!("Exporting settings to: {}", path);

        let config_read = self.config.read().await;
        let config_toml =
            toml::to_string_pretty(&*config_read).context("Failed to serialize configuration")?;

        tokio::fs::write(path, config_toml)
            .await
            .with_context(|| format!("Failed to write settings to {path}"))?;

        info!("Settings exported successfully");
        Ok(())
    }

    pub async fn import_settings(&self, path: &str) -> Result<()> {
        info!("Importing settings from: {}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read settings from {path}"))?;

        let new_config: Config =
            toml::from_str(&content).context("Failed to parse imported settings")?;

        let settings_window =
            SettingsWindow::new(Arc::new(RwLock::new(new_config.clone()))).await?;
        settings_window
            .validate()
            .context("Imported settings are invalid")?;

        let mut config_write = self.config.write().await;
        *config_write = new_config;

        let config_path = Config::get_config_dir()?.join("config.toml");
        config_write.save(&config_path).await?;

        info!("Settings imported and applied successfully");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_current_streams(&self) -> &[Stream] {
        &self.current_streams
    }

    #[allow(dead_code)]
    pub fn get_authenticated_user(&self) -> Option<&User> {
        self.authenticated_user.as_ref()
    }
}
