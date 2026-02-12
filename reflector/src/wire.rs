//! Length-prefixed frame codec for the Paramedic Link protocol.
//!
//! Frames are encoded as a 4-byte big-endian length prefix followed by a JSON payload.
//! The length field describes only the payload size (not including itself).
//! Maximum frame size is 1 MB.

use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_util::codec::LengthDelimitedCodec;

/// Maximum frame payload size: 1 MB.
const MAX_FRAME_SIZE: usize = 1_048_576;

/// Length-prefixed frame codec for the Paramedic Link protocol.
///
/// Wraps a [`LengthDelimitedCodec`] configured for:
/// - u32 big-endian length prefix (4 bytes)
/// - Max frame size: 1 MB
/// - Length field covers payload only (not the prefix itself)
pub struct LinkCodec {
    inner: LengthDelimitedCodec,
}

impl LinkCodec {
    /// Create a new `LinkCodec` with the standard Paramedic Link framing.
    pub fn new() -> Self {
        let inner = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .max_frame_length(MAX_FRAME_SIZE)
            .length_adjustment(0) // length field = payload length only
            .new_codec();

        Self { inner }
    }

    /// Returns a reference to the inner `LengthDelimitedCodec`.
    pub fn inner(&self) -> &LengthDelimitedCodec {
        &self.inner
    }

    /// Consumes self and returns the inner `LengthDelimitedCodec`.
    pub fn into_inner(self) -> LengthDelimitedCodec {
        self.inner
    }
}

impl Default for LinkCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a message to JSON, then wrap it in a length-prefixed frame.
///
/// Returns the complete frame (4-byte length prefix + JSON payload) as [`Bytes`].
pub fn encode_message<T: Serialize>(msg: &T) -> Result<Bytes> {
    let json = serde_json::to_vec(msg).context("failed to serialize message to JSON")?;

    if json.len() > MAX_FRAME_SIZE {
        anyhow::bail!(
            "serialized message ({} bytes) exceeds max frame size ({} bytes)",
            json.len(),
            MAX_FRAME_SIZE
        );
    }

    let len = json.len() as u32;
    let mut buf = BytesMut::with_capacity(4 + json.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&json);

    Ok(buf.freeze())
}

/// Decode a JSON message from a frame payload (without the length prefix).
///
/// This is the inverse of [`encode_message`]. The caller is responsible for
/// stripping the 4-byte length prefix before passing the payload here.
pub fn decode_message<T: DeserializeOwned>(frame: Bytes) -> Result<T> {
    serde_json::from_slice(&frame).context("failed to deserialize message from JSON frame")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        greeting: String,
        count: u64,
    }

    #[test]
    fn test_round_trip() {
        let original = TestMessage {
            greeting: "hello, paramedic".to_string(),
            count: 42,
        };

        // Encode the message into a framed buffer.
        let framed = encode_message(&original).expect("encode should succeed");

        // The first 4 bytes are the big-endian length prefix.
        assert!(framed.len() > 4);
        let payload_len =
            u32::from_be_bytes([framed[0], framed[1], framed[2], framed[3]]) as usize;
        assert_eq!(payload_len, framed.len() - 4);

        // Decode the payload (skip the 4-byte length prefix).
        let payload = framed.slice(4..);
        let decoded: TestMessage = decode_message(payload).expect("decode should succeed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_codec_default() {
        // Ensure Default and new() produce a valid codec.
        let _codec = LinkCodec::default();
        let _codec2 = LinkCodec::new();
    }

    #[test]
    fn test_oversized_message_rejected() {
        // Create a message whose JSON serialization exceeds 1 MB.
        let huge = TestMessage {
            greeting: "x".repeat(MAX_FRAME_SIZE + 1),
            count: 0,
        };
        let result = encode_message(&huge);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_struct_round_trip() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Empty {}

        let original = Empty {};
        let framed = encode_message(&original).expect("encode should succeed");
        let payload = framed.slice(4..);
        let decoded: Empty = decode_message(payload).expect("decode should succeed");
        assert_eq!(original, decoded);
    }
}
