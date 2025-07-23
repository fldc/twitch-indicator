#![allow(dead_code)]

use anyhow::Result;
use notify_rust::{Notification, Timeout, Urgency};
use std::collections::HashSet;
use tracing::{debug, error};

use crate::api::models::Stream;
use crate::config::NotificationConfig;

pub struct NotificationManager {
    config: NotificationConfig,
    shown_streams: HashSet<String>,
}

impl NotificationManager {
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            config,
            shown_streams: HashSet::new(),
        }
    }

    pub fn update_config(&mut self, config: NotificationConfig) {
        self.config = config;
    }

    pub fn notify_new_streams(&mut self, streams: &[Stream]) -> Result<()> {
        if !self.config.enabled {
            debug!("Notifications disabled, skipping");
            return Ok(());
        }

        let new_streams: Vec<&Stream> = streams
            .iter()
            .filter(|stream| !self.shown_streams.contains(&stream.id))
            .collect();

        if new_streams.is_empty() {
            return Ok(());
        }

        debug!(
            "Showing notifications for {} new streams",
            new_streams.len()
        );

        for stream in new_streams {
            if let Err(e) = self.show_stream_notification(stream) {
                error!(
                    "Failed to show notification for {}: {}",
                    stream.user_name, e
                );
            } else {
                self.shown_streams.insert(stream.id.clone());
            }
        }

        Ok(())
    }

    pub fn update_live_streams(&mut self, current_streams: &[Stream]) {
        let current_ids: HashSet<String> = current_streams.iter().map(|s| s.id.clone()).collect();

        self.shown_streams.retain(|id| current_ids.contains(id));
    }

    fn show_stream_notification(&self, stream: &Stream) -> Result<()> {
        let title = format!("{} is now live!", stream.user_name);
        let mut body = stream.title.clone();

        if self.config.show_game && !stream.game_name.is_empty() {
            body.push_str(&format!("\n\nPlaying: {}", stream.game_name));
        }

        if self.config.show_viewer_count {
            body.push_str(&format!("\nViewers: {}", stream.formatted_viewer_count()));
        }

        let mut notification = Notification::new();
        notification
            .summary(&title)
            .body(&body)
            .icon("twitch")
            .timeout(Timeout::Milliseconds(self.config.timeout_ms))
            .urgency(Urgency::Normal);

        let _handle = notification
            .show()
            .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;

        debug!(
            "Showed notification for stream: {} ({})",
            stream.user_name, stream.id
        );

        Ok(())
    }

    pub fn clear_tracked_streams(&mut self) {
        self.shown_streams.clear();
        debug!("Cleared all tracked streams");
    }

    pub fn tracked_stream_count(&self) -> usize {
        self.shown_streams.len()
    }
}
