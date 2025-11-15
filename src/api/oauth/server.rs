use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::api::models::TokenResponse;
use crate::constants::REDIRECT_PORT;
use super::tls;

/// HTML template for the OAuth callback page.
/// Uses JavaScript to extract the access token from the URL fragment
/// and POST it back to the server.
const CALLBACK_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Twitch Authorization</title>
</head>
<body>
    <h1>Processing authorization...</h1>
    <script>
        const fragment = window.location.hash.substring(1);
        const params = new URLSearchParams(fragment);

        const accessToken = params.get('access_token');
        const state = params.get('state');
        const error = params.get('error');

        if (error) {
            document.body.innerHTML = '<h1>Authorization failed: ' + error + '</h1>';
        } else if (accessToken && state === 'STATE_PLACEHOLDER') {
            fetch('/token', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    access_token: accessToken,
                    token_type: params.get('token_type') || 'bearer',
                    scope: (params.get('scope') || '').split(' ')
                })
            }).then(() => {
                document.body.innerHTML = '<h1>Authorization successful!</h1><p>You can close this window.</p>';
            });
        } else {
            document.body.innerHTML = '<h1>Authorization failed: Invalid state or missing token</h1>';
        }
    </script>
</body>
</html>"#;

/// Starts the OAuth callback server and waits for the authorization response.
/// Uses implicit OAuth flow where the token is delivered via URL fragment.
pub async fn start_callback_server(
    state: String,
    sender: oneshot::Sender<Result<TokenResponse>>,
) -> Result<()> {
    let tls_config = tls::generate_self_signed_cert()?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
        .await
        .context("Failed to bind to port")?;

    info!(
        "Started OAuth callback server on https://127.0.0.1:{}",
        REDIRECT_PORT
    );
    debug!("Server listening on 127.0.0.1:{}", REDIRECT_PORT);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    match handle_https_request(tls_stream, &state).await {
                        Ok(Some(token_response)) => {
                            let _ = sender.send(Ok(token_response));
                            return;
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            error!("Failed to handle HTTPS request: {}", e);
                            let _ = sender.send(Err(e));
                            return;
                        }
                    }
                }
                Err(e) => {
                    warn!("TLS handshake failed: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// Handles HTTPS requests from the OAuth callback.
/// Serves the HTML page for initial GET requests, then receives the token via POST.
async fn handle_https_request(
    mut stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    expected_state: &str,
) -> Result<Option<TokenResponse>> {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .context("Failed to read request line")?;

    debug!("Received HTTPS request: {}", request_line.trim());

    if request_line.starts_with("POST /token") {
        handle_token_post(&mut reader, &mut stream).await
    } else {
        serve_callback_html(&mut stream, expected_state).await?;
        Ok(None)
    }
}

/// Serves the HTML callback page with the expected state embedded.
async fn serve_callback_html(
    stream: &mut tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    expected_state: &str,
) -> Result<()> {
    let html_content = CALLBACK_HTML_TEMPLATE.replace("STATE_PLACEHOLDER", expected_state);

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html_content.len(),
        html_content
    );

    stream
        .write_all(response.as_bytes())
        .await
        .context("Failed to send HTML response")?;
    stream.flush().await.context("Failed to flush stream")?;

    Ok(())
}

/// Handles the POST request containing the OAuth token.
async fn handle_token_post(
    reader: &mut BufReader<&mut tokio_rustls::server::TlsStream<tokio::net::TcpStream>>,
    stream: &mut tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
) -> Result<Option<TokenResponse>> {
    let mut content_length = 0;
    let mut line = String::new();

    // Read headers to get content length
    loop {
        line.clear();
        reader
            .read_line(&mut line)
            .await
            .context("Failed to read header")?;
        if line.trim().is_empty() {
            break;
        }
        if line.to_lowercase().starts_with("content-length:") {
            content_length = line
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }

    // Read the request body
    let mut body = vec![0; content_length];
    tokio::io::AsyncReadExt::read_exact(reader, &mut body)
        .await
        .context("Failed to read request body")?;

    let body_str = String::from_utf8(body).context("Invalid UTF-8 in body")?;
    debug!("Received token POST body: {}", body_str);

    let token_response: TokenResponse =
        serde_json::from_str(&body_str).context("Failed to parse token response")?;

    // Send success response
    let success_response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
    stream
        .write_all(success_response.as_bytes())
        .await
        .context("Failed to send success response")?;
    stream.flush().await.context("Failed to flush stream")?;

    Ok(Some(token_response))
}
