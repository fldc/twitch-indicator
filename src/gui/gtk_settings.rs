#![allow(dead_code)]

use anyhow::Result;
use gtk::glib::Propagation;
use gtk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::Config;

pub struct GtkSettingsWindow {
    config: Arc<RwLock<Config>>,
    temp_config: Config,
}

impl GtkSettingsWindow {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let temp_config = {
            let config_guard = config.read().await;
            config_guard.clone()
        };

        Ok(GtkSettingsWindow {
            config,
            temp_config,
        })
    }

    pub fn show_sync(&mut self) -> Result<()> {
        info!("Creating GTK settings window");

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Twitch Indicator Settings");
        window.set_default_size(500, 400);
        window.set_position(gtk::WindowPosition::Center);
        window.set_resizable(true);

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        main_box.set_margin_start(20);
        main_box.set_margin_end(20);
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);

        let notebook = gtk::Notebook::new();

        let general_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        general_box.set_margin_start(10);
        general_box.set_margin_end(10);
        general_box.set_margin_top(10);
        general_box.set_margin_bottom(10);

        let interval_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let interval_label = gtk::Label::new(Some("Refresh interval (minutes):"));
        let interval_spin = gtk::SpinButton::with_range(1.0, 60.0, 1.0);
        interval_spin.set_value(self.temp_config.twitch.refresh_interval_minutes as f64);

        interval_box.pack_start(&interval_label, false, false, 0);
        interval_box.pack_start(&interval_spin, false, false, 0);
        general_box.pack_start(&interval_box, false, false, 0);

        let timeout_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let timeout_label = gtk::Label::new(Some("Notification timeout (ms):"));
        let timeout_spin = gtk::SpinButton::with_range(1000.0, 10000.0, 100.0);
        timeout_spin.set_value(self.temp_config.notifications.timeout_ms as f64);

        timeout_box.pack_start(&timeout_label, false, false, 0);
        timeout_box.pack_start(&timeout_spin, false, false, 0);
        general_box.pack_start(&timeout_box, false, false, 0);

        let autostart_check = gtk::CheckButton::with_label("Start with system");
        autostart_check.set_active(self.temp_config.general.autostart);
        general_box.pack_start(&autostart_check, false, false, 0);

        let minimize_check = gtk::CheckButton::with_label("Minimize to tray");
        minimize_check.set_active(self.temp_config.general.minimize_to_tray);
        general_box.pack_start(&minimize_check, false, false, 0);

        let notifications_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        notifications_box.set_margin_start(10);
        notifications_box.set_margin_end(10);
        notifications_box.set_margin_top(10);
        notifications_box.set_margin_bottom(10);

        let notify_enabled = gtk::CheckButton::with_label("Enable notifications");
        notify_enabled.set_active(self.temp_config.notifications.enabled);
        notifications_box.pack_start(&notify_enabled, false, false, 0);

        let show_game_check = gtk::CheckButton::with_label("Show game in notifications");
        show_game_check.set_active(self.temp_config.notifications.show_game);
        notifications_box.pack_start(&show_game_check, false, false, 0);

        let show_viewers_check = gtk::CheckButton::with_label("Show viewer count in notifications");
        show_viewers_check.set_active(self.temp_config.notifications.show_viewer_count);
        notifications_box.pack_start(&show_viewers_check, false, false, 0);

        let ui_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        ui_box.set_margin_start(10);
        ui_box.set_margin_end(10);
        ui_box.set_margin_top(10);
        ui_box.set_margin_bottom(10);

        let top_channels_check = gtk::CheckButton::with_label("Show selected channels on top");
        top_channels_check.set_active(self.temp_config.ui.show_selected_channels_on_top);
        ui_box.pack_start(&top_channels_check, false, false, 0);

        let dark_theme_check = gtk::CheckButton::with_label("Use dark theme");
        dark_theme_check.set_active(self.temp_config.ui.dark_theme);
        ui_box.pack_start(&dark_theme_check, false, false, 0);

        let stream_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        stream_box.set_margin_start(10);
        stream_box.set_margin_end(10);
        stream_box.set_margin_top(10);
        stream_box.set_margin_bottom(10);

        let program_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let program_label = gtk::Label::new(Some("Program:"));
        program_label.set_size_request(120, -1);
        program_label.set_halign(gtk::Align::Start);
        let program_entry = gtk::Entry::new();
        program_entry.set_placeholder_text(Some("Leave empty to use default browser"));
        if let Some(program) = &self.temp_config.stream_open.program {
            program_entry.set_text(program);
        }
        program_box.pack_start(&program_label, false, false, 0);
        program_box.pack_start(&program_entry, true, true, 0);
        stream_box.pack_start(&program_box, false, false, 0);

        let args_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let args_label = gtk::Label::new(Some("Arguments:"));
        args_label.set_size_request(120, -1);
        args_label.set_halign(gtk::Align::Start);
        let args_entry = gtk::Entry::new();
        args_entry.set_placeholder_text(Some(
            "Arguments passed before URL (URL is always added last)",
        ));
        args_entry.set_text(&self.temp_config.stream_open.arguments.join(" "));
        args_box.pack_start(&args_label, false, false, 0);
        args_box.pack_start(&args_entry, true, true, 0);
        stream_box.pack_start(&args_box, false, false, 0);

        let extra_prog_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let extra_prog_label = gtk::Label::new(Some("Extra Command:"));
        extra_prog_label.set_size_request(120, -1);
        extra_prog_label.set_halign(gtk::Align::Start);
        let extra_prog_entry = gtk::Entry::new();
        extra_prog_entry.set_placeholder_text(Some(
            "Optional extra program (e.g., twitch-tui, chatterino)",
        ));
        if let Some(extra_command) = &self.temp_config.stream_open.extra_command {
            extra_prog_entry.set_text(extra_command);
        }
        extra_prog_box.pack_start(&extra_prog_label, false, false, 0);
        extra_prog_box.pack_start(&extra_prog_entry, true, true, 0);
        stream_box.pack_start(&extra_prog_box, false, false, 0);

        let extra_args_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let extra_args_label = gtk::Label::new(Some("Extra Arguments:"));
        extra_args_label.set_size_request(120, -1);
        extra_args_label.set_halign(gtk::Align::Start);
        let extra_args_entry = gtk::Entry::new();
        extra_args_entry.set_placeholder_text(Some("Arguments before channel name"));
        extra_args_entry.set_text(&self.temp_config.stream_open.extra_arguments.join(" "));
        extra_args_box.pack_start(&extra_args_label, false, false, 0);
        extra_args_box.pack_start(&extra_args_entry, true, true, 0);
        stream_box.pack_start(&extra_args_box, false, false, 0);

        let info_label = gtk::Label::new(Some(
            "Configure how streams are opened when clicking on them.\n\
            If no program is specified, the default browser will be used.\n\
            \n\
            Stream Program Examples:\n\
            • Program: 'mpv', Arguments: '' - Opens stream URL directly in MPV\n\
            • Program: 'streamlink', Arguments: 'best' - Opens with 'streamlink best [URL]'\n\
            • Program: 'vlc', Arguments: '--intf dummy' - Opens with 'vlc --intf dummy [URL]'\n\
            \n\
            Extra Command Examples:\n\
            • Extra Command: 'twitch-tui', Arguments: '' - Opens 'twitch-tui channelname'\n\
            • Extra Command: 'chatterino', Arguments: '' - Opens 'chatterino channelname'\n\
            \n\
            The stream URL is always added last to the main program.\n\
            The channel name is always added last to the extra command.",
        ));
        info_label.set_halign(gtk::Align::Start);
        info_label.set_line_wrap(true);
        info_label.set_margin_top(10);
        stream_box.pack_start(&info_label, false, false, 0);

        notebook.append_page(&general_box, Some(&gtk::Label::new(Some("General"))));
        notebook.append_page(
            &notifications_box,
            Some(&gtk::Label::new(Some("Notifications"))),
        );
        notebook.append_page(&ui_box, Some(&gtk::Label::new(Some("Interface"))));
        notebook.append_page(&stream_box, Some(&gtk::Label::new(Some("Stream Opening"))));

        main_box.pack_start(&notebook, true, true, 0);

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        button_box.set_halign(gtk::Align::End);

        let cancel_button = gtk::Button::with_label("Cancel");
        let apply_button = gtk::Button::with_label("Apply");
        let ok_button = gtk::Button::with_label("OK");

        button_box.pack_start(&cancel_button, false, false, 0);
        button_box.pack_start(&apply_button, false, false, 0);
        button_box.pack_start(&ok_button, false, false, 0);

        main_box.pack_start(&button_box, false, false, 0);

        let config_arc = self.config.clone();
        let interval_spin_clone = interval_spin.clone();
        let timeout_spin_clone = timeout_spin.clone();
        let autostart_check_clone = autostart_check.clone();
        let minimize_check_clone = minimize_check.clone();
        let notify_enabled_clone = notify_enabled.clone();
        let show_game_check_clone = show_game_check.clone();
        let show_viewers_check_clone = show_viewers_check.clone();
        let top_channels_check_clone = top_channels_check.clone();
        let dark_theme_check_clone = dark_theme_check.clone();
        let program_entry_clone = program_entry.clone();
        let args_entry_clone = args_entry.clone();
        let extra_prog_entry_clone = extra_prog_entry.clone();
        let extra_args_entry_clone = extra_args_entry.clone();

        let window_clone = window.clone();
        cancel_button.connect_clicked(move |_| {
            window_clone.close();
        });

        let apply_config = config_arc.clone();
        apply_button.connect_clicked(move |_| {
            let config = apply_config.clone();
            let interval = interval_spin_clone.value() as u64;
            let timeout = timeout_spin_clone.value() as u32;
            let autostart = autostart_check_clone.is_active();
            let minimize = minimize_check_clone.is_active();
            let notify_enabled = notify_enabled_clone.is_active();
            let show_game = show_game_check_clone.is_active();
            let show_viewers = show_viewers_check_clone.is_active();
            let top_channels = top_channels_check_clone.is_active();
            let dark_theme = dark_theme_check_clone.is_active();
            let program_text = program_entry_clone.text();
            let args_text = args_entry_clone.text();
            let extra_prog_text = extra_prog_entry_clone.text();
            let extra_args_text = extra_args_entry_clone.text();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(mut config_guard) = config.try_write() {
                        config_guard.twitch.refresh_interval_minutes = interval;
                        config_guard.notifications.timeout_ms = timeout;
                        config_guard.general.autostart = autostart;
                        config_guard.general.minimize_to_tray = minimize;
                        config_guard.notifications.enabled = notify_enabled;
                        config_guard.notifications.show_game = show_game;
                        config_guard.notifications.show_viewer_count = show_viewers;
                        config_guard.ui.show_selected_channels_on_top = top_channels;
                        config_guard.ui.dark_theme = dark_theme;

                        config_guard.stream_open.program = if program_text.is_empty() {
                            None
                        } else {
                            Some(program_text.to_string())
                        };
                        config_guard.stream_open.arguments = if args_text.is_empty() {
                            vec![]
                        } else {
                            args_text
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect()
                        };
                        config_guard.stream_open.extra_command = if extra_prog_text.is_empty() {
                            None
                        } else {
                            Some(extra_prog_text.to_string())
                        };
                        config_guard.stream_open.extra_arguments = if extra_args_text.is_empty() {
                            vec![]
                        } else {
                            extra_args_text
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect()
                        };

                        if let Err(e) = config_guard.save_default().await {
                            eprintln!("Failed to save settings: {e}");
                        } else {
                            println!("Settings applied successfully");
                        }
                    }
                });
            });
        });

        let ok_config = config_arc.clone();
        let window_clone2 = window.clone();
        ok_button.connect_clicked(move |_| {
            let config = ok_config.clone();
            let interval = interval_spin.value() as u64;
            let timeout = timeout_spin.value() as u32;
            let autostart = autostart_check.is_active();
            let minimize = minimize_check.is_active();
            let notify_enabled = notify_enabled.is_active();
            let show_game = show_game_check.is_active();
            let show_viewers = show_viewers_check.is_active();
            let top_channels = top_channels_check.is_active();
            let dark_theme = dark_theme_check.is_active();
            let program_text = program_entry.text();
            let args_text = args_entry.text();
            let extra_prog_text = extra_prog_entry.text();
            let extra_args_text = extra_args_entry.text();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Ok(mut config_guard) = config.try_write() {
                        config_guard.twitch.refresh_interval_minutes = interval;
                        config_guard.notifications.timeout_ms = timeout;
                        config_guard.general.autostart = autostart;
                        config_guard.general.minimize_to_tray = minimize;
                        config_guard.notifications.enabled = notify_enabled;
                        config_guard.notifications.show_game = show_game;
                        config_guard.notifications.show_viewer_count = show_viewers;
                        config_guard.ui.show_selected_channels_on_top = top_channels;
                        config_guard.ui.dark_theme = dark_theme;

                        config_guard.stream_open.program = if program_text.is_empty() {
                            None
                        } else {
                            Some(program_text.to_string())
                        };
                        config_guard.stream_open.arguments = if args_text.is_empty() {
                            vec![]
                        } else {
                            args_text
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect()
                        };
                        config_guard.stream_open.extra_command = if extra_prog_text.is_empty() {
                            None
                        } else {
                            Some(extra_prog_text.to_string())
                        };
                        config_guard.stream_open.extra_arguments = if extra_args_text.is_empty() {
                            vec![]
                        } else {
                            extra_args_text
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect()
                        };

                        if let Err(e) = config_guard.save_default().await {
                            eprintln!("Failed to save settings: {e}");
                        } else {
                            println!("Settings saved and applied");
                        }
                    }
                });
            });

            window_clone2.close();
        });

        window.add(&main_box);
        window.show_all();

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Propagation::Proceed
        });

        gtk::main();

        Ok(())
    }
}
