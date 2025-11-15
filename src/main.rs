mod api;
mod config;
mod constants;
mod errors;
mod gui;
mod services;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, level_filters::LevelFilter};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::gui::indicator::TwitchIndicator;

#[cfg(target_os = "linux")]
const GTK_DARK_THEME_PROPERTY: &str = "gtk-application-prefer-dark-theme";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    debug: bool,

    #[arg(short, long)]
    config: Option<String>,

    #[arg(long, hide = true)]
    gtk_settings: bool,

    #[arg(long)]
    export_settings: Option<String>,

    #[arg(long)]
    import_settings: Option<String>,
}

/// Apply GTK dark theme preference.
/// Returns true if the theme was applied, false if GTK is not available.
#[cfg(target_os = "linux")]
fn apply_gtk_theme(dark_theme: bool) -> bool {
    use gtk::prelude::*;

    if !gtk::is_initialized() {
        warn!("Cannot set GTK theme: GTK not initialized");
        return false;
    }

    if let Some(settings) = gtk::Settings::default() {
        settings.set_property(GTK_DARK_THEME_PROPERTY, dark_theme);
        info!("Applied GTK theme preference: dark_theme={}", dark_theme);
        true
    } else {
        warn!("Cannot set GTK theme: GTK Settings not available");
        false
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    #[cfg(target_os = "linux")]
    {
        use gtk;
        if gtk::init().is_err() {
            eprintln!("Warning: Failed to initialize GTK. System tray may not work.");
        }
    }

    let level = if args.debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true)
                .with_filter(level),
        )
        .init();

    info!("Starting Twitch Indicator v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration once
    let config = Config::load_or_create(args.config).await?;

    // Apply dark theme preference for GTK interface
    #[cfg(target_os = "linux")]
    apply_gtk_theme(config.ui.dark_theme);

    if args.gtk_settings {
        let config_arc = Arc::new(RwLock::new(config));
        let mut gtk_settings = crate::gui::gtk_settings::GtkSettingsWindow::new(config_arc).await?;
        gtk_settings.show_sync()?;

        return Ok(());
    }

    let config = Arc::new(RwLock::new(config));

    if let Some(export_path) = args.export_settings {
        let indicator = TwitchIndicator::new(config).await?;
        indicator.export_settings(&export_path).await?;
        println!("Settings exported to: {export_path}");
        return Ok(());
    }

    if let Some(import_path) = args.import_settings {
        let indicator = TwitchIndicator::new(config).await?;
        indicator.import_settings(&import_path).await?;
        println!("Settings imported from: {import_path}");
        return Ok(());
    }

    let indicator = TwitchIndicator::new(config).await?;
    indicator.run().await?;

    Ok(())
}
