//! Self-signed X.509 certificate generation from Ed25519 identity.
//!
//! Each reflector generates a self-signed TLS certificate whose Subject
//! Alternative Name encodes the reflector's `EndpointId`.  During mTLS
//! handshake the peer can extract this SAN to verify which reflector (or
//! client) it is talking to.

use anyhow::{anyhow, Context, Result};
use rcgen::{CertificateParams, KeyPair, PKCS_ED25519};
use rustls_pki_types::PrivatePkcs8KeyDer;
use time::{Duration, OffsetDateTime};
use tracing::debug;
use x509_parser::prelude::{
    FromDer, GeneralName, ParsedExtension, X509Certificate,
};

use crate::identity::Identity;

// ---------------------------------------------------------------------------
// PKCS#8 DER helper for Ed25519
// ---------------------------------------------------------------------------

/// The fixed ASN.1 DER prefix for an Ed25519 PKCS#8 v1 private key.
///
/// Structure (RFC 8410):
/// ```text
/// SEQUENCE {
///   INTEGER 0                          -- version
///   SEQUENCE {
///     OID 1.3.101.112                  -- id-EdDSA / Ed25519
///   }
///   OCTET STRING {                     -- privateKey wrapped
///     OCTET STRING (32 bytes)          -- raw private key
///   }
/// }
/// ```
///
/// The prefix encodes everything up to (but not including) the 32 raw key
/// bytes.  Total DER = 16 prefix bytes + 32 key bytes = 48 bytes.
const ED25519_PKCS8_V1_PREFIX: [u8; 16] = [
    0x30, 0x2E, // SEQUENCE, length 46
    0x02, 0x01, 0x00, // INTEGER 0
    0x30, 0x05, // SEQUENCE, length 5
    0x06, 0x03, 0x2B, 0x65, 0x70, // OID 1.3.101.112
    0x04, 0x22, // OCTET STRING, length 34
    0x04, 0x20, // OCTET STRING, length 32  (inner)
];

/// Wrap a raw 32-byte Ed25519 secret key into a PKCS#8 v1 DER encoding.
fn ed25519_to_pkcs8_der(secret: &[u8; 32]) -> Vec<u8> {
    let mut der = Vec::with_capacity(48);
    der.extend_from_slice(&ED25519_PKCS8_V1_PREFIX);
    der.extend_from_slice(secret);
    der
}

// ---------------------------------------------------------------------------
// Certificate generation
// ---------------------------------------------------------------------------

/// Generate a self-signed X.509 certificate from the reflector's Ed25519
/// identity.
///
/// Returns `(cert_der, key_der)` where both are DER-encoded byte vectors
/// suitable for consumption by `rustls`.
///
/// The certificate embeds the endpoint ID as:
///   - Common Name (CN)
///   - Subject Alternative Name DNS entry: `pp:id:<ENDPOINT_ID>`
///
/// Validity period: 10 years from now.
pub fn generate_self_signed_cert(identity: &Identity) -> Result<(Vec<u8>, Vec<u8>)> {
    let endpoint_id = identity.endpoint_id();
    let id_str = endpoint_id.to_string();
    let san_value = format!("pp-id-{}", id_str);

    debug!(endpoint_id = %id_str, "generating self-signed certificate");

    // Build PKCS#8 DER from the raw ed25519-dalek secret key.
    let secret_bytes = identity.signing_key().to_bytes();
    let pkcs8_der = ed25519_to_pkcs8_der(&secret_bytes);

    // Construct rcgen KeyPair from PKCS#8 DER + explicit Ed25519 algorithm.
    let pkcs8_ref = PrivatePkcs8KeyDer::from(pkcs8_der.as_slice());
    let key_pair = KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8_ref, &PKCS_ED25519)
        .context("failed to create rcgen KeyPair from Ed25519 PKCS#8 DER")?;

    // Build certificate parameters.
    let mut params = CertificateParams::new(vec![san_value.clone()])
        .context("failed to create certificate params")?;

    // Set Common Name.
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, id_str.clone());

    // Set validity: 10 years.
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(3652); // ~10 years

    // Self-sign.
    let cert = params
        .self_signed(&key_pair)
        .context("failed to self-sign certificate")?;

    let cert_der = cert.der().to_vec();
    let key_der = pkcs8_der;

    debug!(
        cert_bytes = cert_der.len(),
        "self-signed certificate generated"
    );

    Ok((cert_der, key_der))
}

// ---------------------------------------------------------------------------
// Peer ID extraction from certificate
// ---------------------------------------------------------------------------

/// Extract the PacketParamedic endpoint ID from a DER-encoded X.509
/// certificate.
///
/// Looks for a Subject Alternative Name of the form `pp-id-PP-XXXX-...`
/// (encoded as a DNS name since URI SANs cannot contain arbitrary schemes
/// in all TLS stacks).
pub fn extract_peer_id_from_cert(cert_der: &[u8]) -> Result<String> {
    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| anyhow!("failed to parse X.509 certificate: {e}"))?;

    // Walk the extensions looking for SubjectAlternativeName.
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                match name {
                    GeneralName::DNSName(dns) => {
                        if let Some(id) = dns.strip_prefix("pp-id-") {
                            debug!(peer_id = %id, "extracted peer ID from certificate SAN");
                            return Ok(id.to_string());
                        }
                    }
                    GeneralName::RFC822Name(rfc822) => {
                        if let Some(id) = rfc822.strip_prefix("pp-id-") {
                            return Ok(id.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Err(anyhow!(
        "no pp-id-* Subject Alternative Name found in certificate"
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    #[test]
    fn test_generate_and_extract_peer_id() {
        let identity = Identity::generate();
        let expected_id = identity.endpoint_id().to_string();

        let (cert_der, key_der) = generate_self_signed_cert(&identity).unwrap();

        // Cert DER should be non-empty.
        assert!(!cert_der.is_empty(), "cert DER should not be empty");
        // Key DER should be 48 bytes (PKCS#8 Ed25519).
        assert_eq!(key_der.len(), 48, "Ed25519 PKCS#8 DER should be 48 bytes");

        // Extract the peer ID from the certificate and compare.
        let extracted = extract_peer_id_from_cert(&cert_der).unwrap();
        assert_eq!(
            extracted, expected_id,
            "extracted peer ID should match the identity's endpoint ID"
        );
    }

    #[test]
    fn test_cert_validity_period() {
        let identity = Identity::generate();
        let (cert_der, _) = generate_self_signed_cert(&identity).unwrap();

        let (_, cert) =
            x509_parser::prelude::X509Certificate::from_der(&cert_der).unwrap();

        let validity = cert.validity();
        let duration = validity.not_after.timestamp() - validity.not_before.timestamp();

        // Should be approximately 10 years (3652 days).
        let ten_years_secs: i64 = 3652 * 24 * 3600;
        assert!(
            (duration - ten_years_secs).unsigned_abs() < 86400,
            "certificate validity should be ~10 years, got {} seconds",
            duration
        );
    }

    #[test]
    fn test_cert_common_name() {
        let identity = Identity::generate();
        let expected_id = identity.endpoint_id().to_string();
        let (cert_der, _) = generate_self_signed_cert(&identity).unwrap();

        let (_, cert) =
            x509_parser::prelude::X509Certificate::from_der(&cert_der).unwrap();

        let cn = cert
            .subject()
            .iter_common_name()
            .next()
            .expect("certificate should have a CN");

        let cn_str = cn.as_str().expect("CN should be a valid string");
        assert_eq!(
            cn_str, expected_id,
            "certificate CN should match endpoint ID"
        );
    }

    #[test]
    fn test_pkcs8_der_structure() {
        let secret = [0xABu8; 32];
        let der = ed25519_to_pkcs8_der(&secret);
        assert_eq!(der.len(), 48);
        assert_eq!(&der[..16], &ED25519_PKCS8_V1_PREFIX);
        assert_eq!(&der[16..], &secret);
    }

    #[test]
    fn test_extract_fails_on_missing_san() {
        // Generate a certificate without the pp-id- SAN using rcgen directly.
        let key_pair = KeyPair::generate_for(&PKCS_ED25519).unwrap();
        let params = CertificateParams::new(vec!["example.com".to_string()]).unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        let cert_der = cert.der().to_vec();

        let result = extract_peer_id_from_cert(&cert_der);
        assert!(
            result.is_err(),
            "should fail when no pp-id- SAN is present"
        );
    }
}
