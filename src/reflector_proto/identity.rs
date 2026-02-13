//! Ed25519 identity management for PacketParamedic Reflector endpoints.
//!
//! Each reflector has a persistent Ed25519 keypair that uniquely identifies it.
//! The public key is encoded as a Crockford Base32 `EndpointId` with a Luhn
//! check digit, prefixed with `PP-`.

use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Crockford Base32
// ---------------------------------------------------------------------------

/// Crockford Base32 alphabet (excludes I, L, O, U).
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Encode bytes to Crockford Base32 (uppercase).
fn crockford_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left: u32 = 0;

    for &byte in data {
        buffer = (buffer << 8) | u64::from(byte);
        bits_left += 8;

        while bits_left >= 5 {
            bits_left -= 5;
            let idx = ((buffer >> bits_left) & 0x1F) as usize;
            out.push(CROCKFORD_ALPHABET[idx] as char);
        }
    }

    // Flush remaining bits (padded with zeros on the right).
    if bits_left > 0 {
        let idx = ((buffer << (5 - bits_left)) & 0x1F) as usize;
        out.push(CROCKFORD_ALPHABET[idx] as char);
    }

    out
}

/// Map a Crockford Base32 character to its numeric value (0..31).
/// Returns `None` for invalid characters.
fn crockford_value(c: char) -> Option<usize> {
    let c = c.to_ascii_uppercase();
    match c {
        '0' | 'O' => Some(0),
        '1' | 'I' | 'L' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        'A' => Some(10),
        'B' => Some(11),
        'C' => Some(12),
        'D' => Some(13),
        'E' => Some(14),
        'F' => Some(15),
        'G' => Some(16),
        'H' => Some(17),
        'J' => Some(18),
        'K' => Some(19),
        'M' => Some(20),
        'N' => Some(21),
        'P' => Some(22),
        'Q' => Some(23),
        'R' => Some(24),
        'S' => Some(25),
        'T' => Some(26),
        'V' => Some(27),
        'W' => Some(28),
        'X' => Some(29),
        'Y' => Some(30),
        'Z' => Some(31),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Luhn mod N check digit  (N = 32 for Crockford Base32)
// ---------------------------------------------------------------------------

/// Compute a Luhn mod-N check digit over a sequence of symbol values.
///
/// This implements the Luhn mod N algorithm as described in the ISO/IEC 7812
/// generalisation.  `n` is the alphabet size (32 for Crockford Base32).
fn luhn_mod_n_check(values: &[usize], n: usize) -> usize {
    let mut factor = 2; // start doubling from the rightmost digit
    let mut sum = 0;

    for &v in values.iter().rev() {
        let mut addend = factor * v;
        factor = if factor == 2 { 1 } else { 2 };

        // "fold" addend: sum its digits in base-n
        addend = (addend / n) + (addend % n);
        sum += addend;
    }

    let remainder = sum % n;
    (n - remainder) % n
}

/// Compute the Crockford Base32 Luhn check character for a given string of
/// Crockford Base32 characters.
fn crockford_luhn_check_char(encoded: &str) -> char {
    let values: Vec<usize> = encoded
        .chars()
        .filter_map(crockford_value)
        .collect();

    let check = luhn_mod_n_check(&values, 32);
    CROCKFORD_ALPHABET[check] as char
}

/// Validate that the final character is the correct Luhn check digit.
fn crockford_luhn_validate(encoded_with_check: &str) -> bool {
    let chars: Vec<char> = encoded_with_check.chars().collect();
    if chars.is_empty() {
        return false;
    }
    let payload = &chars[..chars.len() - 1];
    let check_char = chars[chars.len() - 1];

    let values: Vec<usize> = payload.iter().filter_map(|&c| crockford_value(c)).collect();
    if values.len() != payload.len() {
        return false;
    }

    let expected = luhn_mod_n_check(&values, 32);
    crockford_value(check_char) == Some(expected)
}

// ---------------------------------------------------------------------------
// EndpointId
// ---------------------------------------------------------------------------

/// A human-readable identifier for a PacketParamedic endpoint, derived from
/// the Ed25519 public key.
///
/// Format: `PP-XXXX-XXXX-XXXX-...-C` where `X` is Crockford Base32 and `C`
/// is the Luhn mod-32 check digit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointId(String);

impl EndpointId {
    /// Build an `EndpointId` from raw public key bytes.
    pub fn from_public_key_bytes(pk_bytes: &[u8; 32]) -> Self {
        let encoded = crockford_encode(pk_bytes);
        let check = crockford_luhn_check_char(&encoded);

        // Group into chunks of 4, then append check digit.
        let mut formatted = String::from("PP");
        for chunk in encoded.as_bytes().chunks(4) {
            formatted.push('-');
            for &b in chunk {
                formatted.push(b as char);
            }
        }
        formatted.push('-');
        formatted.push(check);

        EndpointId(formatted)
    }

    /// Return the raw string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate the Luhn check digit of this endpoint ID.
    pub fn validate(&self) -> bool {
        // Strip the "PP-" prefix, remove dashes, then validate.
        let stripped = self.0.strip_prefix("PP-").unwrap_or(&self.0);
        let clean: String = stripped.chars().filter(|&c| c != '-').collect();
        crockford_luhn_validate(&clean)
    }
}

impl fmt::Display for EndpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Ed25519 identity for this reflector node.
pub struct Identity {
    signing_key: SigningKey,
}

impl Identity {
    /// Generate a brand-new random Ed25519 identity.
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Identity { signing_key }
    }

    /// Load a private key from `path` (raw 32-byte secret key).
    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = fs::read(path)
            .with_context(|| format!("failed to read identity key from {}", path.display()))?;

        anyhow::ensure!(
            bytes.len() == 32,
            "identity key file must be exactly 32 bytes, got {}",
            bytes.len()
        );

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        bytes.zeroize();

        let signing_key = SigningKey::from_bytes(&key_bytes);
        key_bytes.zeroize();

        Ok(Identity { signing_key })
    }

    /// Persist the 32-byte secret key to `path` with mode 0600.
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut key_bytes = self.signing_key.to_bytes();

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        fs::write(path, key_bytes)
            .with_context(|| format!("failed to write identity key to {}", path.display()))?;

        // Set file permissions to 0600 (owner read/write only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| format!("failed to set permissions on {}", path.display()))?;
        }

        key_bytes.zeroize();
        Ok(())
    }

    /// Load an existing identity from `path`, or generate and save a new one.
    pub fn load_or_generate(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            let id = Self::generate();
            id.save(path)?;
            Ok(id)
        }
    }

    /// Return a reference to the public verifying key.
    pub fn public_key(&self) -> &VerifyingKey {
        self.signing_key.as_ref()
    }

    /// Return the signing key (needed for certificate generation).
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Derive the human-readable endpoint ID from the public key.
    pub fn endpoint_id(&self) -> EndpointId {
        let pk_bytes = self.public_key().to_bytes();
        EndpointId::from_public_key_bytes(&pk_bytes)
    }
}

impl Drop for Identity {
    fn drop(&mut self) {
        // SigningKey implements ZeroizeOnDrop, but we make the intent explicit.
        let _ = &self.signing_key;
    }
}
