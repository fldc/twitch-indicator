use anyhow::{Context, Result};
use tracing::{error, info};

use crate::config::StreamOpenConfig;

/// Service for handling stream-related operations.
pub struct StreamService;

impl StreamService {
    /// Opens a stream URL using the configured program or default browser.
    /// Can also launch an extra command with the channel name.
    pub fn open_stream(config: &StreamOpenConfig, url: &str) -> Result<()> {
        let channel_name = Self::extract_channel_name(url);

        // Open stream in primary program or browser
        if let Some(program) = &config.program {
            if !program.trim().is_empty() {
                Self::launch_program(program, &config.arguments, url)?;
            } else {
                Self::open_in_browser(url)?;
            }
        } else {
            Self::open_in_browser(url)?;
        }

        // Launch extra command if configured
        if let Some(extra_program) = &config.extra_command {
            if !extra_program.trim().is_empty() && !channel_name.is_empty() {
                if let Err(e) = Self::launch_program(
                    extra_program,
                    &config.extra_arguments,
                    &channel_name,
                ) {
                    error!("Failed to launch extra command {}: {}", extra_program, e);
                }
            }
        }

        Ok(())
    }

    /// Launches a program with arguments and a final URL/channel parameter.
    fn launch_program(program: &str, arguments: &[String], final_arg: &str) -> Result<()> {
        let mut args = arguments.to_vec();
        args.push(final_arg.to_string());

        std::process::Command::new(program)
            .args(&args)
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to launch {} with argument: {}",
                    program, final_arg
                )
            })?;

        info!(
            "Opened stream with {}: {} (args: {:?})",
            program, final_arg, args
        );

        Ok(())
    }

    /// Opens a URL in the default browser.
    fn open_in_browser(url: &str) -> Result<()> {
        webbrowser::open(url)
            .with_context(|| format!("Failed to open URL in default browser: {url}"))?;

        info!("Opened stream in default browser: {}", url);
        Ok(())
    }

    /// Extracts the channel name from a Twitch URL.
    /// Returns an empty string if the channel name cannot be extracted.
    pub fn extract_channel_name(url: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_channel_name() {
        assert_eq!(
            StreamService::extract_channel_name("https://www.twitch.tv/example"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example/videos"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example?param=value"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example#fragment"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://example.com"),
            ""
        );
    }
}
