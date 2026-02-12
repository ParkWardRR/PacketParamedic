//! Peer identity and authorization for the PacketParamedic Reflector.
//!
//! During mTLS handshake, the remote peer presents a certificate containing
//! its `EndpointId` in a Subject Alternative Name.  The `PeerId` newtype
//! wraps that extracted identifier, and `AuthorizedPeers` enforces an
//! allow-list of peers that may connect to this reflector.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cert;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during peer identity operations.
#[derive(Debug, Error)]
pub enum PeerError {
    /// The certificate could not be parsed or does not contain a valid
    /// PacketParamedic endpoint ID SAN.
    #[error("failed to extract peer ID from certificate: {0}")]
    CertificateExtraction(String),

    /// The peer is not in the authorized peers set.
    #[error("peer {0} is not authorized")]
    Unauthorized(PeerId),
}

// ---------------------------------------------------------------------------
// PeerId
// ---------------------------------------------------------------------------

/// Opaque identifier for a remote PacketParamedic peer, extracted from its
/// mTLS certificate.
///
/// The inner string is the `EndpointId` representation (e.g.
/// `PP-5R6Q-2M1K-9D3F-C3`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    /// Create a `PeerId` from a raw endpoint ID string.
    ///
    /// No validation is performed; use [`PeerId::from_cert`] for safe
    /// extraction from a certificate.
    pub fn new(id: impl Into<String>) -> Self {
        PeerId(id.into())
    }

    /// Extract a `PeerId` from a DER-encoded X.509 certificate by looking for
    /// the `pp-id-*` Subject Alternative Name.
    pub fn from_cert(cert_der: &[u8]) -> Result<Self, PeerError> {
        let id = cert::extract_peer_id_from_cert(cert_der)
            .map_err(|e| PeerError::CertificateExtraction(e.to_string()))?;
        Ok(PeerId(id))
    }

    /// Return the inner endpoint ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for PeerId {
    fn from(s: String) -> Self {
        PeerId(s)
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        PeerId(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// AuthorizedPeers
// ---------------------------------------------------------------------------

/// An allow-list of peers that are permitted to connect to this reflector.
///
/// Persistence is supported through `Serialize` / `Deserialize`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuthorizedPeers {
    peers: HashSet<PeerId>,
}

impl AuthorizedPeers {
    /// Create an empty authorized peers set.
    pub fn new() -> Self {
        AuthorizedPeers {
            peers: HashSet::new(),
        }
    }

    /// Check whether a given peer is authorized.
    pub fn is_authorized(&self, peer: &PeerId) -> bool {
        self.peers.contains(peer)
    }

    /// Add a peer to the authorized set.
    pub fn add(&mut self, peer: PeerId) {
        self.peers.insert(peer);
    }

    /// Remove a peer from the authorized set.  Returns `true` if the peer
    /// was present.
    pub fn remove(&mut self, peer: &PeerId) -> bool {
        self.peers.remove(peer)
    }

    /// Return the number of authorized peers.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Return `true` if the authorized set is empty.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Iterate over all authorized peer IDs.
    pub fn iter(&self) -> impl Iterator<Item = &PeerId> {
        self.peers.iter()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert::generate_self_signed_cert;
    use crate::identity::Identity;

    #[test]
    fn test_peer_id_from_cert() {
        let identity = Identity::generate();
        let expected = identity.endpoint_id().to_string();

        let (cert_der, _) = generate_self_signed_cert(&identity).unwrap();

        let peer_id = PeerId::from_cert(&cert_der).unwrap();
        assert_eq!(peer_id.as_str(), expected);
        assert_eq!(peer_id.to_string(), expected);
    }

    #[test]
    fn test_peer_id_display() {
        let peer = PeerId::new("PP-ABCD-1234-EFGH-K");
        assert_eq!(format!("{peer}"), "PP-ABCD-1234-EFGH-K");
    }

    #[test]
    fn test_authorized_peers_add_remove() {
        let mut auth = AuthorizedPeers::new();
        assert!(auth.is_empty());

        let p1 = PeerId::new("PP-AAAA-BBBB-CCCC-0");
        let p2 = PeerId::new("PP-DDDD-EEEE-FFFF-1");

        auth.add(p1.clone());
        assert_eq!(auth.len(), 1);
        assert!(auth.is_authorized(&p1));
        assert!(!auth.is_authorized(&p2));

        auth.add(p2.clone());
        assert_eq!(auth.len(), 2);
        assert!(auth.is_authorized(&p2));

        assert!(auth.remove(&p1));
        assert!(!auth.is_authorized(&p1));
        assert_eq!(auth.len(), 1);

        // Removing again returns false.
        assert!(!auth.remove(&p1));
    }

    #[test]
    fn test_authorized_peers_serialization() {
        let mut auth = AuthorizedPeers::new();
        auth.add(PeerId::new("PP-AAAA-BBBB-CCCC-0"));
        auth.add(PeerId::new("PP-DDDD-EEEE-FFFF-1"));

        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: AuthorizedPeers = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert!(deserialized.is_authorized(&PeerId::new("PP-AAAA-BBBB-CCCC-0")));
        assert!(deserialized.is_authorized(&PeerId::new("PP-DDDD-EEEE-FFFF-1")));
    }

    #[test]
    fn test_peer_id_from_invalid_cert_fails() {
        let result = PeerId::from_cert(&[0xFF, 0x00, 0x01]);
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_error_display() {
        let err = PeerError::Unauthorized(PeerId::new("PP-TEST-1234-5678-A"));
        let msg = err.to_string();
        assert!(msg.contains("PP-TEST-1234-5678-A"));
        assert!(msg.contains("not authorized"));
    }

    #[test]
    fn test_peer_id_equality() {
        let a = PeerId::new("PP-AAAA-BBBB-CCCC-0");
        let b = PeerId::new("PP-AAAA-BBBB-CCCC-0");
        let c = PeerId::new("PP-XXXX-YYYY-ZZZZ-1");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_peer_id_from_conversions() {
        let from_string: PeerId = String::from("PP-1234-5678-9ABC-D").into();
        let from_str: PeerId = "PP-1234-5678-9ABC-D".into();
        assert_eq!(from_string, from_str);
    }
}
