
use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use tokio_util::codec::LengthDelimitedCodec;

/// Maximum frame payload size: 1 MB.
const MAX_FRAME_SIZE: usize = 1_048_576;

/// Length-prefixed frame codec for the Paramedic Link protocol.
pub struct LinkCodec {
    inner: LengthDelimitedCodec,
}

impl LinkCodec {
    pub fn new() -> Self {
        let inner = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .max_frame_length(MAX_FRAME_SIZE)
            .length_adjustment(0)
            .new_codec();

        Self { inner }
    }

    pub fn inner(&self) -> &LengthDelimitedCodec {
        &self.inner
    }

    pub fn into_inner(self) -> LengthDelimitedCodec {
        self.inner
    }
}

use crate::reflector_proto::rpc::LinkMessage;
use tokio_util::codec::{Decoder, Encoder};

impl Encoder<LinkMessage> for LinkCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: LinkMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_vec(&item).context("failed to serialize message")?;
        let bytes = Bytes::from(json);
        self.inner.encode(bytes, dst).map_err(|e| anyhow::anyhow!(e))
    }
}

impl Decoder for LinkCodec {
    type Item = LinkMessage;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src).map_err(|e| anyhow::anyhow!(e))? {
            Some(bytes) => {
                let msg = serde_json::from_slice(&bytes).context("failed to deserialize message")?;
                Ok(Some(msg))
            },
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflector_proto::rpc::{LinkMessage, MessagePayload, Hello};
    use tokio_util::codec::{Encoder, Decoder};

    #[test]
    fn test_round_trip() {
        let msg = LinkMessage {
            request_id: "test-1".to_string(),
            payload: MessagePayload::Hello(Hello {
                version: "1.0".to_string(),
                features: vec![],
            }),
        };

        let mut codec = LinkCodec::new();
        let mut buf = BytesMut::new();
        codec.encode(msg.clone(), &mut buf).expect("encode failed");

        assert!(buf.len() > 4);
        
        let mut decode_codec = LinkCodec::new();
        let decoded = decode_codec.decode(&mut buf).expect("decode failed").expect("should have frame");
        
        // request_id match?
        assert_eq!(msg.request_id, decoded.request_id);
    }
}
