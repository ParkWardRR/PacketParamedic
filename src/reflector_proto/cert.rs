
use anyhow::{anyhow, Context, Result};
use rcgen::{CertificateParams, KeyPair, PKCS_ED25519};
use rustls_pki_types::PrivatePkcs8KeyDer;
use time::{Duration, OffsetDateTime};
use tracing::debug;
use x509_parser::prelude::{FromDer, GeneralName, ParsedExtension, X509Certificate};

use crate::reflector_proto::identity::Identity;

const ED25519_PKCS8_V1_PREFIX: [u8; 16] = [
    0x30, 0x2E, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2B, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

fn ed25519_to_pkcs8_der(secret: &[u8; 32]) -> Vec<u8> {
    let mut der = Vec::with_capacity(48);
    der.extend_from_slice(&ED25519_PKCS8_V1_PREFIX);
    der.extend_from_slice(secret);
    der
}

pub fn generate_self_signed_cert(identity: &Identity) -> Result<(Vec<u8>, Vec<u8>)> {
    let endpoint_id = identity.endpoint_id();
    let id_str = endpoint_id.to_string();
    let san_value = format!("pp-id-{}", id_str);

    debug!(endpoint_id = %id_str, "generating self-signed certificate");

    let secret_bytes = identity.signing_key().to_bytes();
    let pkcs8_der = ed25519_to_pkcs8_der(&secret_bytes);

    let pkcs8_ref = PrivatePkcs8KeyDer::from(pkcs8_der.as_slice());
    let key_pair = KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8_ref, &PKCS_ED25519)
        .context("failed to create rcgen KeyPair from Ed25519 PKCS#8 DER")?;

    let mut params = CertificateParams::new(vec![san_value.clone()])
        .context("failed to create certificate params")?;

    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, id_str.clone());

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(3652); // ~10 years

    let cert = params
        .self_signed(&key_pair)
        .context("failed to self-sign certificate")?;

    let cert_der = cert.der().to_vec();
    let key_der = pkcs8_der;

    Ok((cert_der, key_der))
}

pub fn extract_peer_id_from_cert(cert_der: &[u8]) -> Result<String> {
    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| anyhow!("failed to parse X.509 certificate: {e}"))?;

    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                match name {
                    GeneralName::DNSName(dns) => {
                        if let Some(id) = dns.strip_prefix("pp-id-") {
                            return Ok(id.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Err(anyhow!("no pp-id-* Subject Alternative Name found in certificate"))
}
