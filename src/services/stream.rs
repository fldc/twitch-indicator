use tracing::{error, info};

use crate::config::StreamOpenConfig;
use crate::errors::{Result, StreamError};

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
            .map_err(|e| StreamError::ProgramLaunchFailed {
                program: program.to_string(),
                source: e,
            })?;

        info!(
            "Opened stream with {}: {} (args: {:?})",
            program, final_arg, args
        );

        Ok(())
    }

    /// Opens a URL in the default browser.
    fn open_in_browser(url: &str) -> Result<()> {
        webbrowser::open(url).map_err(|e| {
            StreamError::BrowserOpenFailed(format!("Failed to open {}: {}", url, e))
        })?;

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
    fn test_extract_channel_name_standard_url() {
        assert_eq!(
            StreamService::extract_channel_name("https://www.twitch.tv/example"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_with_path() {
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example/videos"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example/clips"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_with_query() {
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example?param=value"),
            "example"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example?foo=bar&baz=qux"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_with_fragment() {
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/example#fragment"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_complex_url() {
        assert_eq!(
            StreamService::extract_channel_name("https://www.twitch.tv/example/videos?filter=archives&sort=time#section"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_no_protocol() {
        assert_eq!(
            StreamService::extract_channel_name("twitch.tv/example"),
            "example"
        );
    }

    #[test]
    fn test_extract_channel_name_invalid_domain() {
        assert_eq!(
            StreamService::extract_channel_name("https://example.com"),
            ""
        );
        assert_eq!(
            StreamService::extract_channel_name("https://youtube.com/watch?v=123"),
            ""
        );
    }

    #[test]
    fn test_extract_channel_name_empty_channel() {
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/"),
            ""
        );
    }

    #[test]
    fn test_extract_channel_name_special_characters() {
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/user_name123"),
            "user_name123"
        );
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/user-name"),
            "user-name"
        );
    }

    #[test]
    fn test_extract_channel_name_edge_cases() {
        // Empty string
        assert_eq!(StreamService::extract_channel_name(""), "");

        // Just domain
        assert_eq!(StreamService::extract_channel_name("twitch.tv"), "");

        // Case sensitivity preserved
        assert_eq!(
            StreamService::extract_channel_name("https://twitch.tv/CoolStreamer123"),
            "CoolStreamer123"
        );
    }
}
