#![allow(dead_code)]

use anyhow::Result;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[cfg(target_os = "linux")]
use libappindicator::{AppIndicator, AppIndicatorStatus};

#[cfg(target_os = "linux")]
use gtk::prelude::*;

use crate::api::models::Stream;
use crate::config::Config;

pub struct SystemTray {
    #[cfg(target_os = "linux")]
    indicator: AppIndicator,
    config: Arc<RwLock<Config>>,
    streams: Vec<Stream>,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl SystemTray {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let mut indicator = AppIndicator::new(
                "twitch-indicator",
                "network-wireless", // More visible network icon
            );

            let icon_path = std::path::Path::new("assets/twitch-icon.png");
            if icon_path.exists() {
                indicator.set_icon_theme_path("assets");
                indicator.set_icon_full("twitch-icon", "Twitch Indicator");
            } else {
                indicator.set_icon_full("applications-internet", "Twitch Indicator");
            }

            indicator.set_status(AppIndicatorStatus::Active);
            indicator.set_title("Twitch Indicator");

            let _menu = Self::create_initial_menu()?;
            indicator.set_menu(&mut gtk::Menu::new());

            Ok(Self {
                indicator,
                config,
                streams: Vec::new(),
                shutdown_tx: None,
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(anyhow::anyhow!("System tray is only supported on Linux"))
        }
    }

    #[cfg(target_os = "linux")]
    fn create_initial_menu() -> Result<gtk::Menu> {
        let menu = gtk::Menu::new();

        let no_streams_item = gtk::MenuItem::with_label("No live streams");
        no_streams_item.set_sensitive(false);
        menu.append(&no_streams_item);

        let separator = gtk::SeparatorMenuItem::new();
        menu.append(&separator);

        let settings_item = gtk::MenuItem::with_label("Settings");
        menu.append(&settings_item);

        let refresh_item = gtk::MenuItem::with_label("Refresh");
        menu.append(&refresh_item);

        let separator2 = gtk::SeparatorMenuItem::new();
        menu.append(&separator2);

        let quit_item = gtk::MenuItem::with_label("Quit");
        menu.append(&quit_item);

        menu.show_all();
        Ok(menu)
    }

    pub fn update_streams(&mut self, streams: Vec<Stream>) -> Result<()> {
        self.streams = streams;
        self.rebuild_menu()
    }

    #[cfg(target_os = "linux")]
    fn rebuild_menu(&mut self) -> Result<()> {
        let mut menu = gtk::Menu::new();

        if self.streams.is_empty() {
            let no_streams_item = gtk::MenuItem::with_label("No live streams");
            no_streams_item.set_sensitive(false);
            menu.append(&no_streams_item);
        } else {
            let mut sorted_streams = self.streams.clone();
            sorted_streams.sort_by(|a, b| b.viewer_count.cmp(&a.viewer_count));

            for stream in &sorted_streams {
                let label = format!("{} ({})", stream.user_name, stream.formatted_viewer_count());

                let stream_item = gtk::MenuItem::with_label(&label);

                let url = stream.url();
                let config_clone = self.config.clone();
                stream_item.connect_activate(move |_| {
                    let url = url.clone();
                    let config = config_clone.clone();

                    tokio::spawn(async move {
                        match crate::config::Config::load_or_create(None).await {
                            Ok(fresh_config) => {
                                if let Err(e) = fresh_config.open_stream_url(&url) {
                                    error!("Failed to open stream: {e}");
                                }
                            }
                            Err(e) => {
                                error!("Failed to reload config ({}), using cached version", e);
                                let config_guard = config.read().await;
                                if let Err(e) = config_guard.open_stream_url(&url) {
                                    error!("Failed to open stream: {e}");
                                }
                            }
                        }
                    });
                });

                menu.append(&stream_item);
            }
        }

        let separator = gtk::SeparatorMenuItem::new();
        menu.append(&separator);

        let settings_item = gtk::MenuItem::with_label("Settings");
        settings_item.connect_activate(move |_| {
            info!("Settings requested - opening GTK configuration");

            let current_exe =
                std::env::current_exe().expect("Failed to get current executable path");

            let result = std::process::Command::new(&current_exe)
                .arg("--gtk-settings")
                .spawn();

            match result {
                Ok(_child) => {
                    info!("GTK settings process launched successfully");
                }
                Err(e) => {
                    error!("Failed to launch GTK settings: {e}");
                    eprintln!("Failed to launch GTK settings: {e}");
                }
            }
        });
        menu.append(&settings_item);

        let refresh_item = gtk::MenuItem::with_label("Refresh");
        refresh_item.connect_activate(move |_| {
            info!("Manual refresh requested");
        });
        menu.append(&refresh_item);

        let separator2 = gtk::SeparatorMenuItem::new();
        menu.append(&separator2);

        let quit_item = gtk::MenuItem::with_label("Quit");
        let shutdown_sender = self.shutdown_tx.clone();
        quit_item.connect_activate(move |_| {
            info!("Quit requested from tray menu");
            if let Some(sender) = &shutdown_sender {
                let _ = sender.send(true);
            } else {
                std::process::exit(0);
            }
        });
        menu.append(&quit_item);

        menu.show_all();
        self.indicator.set_menu(&mut menu);

        debug!("Updated tray menu with {} streams", self.streams.len());
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn rebuild_menu(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn run<F>(mut self, _menu_handler: F) -> Result<()>
    where
        F: FnMut(String) + Send + 'static,
    {
        info!("Starting system tray");

        #[cfg(target_os = "linux")]
        {
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
            self.shutdown_tx = Some(shutdown_tx);

            self.rebuild_menu()?;

            loop {
                if shutdown_rx.has_changed().unwrap_or(false) {
                    let shutdown = *shutdown_rx.borrow_and_update();
                    if shutdown {
                        info!("Shutdown signal received, exiting tray");
                        return Ok(());
                    }
                }

                while gtk::events_pending() {
                    gtk::main_iteration();
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    pub fn set_tooltip(&mut self, tooltip: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.indicator.set_title(tooltip);
        }
        debug!("Set tooltip: {}", tooltip);
        Ok(())
    }

    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }
}

pub struct SimpleTray {
    config: Arc<RwLock<Config>>,
    streams: Vec<Stream>,
}

impl SimpleTray {
    pub fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        Ok(Self {
            config,
            streams: Vec::new(),
        })
    }

    pub fn update_streams(&mut self, streams: Vec<Stream>) -> Result<()> {
        self.streams = streams;
        info!("Updated streams: {} live", self.streams.len());

        for stream in &self.streams {
            info!(
                "  {} - {} ({})",
                stream.user_name,
                stream.title,
                stream.formatted_viewer_count()
            );
        }
        Ok(())
    }

    pub fn set_tooltip(&mut self, tooltip: &str) -> Result<()> {
        debug!("Tooltip: {}", tooltip);
        Ok(())
    }

    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    pub async fn run<F>(self, mut _menu_handler: F) -> Result<()>
    where
        F: FnMut(String) + Send + 'static,
    {
        info!("Running simple tray (console mode)");

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            debug!("Tray running... {} streams", self.streams.len());
        }
    }
}
