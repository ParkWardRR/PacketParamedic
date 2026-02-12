//! Authorization gate for the PacketParamedic Reflector.
//!
//! Enforces an allow-list of authorized peers and provides a time-limited
//! pairing flow for enrolling new peers.  All mutable state is protected
//! by async-aware locks so the gate is safe to share across tasks.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::config::AccessConfig;
use crate::peer::{AuthorizedPeers, PeerId};

/// Charset for 8-digit pairing codes: uppercase alphanumeric, no ambiguous chars (0/O, 1/I/L).
const PAIRING_CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Generate an 8-character alphanumeric pairing code.
fn generate_pairing_code() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..PAIRING_CHARSET.len());
            PAIRING_CHARSET[idx] as char
        })
        .collect()
}

// ---------------------------------------------------------------------------
// AuthDecision
// ---------------------------------------------------------------------------

/// Result of an authorization check against the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    /// The peer is in the authorized set and may proceed.
    Allowed,
    /// The peer is not authorized.  The `String` contains a human-readable
    /// reason suitable for logging or returning over RPC.
    Denied(String),
    /// The peer is unknown but pairing mode is active -- the caller should
    /// initiate the pairing handshake.
    PairingRequired,
}

// ---------------------------------------------------------------------------
// PairingToken
// ---------------------------------------------------------------------------

/// A one-time token generated when pairing mode is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingToken {
    /// The opaque token string (UUID v4).
    pub token: String,
    /// ISO 8601 timestamp after which the token is no longer valid.
    pub expires_at: String,
}

/// Internal pairing state shared behind a `Mutex`.
#[derive(Debug)]
struct PairingState {
    /// `Some` while pairing mode is active.
    active_token: Option<PairingToken>,
    /// Parsed expiry for fast comparison.
    expiry: Option<chrono::DateTime<Utc>>,
}

impl PairingState {
    fn new() -> Self {
        Self {
            active_token: None,
            expiry: None,
        }
    }

    fn is_active(&self) -> bool {
        match self.expiry {
            Some(exp) => Utc::now() < exp,
            None => false,
        }
    }

    fn validate_token(&self, candidate: &str) -> bool {
        if !self.is_active() {
            return false;
        }
        match &self.active_token {
            Some(tok) => tok.token.eq_ignore_ascii_case(candidate),
            None => false,
        }
    }

    /// Consume the token so it cannot be reused.
    fn consume(&mut self) {
        self.active_token = None;
        self.expiry = None;
    }
}

// ---------------------------------------------------------------------------
// AuthGate
// ---------------------------------------------------------------------------

/// Thread-safe authorization gate.
///
/// Holds the set of authorized peers (behind an `RwLock`) and transient
/// pairing state (behind a `Mutex`).
#[derive(Clone)]
pub struct AuthGate {
    peers: Arc<RwLock<AuthorizedPeers>>,
    pairing: Arc<Mutex<PairingState>>,
    pairing_enabled: bool,
}

impl AuthGate {
    /// Build a new `AuthGate` from the access configuration.
    ///
    /// Pre-authorized peer IDs listed in the config are loaded into the
    /// initial allow-list.
    pub fn new(config: &AccessConfig) -> Self {
        let mut authorized = AuthorizedPeers::new();
        for id_str in &config.authorized_peers {
            authorized.add(PeerId::new(id_str.clone()));
        }
        info!(
            count = authorized.len(),
            "initialized auth gate with pre-authorized peers"
        );

        Self {
            peers: Arc::new(RwLock::new(authorized)),
            pairing: Arc::new(Mutex::new(PairingState::new())),
            pairing_enabled: config.pairing_enabled,
        }
    }

    /// Check whether `peer_id` is authorized to use this reflector.
    pub async fn check(&self, peer_id: &PeerId) -> AuthDecision {
        let peers = self.peers.read().await;
        if peers.is_authorized(peer_id) {
            debug!(peer = %peer_id, "peer authorized");
            return AuthDecision::Allowed;
        }
        drop(peers);

        // Not in the allow-list.  If pairing mode is configured and active,
        // signal that the caller should attempt pairing.
        if self.pairing_enabled {
            let state = self.pairing.lock().await;
            if state.is_active() {
                debug!(peer = %peer_id, "peer unknown but pairing mode active");
                return AuthDecision::PairingRequired;
            }
        }

        let reason = format!("peer {} is not authorized", peer_id);
        warn!(peer = %peer_id, "authorization denied");
        AuthDecision::Denied(reason)
    }

    /// Enable pairing mode for `ttl` duration and return a one-time token.
    ///
    /// Generates a random 8-character alphanumeric code.
    /// If pairing is already active the previous token is replaced.
    pub async fn enable_pairing(&self, ttl: Duration) -> PairingToken {
        self.enable_pairing_with_code(ttl, generate_pairing_code()).await
    }

    /// Enable pairing mode with a specific code (for bidirectional pairing).
    ///
    /// Use this when the other side generated the code and provided it to us.
    pub async fn enable_pairing_with_code(&self, ttl: Duration, code: String) -> PairingToken {
        let expiry = Utc::now() + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::seconds(300));
        let token = PairingToken {
            token: code.to_uppercase(),
            expires_at: expiry.to_rfc3339(),
        };

        let mut state = self.pairing.lock().await;
        state.active_token = Some(token.clone());
        state.expiry = Some(expiry);

        info!(
            expires_at = %expiry.to_rfc3339(),
            "pairing mode enabled"
        );

        token
    }

    /// Attempt to pair a new peer using a pairing `token`.
    ///
    /// If the token is valid and has not expired, `peer_id` is added to the
    /// authorized set and the token is consumed (one-time use).
    pub async fn try_pair(&self, peer_id: &PeerId, token: &str) -> Result<()> {
        let mut state = self.pairing.lock().await;

        if !state.validate_token(token) {
            bail!("invalid or expired pairing token");
        }

        // Consume the token so it cannot be reused.
        state.consume();
        drop(state);

        // Add the peer to the authorized set.
        let mut peers = self.peers.write().await;
        peers.add(peer_id.clone());

        info!(peer = %peer_id, "peer paired successfully");
        Ok(())
    }

    /// Add a peer directly to the authorized set (admin operation).
    pub async fn add_peer(&self, peer_id: PeerId) {
        let mut peers = self.peers.write().await;
        peers.add(peer_id);
    }

    /// Remove a peer from the authorized set.  Returns `true` if the peer
    /// was present.
    pub async fn remove_peer(&self, peer_id: &PeerId) -> bool {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id)
    }

    /// Return the number of currently authorized peers.
    pub async fn peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    /// Return whether pairing support is configured (does not imply active).
    pub fn pairing_configured(&self) -> bool {
        self.pairing_enabled
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(peers: Vec<&str>, pairing: bool) -> AccessConfig {
        AccessConfig {
            pairing_enabled: pairing,
            authorized_peers: peers.into_iter().map(String::from).collect(),
        }
    }

    #[tokio::test]
    async fn test_authorized_peer_allowed() {
        let config = make_config(vec!["PP-AAAA-BBBB-CCCC-0"], false);
        let gate = AuthGate::new(&config);

        let peer = PeerId::new("PP-AAAA-BBBB-CCCC-0");
        assert_eq!(gate.check(&peer).await, AuthDecision::Allowed);
    }

    #[tokio::test]
    async fn test_unknown_peer_denied() {
        let config = make_config(vec!["PP-AAAA-BBBB-CCCC-0"], false);
        let gate = AuthGate::new(&config);

        let peer = PeerId::new("PP-XXXX-YYYY-ZZZZ-1");
        let decision = gate.check(&peer).await;
        assert!(matches!(decision, AuthDecision::Denied(_)));
    }

    #[tokio::test]
    async fn test_pairing_required_when_active() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        // Enable pairing with a generous TTL.
        let _token = gate.enable_pairing(Duration::from_secs(300)).await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        assert_eq!(gate.check(&peer).await, AuthDecision::PairingRequired);
    }

    #[tokio::test]
    async fn test_pairing_not_signaled_when_disabled() {
        let config = make_config(vec![], false);
        let gate = AuthGate::new(&config);

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        let decision = gate.check(&peer).await;
        assert!(matches!(decision, AuthDecision::Denied(_)));
    }

    #[tokio::test]
    async fn test_try_pair_success() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        let token = gate.enable_pairing(Duration::from_secs(300)).await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        gate.try_pair(&peer, &token.token).await.unwrap();

        // Peer should now be authorized.
        assert_eq!(gate.check(&peer).await, AuthDecision::Allowed);
        assert_eq!(gate.peer_count().await, 1);
    }

    #[tokio::test]
    async fn test_try_pair_bad_token() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        let _token = gate.enable_pairing(Duration::from_secs(300)).await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        let result = gate.try_pair(&peer, "wrong-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_pair_token_consumed() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        let token = gate.enable_pairing(Duration::from_secs(300)).await;

        let peer1 = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        gate.try_pair(&peer1, &token.token).await.unwrap();

        // Second attempt with the same token should fail (consumed).
        let peer2 = PeerId::new("PP-ANOT-HERR-PEER-3");
        let result = gate.try_pair(&peer2, &token.token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_and_remove_peer() {
        let config = make_config(vec![], false);
        let gate = AuthGate::new(&config);

        let peer = PeerId::new("PP-AAAA-BBBB-CCCC-0");
        gate.add_peer(peer.clone()).await;
        assert_eq!(gate.check(&peer).await, AuthDecision::Allowed);
        assert_eq!(gate.peer_count().await, 1);

        assert!(gate.remove_peer(&peer).await);
        assert!(matches!(gate.check(&peer).await, AuthDecision::Denied(_)));
        assert_eq!(gate.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_pairing_configured_flag() {
        let enabled = AuthGate::new(&make_config(vec![], true));
        assert!(enabled.pairing_configured());

        let disabled = AuthGate::new(&make_config(vec![], false));
        assert!(!disabled.pairing_configured());
    }

    #[tokio::test]
    async fn test_pairing_code_format() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);
        let token = gate.enable_pairing(Duration::from_secs(300)).await;

        // Code should be exactly 8 characters, all uppercase alphanumeric.
        assert_eq!(token.token.len(), 8);
        assert!(token.token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn test_pairing_with_external_code() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        let token = gate
            .enable_pairing_with_code(Duration::from_secs(300), "ABCD1234".into())
            .await;
        assert_eq!(token.token, "ABCD1234");

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        gate.try_pair(&peer, "ABCD1234").await.unwrap();
        assert_eq!(gate.check(&peer).await, AuthDecision::Allowed);
    }

    #[tokio::test]
    async fn test_pairing_case_insensitive() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        gate.enable_pairing_with_code(Duration::from_secs(300), "ABCD1234".into())
            .await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        // Lowercase should also work.
        gate.try_pair(&peer, "abcd1234").await.unwrap();
        assert_eq!(gate.check(&peer).await, AuthDecision::Allowed);
    }

    #[tokio::test]
    async fn test_expired_pairing_denies() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        // Enable pairing with a zero-duration TTL (expires immediately).
        let _token = gate.enable_pairing(Duration::from_secs(0)).await;

        // A small sleep to ensure we are past the expiry.
        tokio::time::sleep(Duration::from_millis(10)).await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        let decision = gate.check(&peer).await;
        assert!(
            matches!(decision, AuthDecision::Denied(_)),
            "expired pairing should result in Denied, got: {:?}",
            decision
        );
    }

    #[tokio::test]
    async fn test_expired_pairing_token_rejected() {
        let config = make_config(vec![], true);
        let gate = AuthGate::new(&config);

        let token = gate.enable_pairing(Duration::from_secs(0)).await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        let peer = PeerId::new("PP-NEWP-EEEE-RRRR-2");
        let result = gate.try_pair(&peer, &token.token).await;
        assert!(result.is_err());
    }
}
