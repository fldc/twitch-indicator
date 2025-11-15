use anyhow::{Context, Result};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls;

/// Generates a self-signed TLS certificate for the OAuth callback server.
/// Required because the implicit OAuth flow delivers tokens via URL fragments,
/// which are only accessible via JavaScript running in a browser.
pub fn generate_self_signed_cert() -> Result<rustls::ServerConfig> {
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
