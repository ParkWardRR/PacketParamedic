//! mTLS configuration for the PacketParamedic Reflector.
//!
//! Both server and client configs enforce TLS 1.3 only and use the `pp-link/1` ALPN.
//! Certificate verification is intentionally permissive — identity is verified at the
//! application layer by extracting the peer's public key from the presented certificate.

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring::default_provider;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    ClientConfig, DigitallySignedStruct, DistinguishedName, Error, ServerConfig, SignatureScheme,
};

/// ALPN protocol identifier for the Paramedic Link protocol.
const ALPN_PP_LINK: &[u8] = b"pp-link/1";

// ---------------------------------------------------------------------------
// Server-side: custom ClientCertVerifier
// ---------------------------------------------------------------------------

/// A permissive client certificate verifier that accepts any presented certificate.
///
/// Authorization is **not** performed at the TLS layer — instead the reflector
/// extracts the peer's public key from the certificate after the handshake and
/// checks it against the endpoint-ID allowlist at the application layer.
#[derive(Debug)]
struct AcceptAnyClientCert {
    supported_schemes: Vec<SignatureScheme>,
}

impl AcceptAnyClientCert {
    fn new(provider: &CryptoProvider) -> Self {
        Self {
            supported_schemes: provider
                .signature_verification_algorithms
                .supported_schemes(),
        }
    }
}

impl ClientCertVerifier for AcceptAnyClientCert {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        // Return empty — the client should always present whatever cert it has.
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        // Accept any certificate — peer identity verification happens at the app layer.
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // We only negotiate TLS 1.3, so this should never be called.
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // Delegate to the ring crypto provider for actual signature verification
        // so that the TLS handshake integrity is maintained even though we don't
        // validate the CA chain.
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported_schemes.clone()
    }
}

// ---------------------------------------------------------------------------
// Client-side: custom ServerCertVerifier
// ---------------------------------------------------------------------------

/// A permissive server certificate verifier that accepts any server certificate.
///
/// In the Paramedic Link protocol the client already knows the server's identity
/// via its endpoint-ID (derived from the server's public key), so CA-chain
/// verification is unnecessary.
#[derive(Debug)]
struct AcceptAnyServerCert {
    supported_schemes: Vec<SignatureScheme>,
}

impl AcceptAnyServerCert {
    fn new(provider: &CryptoProvider) -> Self {
        Self {
            supported_schemes: provider
                .signature_verification_algorithms
                .supported_schemes(),
        }
    }
}

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        // Accept any certificate — server identity is verified via endpoint-ID.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // We only negotiate TLS 1.3, so this should never be called.
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported_schemes.clone()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a `rustls::ServerConfig` for the reflector's mTLS listener.
///
/// - TLS 1.3 only
/// - Requires client certificates (mTLS)
/// - Uses [`AcceptAnyClientCert`] — authorization happens at the app layer
/// - ALPN: `pp-link/1`
pub fn build_server_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> Result<rustls::ServerConfig> {
    let provider = default_provider();
    let verifier = Arc::new(AcceptAnyClientCert::new(&provider));

    let cert = CertificateDer::from(cert_der);
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_der));

    let mut config = ServerConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("failed to set TLS 1.3 protocol version")?
        .with_client_cert_verifier(verifier)
        .with_single_cert(vec![cert], key)
        .context("failed to configure server certificate")?;

    config.alpn_protocols = vec![ALPN_PP_LINK.to_vec()];

    Ok(config)
}

/// Build a `rustls::ClientConfig` for connecting to a remote reflector.
///
/// - TLS 1.3 only
/// - Presents the given client certificate (for mTLS)
/// - Uses [`AcceptAnyServerCert`] — identity verified via endpoint-ID
/// - ALPN: `pp-link/1`
pub fn build_client_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> Result<rustls::ClientConfig> {
    let provider = default_provider();
    let verifier = Arc::new(AcceptAnyServerCert::new(&provider));

    let cert = CertificateDer::from(cert_der);
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_der));

    let mut config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("failed to set TLS 1.3 protocol version")?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(vec![cert], key)
        .context("failed to configure client certificate")?;

    config.alpn_protocols = vec![ALPN_PP_LINK.to_vec()];

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a self-signed certificate + key pair using rcgen for testing.
    fn generate_test_cert() -> (Vec<u8>, Vec<u8>) {
        let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).unwrap();
        let params = rcgen::CertificateParams::new(vec!["localhost".to_string()]).unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        (cert.der().to_vec(), key_pair.serialize_der().to_vec())
    }

    #[test]
    fn test_build_server_config() {
        let (cert_der, key_der) = generate_test_cert();
        let config = build_server_config(cert_der, key_der).expect("should build server config");
        assert_eq!(config.alpn_protocols, vec![ALPN_PP_LINK.to_vec()]);
    }

    #[test]
    fn test_build_client_config() {
        let (cert_der, key_der) = generate_test_cert();
        let config = build_client_config(cert_der, key_der).expect("should build client config");
        assert_eq!(config.alpn_protocols, vec![ALPN_PP_LINK.to_vec()]);
    }

    #[test]
    fn test_accept_any_client_cert_is_mandatory() {
        let verifier = AcceptAnyClientCert::new(&default_provider());
        assert!(verifier.offer_client_auth());
        assert!(verifier.client_auth_mandatory());
        assert!(verifier.root_hint_subjects().is_empty());
    }

    #[test]
    fn test_accept_any_client_cert_verify_returns_ok() {
        let verifier = AcceptAnyClientCert::new(&default_provider());
        let dummy_cert = CertificateDer::from(vec![0u8; 1]);
        let result = verifier.verify_client_cert(&dummy_cert, &[], UnixTime::now());
        assert!(result.is_ok());
    }

    #[test]
    fn test_accept_any_server_cert_verify_returns_ok() {
        let verifier = AcceptAnyServerCert::new(&default_provider());
        let dummy_cert = CertificateDer::from(vec![0u8; 1]);
        let server_name = ServerName::try_from("example.com").unwrap();
        let result =
            verifier.verify_server_cert(&dummy_cert, &[], &server_name, &[], UnixTime::now());
        assert!(result.is_ok());
    }
}
