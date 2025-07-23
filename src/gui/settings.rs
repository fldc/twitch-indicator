#![allow(dead_code)]

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::{Config, GeneralConfig, NotificationConfig, TwitchConfig, UiConfig};

pub struct SettingsWindow {
    config: Arc<RwLock<Config>>,
    temp_config: Config,
}

impl SettingsWindow {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let temp_config = {
            let config_guard = config.read().await;
            config_guard.clone()
        };

        Ok(Self {
            config,
            temp_config,
        })
    }

    pub async fn show(&mut self) -> Result<bool> {
        info!("Opening settings dialog");

        self.show_text_interface().await
    }

    async fn show_text_interface(&mut self) -> Result<bool> {
        use std::io::{self, Write};

        loop {
            println!("\n=== Twitch Indicator Settings ===");

            println!("\n--- General Settings ---");
            println!("1. Autostart: {}", self.temp_config.general.autostart);
            println!(
                "2. Minimize to tray: {}",
                self.temp_config.general.minimize_to_tray
            );

            println!("\n--- Notification Settings ---");
            println!(
                "3. Notifications enabled: {}",
                self.temp_config.notifications.enabled
            );
            println!(
                "4. Show game in notifications: {}",
                self.temp_config.notifications.show_game
            );
            println!(
                "5. Show viewer count: {}",
                self.temp_config.notifications.show_viewer_count
            );
            println!(
                "6. Notification timeout (ms): {}",
                self.temp_config.notifications.timeout_ms
            );

            println!("\n--- UI Settings ---");
            println!(
                "7. Show followed channels on top: {}",
                self.temp_config.ui.show_selected_channels_on_top
            );
            println!("8. Dark theme: {}", self.temp_config.ui.dark_theme);

            println!("\n--- Twitch Settings ---");
            println!(
                "9. Refresh interval (minutes): {}",
                self.temp_config.twitch.refresh_interval_minutes
            );

            println!("\n--- Presets ---");
            println!("p1. Apply minimal/performance preset");
            println!("p2. Apply full features preset");
            println!("p3. Apply privacy-focused preset");
            println!("r. Reset to defaults");

            println!("\nEnter setting number to change, or:");
            println!("  s = save and exit");
            println!("  q = quit without saving");
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            match input {
                "s" => {
                    if self.validate().is_ok() {
                        return Ok(true);
                    } else {
                        println!("Invalid settings! Please fix errors before saving.");
                        continue;
                    }
                }
                "q" => return Ok(false),
                "1" => self.temp_config.general.autostart = !self.temp_config.general.autostart,
                "2" => {
                    self.temp_config.general.minimize_to_tray =
                        !self.temp_config.general.minimize_to_tray
                }
                "3" => {
                    self.temp_config.notifications.enabled = !self.temp_config.notifications.enabled
                }
                "4" => {
                    self.temp_config.notifications.show_game =
                        !self.temp_config.notifications.show_game
                }
                "5" => {
                    self.temp_config.notifications.show_viewer_count =
                        !self.temp_config.notifications.show_viewer_count
                }
                "6" => {
                    print!(
                        "Enter new timeout in milliseconds (current: {}): ",
                        self.temp_config.notifications.timeout_ms
                    );
                    io::stdout().flush()?;
                    let mut timeout_input = String::new();
                    io::stdin().read_line(&mut timeout_input)?;
                    if let Ok(timeout) = timeout_input.trim().parse::<u32>() {
                        self.temp_config.notifications.timeout_ms = timeout;
                    } else {
                        println!("Invalid number!");
                    }
                }
                "7" => {
                    self.temp_config.ui.show_selected_channels_on_top =
                        !self.temp_config.ui.show_selected_channels_on_top
                }
                "8" => self.temp_config.ui.dark_theme = !self.temp_config.ui.dark_theme,
                "9" => {
                    print!(
                        "Enter refresh interval in minutes (current: {}): ",
                        self.temp_config.twitch.refresh_interval_minutes
                    );
                    io::stdout().flush()?;
                    let mut interval_input = String::new();
                    io::stdin().read_line(&mut interval_input)?;
                    if let Ok(interval) = interval_input.trim().parse::<u64>() {
                        if interval > 0 && interval <= 60 {
                            self.temp_config.twitch.refresh_interval_minutes = interval;
                        } else {
                            println!("Interval must be between 1 and 60 minutes!");
                        }
                    } else {
                        println!("Invalid number!");
                    }
                }
                "p1" => {
                    self.apply_minimal_preset();
                    println!("Minimal preset applied!");
                }
                "p2" => {
                    self.apply_full_preset();
                    println!("Full features preset applied!");
                }
                "p3" => {
                    self.apply_privacy_preset();
                    println!("Privacy preset applied!");
                }
                "r" => {
                    self.reset_to_defaults();
                    println!("Reset to defaults!");
                }
                _ => {
                    println!("Invalid option! Please try again.");
                }
            }
        }
    }

    pub async fn apply_changes(&self) -> Result<()> {
        info!("Applying settings changes");

        let mut config_guard = self.config.write().await;
        *config_guard = self.temp_config.clone();

        let config_path = Config::get_config_dir()?.join("config.toml");
        config_guard.save(&config_path).await?;

        info!("Settings saved successfully");
        Ok(())
    }

    pub fn reset_to_defaults(&mut self) {
        debug!("Resetting settings to defaults");
        self.temp_config = Config::default();
    }

    pub fn update_general(&mut self, general: GeneralConfig) {
        self.temp_config.general = general;
    }

    pub fn update_notifications(&mut self, notifications: NotificationConfig) {
        self.temp_config.notifications = notifications;
    }

    pub fn update_ui(&mut self, ui: UiConfig) {
        self.temp_config.ui = ui;
    }

    pub fn update_twitch_settings(&mut self, client_id: String, refresh_interval: u64) {
        self.temp_config.twitch.client_id = client_id;
        self.temp_config.twitch.refresh_interval_minutes = refresh_interval;
    }

    pub fn get_general(&self) -> &GeneralConfig {
        &self.temp_config.general
    }

    pub fn get_notifications(&self) -> &NotificationConfig {
        &self.temp_config.notifications
    }

    pub fn get_ui(&self) -> &UiConfig {
        &self.temp_config.ui
    }

    pub fn get_twitch(&self) -> &TwitchConfig {
        &self.temp_config.twitch
    }
}

impl SettingsWindow {
    pub fn validate(&self) -> Result<()> {
        if self.temp_config.twitch.refresh_interval_minutes == 0 {
            return Err(anyhow::anyhow!("Refresh interval must be greater than 0"));
        }

        if self.temp_config.twitch.refresh_interval_minutes > 60 {
            return Err(anyhow::anyhow!(
                "Refresh interval should not exceed 60 minutes"
            ));
        }

        if self.temp_config.notifications.timeout_ms > 30000 {
            return Err(anyhow::anyhow!(
                "Notification timeout should not exceed 30 seconds (30000ms)"
            ));
        }

        if self.temp_config.twitch.client_id.is_empty() {
            return Err(anyhow::anyhow!("Twitch Client ID cannot be empty"));
        }

        Ok(())
    }
}

impl SettingsWindow {
    pub fn apply_minimal_preset(&mut self) {
        self.temp_config.notifications.enabled = false;
        self.temp_config.notifications.show_game = false;
        self.temp_config.notifications.show_viewer_count = false;
        self.temp_config.twitch.refresh_interval_minutes = 5;
        self.temp_config.ui.show_selected_channels_on_top = false;
    }

    pub fn apply_full_preset(&mut self) {
        self.temp_config.notifications.enabled = true;
        self.temp_config.notifications.show_game = true;
        self.temp_config.notifications.show_viewer_count = true;
        self.temp_config.notifications.timeout_ms = 5000;
        self.temp_config.twitch.refresh_interval_minutes = 1;
        self.temp_config.ui.show_selected_channels_on_top = true;
    }

    pub fn apply_privacy_preset(&mut self) {
        self.temp_config.notifications.show_viewer_count = false;
        self.temp_config.general.autostart = false;
        self.temp_config.twitch.refresh_interval_minutes = 3;
    }
}
