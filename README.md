# Twitch Indicator

A Linux system tray application that monitors your followed Twitch streams and provides desktop notifications when streamers go live written in Rust.

## Features

- **System Tray Integration**: Lightweight tray icon showing live stream count
- **Desktop Notifications**: Get notified when followed streamers go live
- **Stream Management**: Click streams in the tray menu to open them directly
- **OAuth Authentication**: Secure Twitch authentication flow
- **Configurable Settings**: Customize notifications, refresh intervals, and UI preferences
- **GTK Settings Window**: Easy-to-use configuration interface

## Installation

### Prerequisites

- Linux system with GTK3 support
- Rust 1.70+ (for building from source)
- System tray support (most desktop environments)

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd twitch-indicator

# Build the application
cargo build --release

# Run the application
./target/release/twitch-indicator
```

### Dependencies

The application requires the following system libraries:

- `libgtk-3-dev`
- `libappindicator3-dev`
- `libssl-dev`
- `pkg-config`

On Ubuntu/Debian:

```bash
sudo apt install libgtk-3-dev libappindicator3-dev libssl-dev pkg-config
```

## Configuration

### First Run

1. Launch the application
2. Complete the OAuth authentication flow
3. Configure your preferences through the settings menu

### Settings

Access settings through the tray menu or run with `--gtk-settings`:

- **General**: Autostart, minimize to tray behavior
- **Notifications**: Enable/disable notifications, timeout settings, display options
- **UI**: Theme preferences, channel sorting
- **Twitch**: Refresh intervals, authentication management

### Configuration File

Settings are stored in `~/.config/twitch-indicator/config.toml`:

```toml
[general]
autostart = false
minimize_to_tray = true

[notifications]
enabled = true
show_game = true
show_viewer_count = true
timeout_ms = 5000

[ui]
show_selected_channels_on_top = true
dark_theme = false

[twitch]
client_id = "your-client-id"
refresh_interval_minutes = 2
```

## Usage

### Basic Usage

1. **Start the application**: Run `twitch-indicator`
2. **Authenticate**: Follow the OAuth flow on first run
3. **Monitor streams**: The tray icon shows live stream count
4. **View streams**: Right-click the tray icon to see live streams
5. **Open streams**: Click on a stream to open it in your default browser
6. **Settings**: Access configuration through the tray menu

### Command Line Options

```bash
# Show help
twitch-indicator --help

# Open GTK settings window
twitch-indicator --gtk-settings

# Run with debug logging
RUST_LOG=debug twitch-indicator
```

### Tray Menu

- **Live Streams**: List of currently live followed channels
- **Settings**: Open configuration window
- **Refresh**: Manually refresh stream status
- **Quit**: Exit the application

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run with logging
RUST_LOG=info cargo run

# Check code
cargo check
```

### Key Dependencies

Key dependencies include:

- `tokio` - Async runtime
- `reqwest` - HTTP client for Twitch API
- `gtk` - GUI toolkit
- `libappindicator` - System tray support
- `serde` - Serialization
- `anyhow` - Error handling
- `tracing` - Logging

## License

This project is licensed under the MIT License - see the LICENSE file for details.

