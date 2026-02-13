
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::reflector_proto::{
    cert,
    identity::Identity,
    rpc::{self, LinkMessage, MessagePayload},
    wire::LinkCodec,
};

/// A client for the Paramedic Link protocol that talks to a Reflector.
pub struct ReflectorClient {
    framed: Framed<TlsStream<TcpStream>, LinkCodec>,
    request_counter: u64,
}

impl ReflectorClient {
    /// Connect to a reflector at `addr` using the given `identity`.
    ///
    /// This performs the TCP connect, mTLS handshake, and Paramedic Link Hello exchange.
    pub async fn connect(addr: SocketAddr, identity: &Identity) -> Result<Self> {
        // 1. Generate self-signed client certificate.
        let (cert_der, key_der) = cert::generate_self_signed_cert(identity)
            .context("failed to generate client certificate")?;
        
        let cert_chain = vec![CertificateDer::from(cert_der)];
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));

        // 2. Configure TLS Client.
        // We use a "dangerous" verifier because the reflector uses self-signed certs.
        // In a real implementation, we should pin the server ID after pairing.
        let verifier = Arc::new(BlindVerifier);
        
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(cert_chain, private_key)?;

        let connector = TlsConnector::from(Arc::new(config));

        // 3. Connect TCP.
        info!(address = %addr, "connecting to reflector");
        let tcp_stream = TcpStream::connect(addr).await
            .context("failed to connect TCP")?;

        // 4. Upgrade to TLS.
        // Use "reflector" as the server name for SNI (it's ignored by our verifier but required by API).
        let domain = ServerName::try_from("reflector").unwrap();
        let tls_stream = connector.connect(domain, tcp_stream).await
            .context("failed TLS handshake")?;

        // 5. Wrap in codec.
        let mut framed = Framed::new(tls_stream, LinkCodec::new());

        // 6. Send Hello.
        let hello = LinkMessage {
            request_id: "init-0".to_string(),
            payload: MessagePayload::Hello(rpc::Hello {
                version: "1.0".to_string(),
                features: vec!["throughput".to_string(), "udp_echo".to_string()],
            }),
        };
        framed.send(hello).await.context("failed to send Hello")?;

        // 7. Receive ServerHello.
        let response = framed.next().await
            .ok_or_else(|| anyhow!("connection closed before ServerHello"))?
            .context("failed to decode ServerHello frame")?;

        match response.payload {
            MessagePayload::ServerHello(sh) => {
                info!(server_version = %sh.version, "handshake complete");
            }
            other => anyhow::bail!("expected ServerHello, got {:?}", other),
        }

        Ok(Self {
            framed,
            request_counter: 1,
        })
    }

    /// Send a PairRequest and await the response.
    pub async fn pair(&mut self, token: String) -> Result<rpc::PairResponse> {
        let req_id = self.next_id();
        let msg = LinkMessage {
            request_id: req_id.clone(),
            payload: MessagePayload::PairRequest(rpc::PairRequest { token }),
        };

        self.framed.send(msg).await.context("failed to send PairRequest")?;
        
        // Wait for response matching req_id.
        let resp = self.expect_response(&req_id).await?;
        match resp {
            MessagePayload::PairResponse(pr) => Ok(pr),
            MessagePayload::Error(e) => Err(anyhow!("reflector error {}: {}", e.code, e.message)),
            other => Err(anyhow!("expected PairResponse, got {:?}", other)),
        }
    }

    /// Request a throughput session.
    pub async fn request_throughput_session(&mut self, duration_sec: u64, streams: u32, reverse: bool) -> Result<rpc::SessionGrant> {
        let req_id = self.next_id();
        let msg = LinkMessage {
            request_id: req_id.clone(),
            payload: MessagePayload::SessionRequest(rpc::SessionRequest {
                test_type: rpc::TestType::Throughput,
                params: rpc::TestParams {
                    duration_sec,
                    protocol: Some("tcp".to_string()),
                    streams: Some(streams),
                    reverse: Some(reverse),
                },
            }),
        };

        self.framed.send(msg).await.context("failed to send SessionRequest")?;

        let resp = self.expect_response(&req_id).await?;
        match resp {
            MessagePayload::SessionGrant(sg) => Ok(sg),
            MessagePayload::SessionDeny(sd) => Err(anyhow!("session denied: {:?} ({})", sd.reason, sd.message)),
            MessagePayload::Error(e) => Err(anyhow!("reflector error {}: {}", e.code, e.message)),
            other => Err(anyhow!("expected SessionGrant, got {:?}", other)),
        }
    }

    fn next_id(&mut self) -> String {
        let id = format!("req-{}", self.request_counter);
        self.request_counter += 1;
        id
    }

    async fn expect_response(&mut self, req_id: &str) -> Result<MessagePayload> {
        loop {
            let frame = self.framed.next().await
                .ok_or_else(|| anyhow!("connection closed"))?
                .context("failed to decode frame")?;

            if frame.request_id == req_id {
                return Ok(frame.payload);
            }
            // Ignore other messages (like heartbeats if we had them)
            debug!("ignoring message with id {} (waiting for {})", frame.request_id, req_id);
        }
    }
}

/// A verifier that accepts any server certificate (dangerous!).
/// Used for pairing when we don't know the server's ID yet.
#[derive(Debug)]
struct BlindVerifier;

impl ServerCertVerifier for BlindVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Accept everything.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
         Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}
