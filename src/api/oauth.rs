#![allow(dead_code)]

use anyhow::{Context, Result, anyhow};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::{TlsAcceptor, rustls};
use tracing::{debug, error, info, warn};

use crate::api::models::TokenResponse;

const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TWITCH_TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const SCOPES: &[&str] = &["user:read:follows"];
const REDIRECT_PORT: u16 = 17563;
const REDIRECT_URI: &str = "https://localhost:17563";

pub struct OAuthFlow {
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthCallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

impl OAuthFlow {
    pub fn new(client_id: String) -> Self {
        Self { client_id }
    }

    fn generate_self_signed_cert() -> Result<rustls::ServerConfig> {
        let subject_alt_names = vec!["localhost".to_string()];
        let certified_key = generate_simple_self_signed(subject_alt_names)?;

        let cert_der = certified_key.cert.der();
        let private_key_der = certified_key.key_pair.serialize_der();

        let cert_chain = vec![CertificateDer::from(cert_der.to_vec())];
        let private_key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(private_key_der));

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .context("Failed to create TLS config")?;

        Ok(config)
    }

    pub async fn authenticate(&mut self) -> Result<TokenResponse> {
        let state = uuid::Uuid::new_v4().to_string();
        let auth_url = self.get_auth_url(&state);
        info!("Opening browser for authorization: {}", auth_url);

        let (sender, receiver) = oneshot::channel();

        self.start_callback_server_implicit(state, sender).await?;

        webbrowser::open(&auth_url).context("Failed to open browser")?;

        receiver.await.context("Failed to receive OAuth callback")?
    }

    async fn start_callback_server(
        &mut self,
        state: String,
        sender: oneshot::Sender<Result<TokenResponse>>,
    ) -> Result<()> {
        let tls_config = Self::generate_self_signed_cert()?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));

        let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
            .await
            .context("Failed to bind to port")?;

        info!(
            "Started OAuth callback server on https://127.0.0.1:{}",
            REDIRECT_PORT
        );
        debug!("Server listening on 127.0.0.1:{}", REDIRECT_PORT);

        let client_id = self.client_id.clone();

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        match Self::handle_https_request(tls_stream, &client_id, &state).await {
                            Ok(Some(token_response)) => {
                                let _ = sender.send(Ok(token_response));
                                return;
                            }
                            Ok(None) => continue, // No valid callback yet
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

    async fn handle_https_request(
        mut stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
        client_id: &str,
        expected_state: &str,
    ) -> Result<Option<TokenResponse>> {
        let mut reader = BufReader::new(&mut stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .context("Failed to read request line")?;

        debug!("Received HTTPS request: {}", request_line.trim());

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Invalid HTTP request"));
        }

        let path_and_query = parts[1];
        if let Some(query_start) = path_and_query.find('?') {
            let query = &path_and_query[query_start + 1..];
            debug!("Received OAuth callback: GET {}", path_and_query);

            let params = Self::parse_query_params(query);

            let response = "HTTP/1.1 200 OK\r\n\
                           Content-Type: text/html\r\n\
                           Connection: close\r\n\r\n\
                           <html><body><h1>Authorization successful!</h1>\
                           <p>You can close this window and return to the application.</p>\
                           </body></html>";

            stream
                .write_all(response.as_bytes())
                .await
                .context("Failed to write response")?;
            stream.flush().await.context("Failed to flush response")?;

            if let Some(error) = params.error {
                return Err(anyhow!("OAuth error: {}", error));
            }

            let code = params
                .code
                .ok_or_else(|| anyhow!("No authorization code received"))?;
            let state = params
                .state
                .ok_or_else(|| anyhow!("No state parameter received"))?;

            if state != expected_state {
                return Err(anyhow!(
                    "State mismatch: expected {}, got {}",
                    expected_state,
                    state
                ));
            }

            debug!("Exchanging authorization code for access token");
            let token_response = Self::exchange_code_for_token(client_id, &code).await?;

            return Ok(Some(token_response));
        }

        Ok(None)
    }

    fn parse_query_params(query: &str) -> AuthCallbackParams {
        let mut params = HashMap::new();

        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    key.to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }

        AuthCallbackParams {
            code: params.get("code").cloned(),
            state: params.get("state").cloned(),
            error: params.get("error").cloned(),
        }
    }

    async fn exchange_code_for_token(client_id: &str, code: &str) -> Result<TokenResponse> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", client_id),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", REDIRECT_URI),
        ];

        let response = client
            .post(TWITCH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to send token request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Token exchange failed: {}", error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        info!("Successfully exchanged authorization code for access token");
        Ok(token_response)
    }

    fn get_auth_url(&self, state: &str) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=token&scope={}&state={}&force_verify=true",
            TWITCH_AUTH_URL,
            self.client_id,
            urlencoding::encode(REDIRECT_URI),
            SCOPES.join(" "),
            state
        )
    }

    async fn start_callback_server_implicit(
        &mut self,
        state: String,
        sender: oneshot::Sender<Result<TokenResponse>>,
    ) -> Result<()> {
        let tls_config = Self::generate_self_signed_cert()?;
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
                        match Self::handle_https_request_implicit(tls_stream, &state).await {
                            Ok(Some(token_response)) => {
                                let _ = sender.send(Ok(token_response));
                                return;
                            }
                            Ok(None) => continue, // No valid callback yet
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

    async fn handle_https_request_implicit(
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

        let html_response = format!(
            r#"HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Content-Length: {}

<!DOCTYPE html>
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
        
        if (error) {{
            document.body.innerHTML = '<h1>Authorization failed: ' + error + '</h1>';
        }} else if (accessToken && state === '{}') {{
            fetch('/token', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify({{ 
                    access_token: accessToken,
                    token_type: params.get('token_type') || 'bearer',
                    scope: (params.get('scope') || '').split(' ')
                }})
            }}).then(() => {{
                document.body.innerHTML = '<h1>Authorization successful!</h1><p>You can close this window.</p>';
            }});
        }} else {{
            document.body.innerHTML = '<h1>Authorization failed: Invalid state or missing token</h1>';
        }}
    </script>
</body>
</html>"#,
            0, // Will calculate length
            expected_state
        );

        let content_length = html_response.len() - html_response.find("\r\n\r\n").unwrap_or(0) - 4;
        let html_response = html_response.replace(
            "Content-Length: 0",
            &format!("Content-Length: {content_length}"),
        );

        if request_line.starts_with("POST /token") {
            let mut content_length = 0;
            let mut line = String::new();

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

            let mut body = vec![0; content_length];
            tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body)
                .await
                .context("Failed to read request body")?;

            let body_str = String::from_utf8(body).context("Invalid UTF-8 in body")?;
            debug!("Received token POST body: {}", body_str);

            let token_response: TokenResponse =
                serde_json::from_str(&body_str).context("Failed to parse token response")?;

            let success_response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
            stream
                .write_all(success_response.as_bytes())
                .await
                .context("Failed to send success response")?;
            stream.flush().await.context("Failed to flush stream")?;

            return Ok(Some(token_response));
        } else {
            stream
                .write_all(html_response.as_bytes())
                .await
                .context("Failed to send HTML response")?;
            stream.flush().await.context("Failed to flush stream")?;
        }

        Ok(None)
    }
}
