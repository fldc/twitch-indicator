# Architecture Documentation

## Overview

Twitch Indicator is a Rust application that monitors Twitch streams and provides desktop notifications for followed channels going live. This document describes the architectural decisions and code organization.

## Project Structure

```
twitch-indicator/
├── src/
│   ├── api/                    # Twitch API integration
│   │   ├── oauth/             # OAuth 2.0 authentication
│   │   │   ├── mod.rs        # OAuth flow coordination
│   │   │   ├── tls.rs        # TLS certificate generation
│   │   │   └── server.rs     # HTTPS callback server
│   │   ├── client.rs         # Twitch API client
│   │   ├── models.rs         # API data models
│   │   └── mod.rs
│   ├── config/                # Configuration management
│   │   └── mod.rs            # TOML-based config with validation
│   ├── services/              # Business logic layer
│   │   ├── stream.rs         # Stream operations service
│   │   └── mod.rs
│   ├── gui/                   # User interface
│   │   ├── indicator.rs      # Main application coordinator
│   │   ├── tray.rs           # System tray (Tray trait + impls)
│   │   ├── gtk_settings.rs   # GTK settings window
│   │   ├── settings.rs       # Text-based settings
│   │   ├── notifications.rs  # Desktop notifications
│   │   └── mod.rs
│   ├── constants.rs           # Application constants
│   ├── errors.rs              # Custom error types
│   ├── lib.rs                 # Library interface (for testing)
│   └── main.rs                # Application entry point
├── Cargo.toml
└── ARCHITECTURE.md (this file)
```

## Architectural Layers

### 1. API Layer (`src/api/`)

**Purpose**: Handles all external communication with the Twitch API.

#### OAuth Module (`api/oauth/`)
- **Separation of Concerns**: Split into three focused modules
  - `mod.rs`: Orchestrates OAuth flow, builds authorization URLs
  - `tls.rs`: Generates self-signed TLS certificates
  - `server.rs`: Manages HTTPS callback server and request handling

**Why HTTPS?** The OAuth implicit flow delivers tokens via URL fragments (e.g., `#access_token=...`), which are only accessible to JavaScript in the browser. We serve an HTML page with JavaScript that extracts the token and POSTs it back to our local server.

#### API Client (`api/client.rs`)
- Manages HTTP requests to Twitch API
- Handles token validation and refresh
- Implements retry logic and error handling
- Uses `reqwest` for async HTTP operations

#### Data Models (`api/models.rs`)
- DTOs for API responses: `User`, `Stream`, `Game`, `FollowedChannel`
- Helper methods for formatting (viewer counts, URLs, thumbnails)
- Serde serialization/deserialization

### 2. Services Layer (`src/services/`)

**Purpose**: Encapsulates business logic separate from infrastructure concerns.

#### StreamService (`services/stream.rs`)
- **Extracted from Config** (following Single Responsibility Principle)
- Handles stream URL opening with configured programs
- Manages extra commands (e.g., opening chat in separate program)
- Channel name extraction and validation
- **Testable**: Includes comprehensive unit tests (13 test cases)

**Design Pattern**: Service layer pattern - business logic independent of UI/API

### 3. Configuration Layer (`src/config/`)

- **Pure Data**: Configuration is now purely data, no business logic
- TOML-based persistence with validation
- Nested structures: `TwitchConfig`, `NotificationConfig`, `UiConfig`, etc.
- Delegates operations to services (e.g., `open_stream_url()` → `StreamService`)

### 4. GUI Layer (`src/gui/`)

#### Tray System (`gui/tray.rs`)
- **Trait-based Design**: `Tray` trait defines common interface
  ```rust
  pub trait Tray {
      fn update_streams(&mut self, streams: Vec<Stream>) -> Result<()>;
      fn set_tooltip(&mut self, tooltip: &str) -> Result<()>;
      fn stream_count(&self) -> usize;
  }
  ```
- **Implementations**:
  - `SystemTray`: Full libappindicator integration (Linux)
  - `SimpleTray`: Console-only fallback

**Benefits**: Polymorphism, easy to add new implementations, testable

#### GTK Settings (`gui/gtk_settings.rs`)
- **DRY Principle Applied**: Eliminated 100+ lines of duplication
- Shared `save_settings_from_ui()` method for Apply/OK buttons
- **Async Integration**: Uses `glib::MainContext::spawn_local()` instead of spawning new tokio runtime
- Better integration with GTK event loop

#### Notifications (`gui/notifications.rs`)
- Tracks shown streams to avoid duplicates
- Configurable content and timeout
- Auto-cleanup of offline streams

### 5. Error Handling (`src/errors.rs`)

**Type-safe Errors**: Custom error types using `thiserror`

```rust
pub enum AppError {
    Config(ConfigError),
    Api(ApiError),
    OAuth(OAuthError),
    Stream(StreamError),
    // ...
}
```

**Benefits**:
- More informative error messages
- Better error context (e.g., `StreamError::ProgramLaunchFailed { program, source }`)
- Type-safe pattern matching
- Easier debugging

### 6. Constants (`src/constants.rs`)

**Centralized Configuration**:
- Application name, file paths
- OAuth URLs, scopes, redirect URIs
- Default values for configuration

**Single Source of Truth**: One place to update all magic values

## Design Principles Applied

### SOLID Principles

1. **Single Responsibility**: Each module has one clear purpose
   - OAuth flow vs TLS vs server handling (separate modules)
   - StreamService handles stream operations only
   - Config is pure data

2. **Open/Closed**: Tray trait allows extension without modification
   - Can add new tray implementations without changing existing code

3. **Liskov Substitution**: Any `Tray` implementation can be used interchangeably

4. **Interface Segregation**: Traits are focused (e.g., `Tray` has only essential methods)

5. **Dependency Inversion**: Services depend on abstractions (traits) not concretions

### DRY (Don't Repeat Yourself)

- Eliminated duplicate settings save logic (gtk_settings.rs)
- Centralized constants
- Removed redundant OAuth implementations
- Shared code through services layer

### Clean Architecture

Clear separation of layers:
```
API Layer → Services Layer → GUI Layer
     ↓           ↓              ↓
  Models     Business Logic   UI Logic
```

## Threading Model

### GTK Integration
- Main thread runs GTK event loop
- Async operations use `glib::MainContext` for proper integration
- **Avoid**: Creating new tokio runtimes in GTK callbacks (causes resource leaks)

### Tokio Runtime
- Single tokio runtime created at application start
- `tokio::main` with `current_thread` flavor
- Async operations for I/O-bound tasks (API calls, file operations)

### Shared State
- `Arc<RwLock<Config>>` for thread-safe configuration access
- Minimal lock contention through fine-grained updates

## Testing Strategy

### Unit Tests
- StreamService has 13 unit tests covering edge cases
- Channel extraction tested thoroughly
- Run with: `cargo test --lib` (requires GTK dependencies)

### Integration Tests
- OAuth flow can be tested with mock servers
- API client testable with `reqwest::blocking` test utilities

## Dependencies

### Core
- **tokio**: Async runtime
- **reqwest**: HTTP client
- **serde/serde_json**: Serialization
- **anyhow/thiserror**: Error handling
- **tracing**: Structured logging

### GUI (Linux)
- **gtk**: GTK3 bindings
- **libappindicator**: System tray
- **notify-rust**: Desktop notifications

### Security
- **rustls/tokio-rustls**: TLS implementation
- **rcgen**: Self-signed certificate generation

## Future Improvements

1. **Error Handling**: Expand custom error types throughout codebase
2. **Testing**: Add integration tests for OAuth flow
3. **Async Consistency**: Replace remaining thread spawns with async
4. **Configuration**: Add configuration validation at load time
5. **Logging**: Structured logging with spans for better debugging

## Migration Guide

If updating from previous versions:

1. **OAuth Module**: Import from `api::oauth` instead of direct file
2. **Stream Operations**: Use `StreamService::open_stream()` instead of `Config::open_stream_url()`
3. **Constants**: Import from `constants` module
4. **Errors**: Handle specific error types when needed

## Performance Considerations

- **Lazy Loading**: API calls only when needed
- **Caching**: Config loaded once, cached in Arc<RwLock<>>
- **Async I/O**: Non-blocking for API and file operations
- **Efficient Updates**: Only rebuild tray menu when streams change

## Security

- **TLS**: Self-signed certificates for localhost OAuth callback
- **Token Storage**: Access tokens stored in config file (user-only permissions)
- **No Secrets**: Client ID is public (OAuth2 implicit flow)
- **Input Validation**: URL and channel name validation

---

**Last Updated**: 2025-11-15
**Version**: 0.1.1
