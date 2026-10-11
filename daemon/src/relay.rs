//! The computer's side of the Relay (ADR-0012, docs/relay-protocol.md): reaching the box through
//! `vibebuddy-relay` when neither the cable nor the local network does.
//!
//! Once linked, a bridge task turns the Relay's `data` messages into the same NDJSON byte stream the
//! cable carries, so the serial worker's session loop runs over it unchanged. The bridge also sends
//! WebSocket pings, gives the link up when the Relay goes quiet, and ends the stream when the box is
//! taken away.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, DuplexStream};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, warn};

use crate::identity::Identity;

/// Where the Relay is when nothing overrides it.
pub const DEFAULT_URL: &str = "wss://relay.korekore.ai";
/// Pings go out this often, and a Relay silent this long is taken for gone (docs/relay-protocol.md).
const PING_EVERY: Duration = Duration::from_secs(30);
const DEAD_AFTER: Duration = Duration::from_secs(75);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Clone)]
pub struct RelayConfig {
    pub identity: Arc<Identity>,
    /// `wss://relay.korekore.ai`, or `ws://127.0.0.1:8787` under `wrangler dev`; the update manifest may move it.
    url: Arc<std::sync::RwLock<String>>,
    /// Set by `VIBEBUDDY_RELAY_URL`, which the manifest doesn't override.
    fixed: bool,
}

impl RelayConfig {
    /// `fixed` is an address given by hand; None means the compiled default, which the manifest may move.
    pub fn new(identity: Arc<Identity>, fixed: Option<String>) -> Self {
        let url = fixed.clone().unwrap_or_else(|| DEFAULT_URL.to_owned());
        Self { identity, url: Arc::new(std::sync::RwLock::new(url)), fixed: fixed.is_some() }
    }

    pub fn url(&self) -> String {
        self.url.read().map(|url| url.clone()).unwrap_or_else(|_| DEFAULT_URL.to_owned())
    }

    /// The signed update manifest names the Relay: used from the next connection on. Only `wss://`, and never over
    /// an address given by hand.
    pub fn offer(&self, url: &str) {
        if self.fixed || !url.starts_with("wss://") {
            return;
        }
        if let Ok(mut current) = self.url.write()
            && *current != url
        {
            info!(%url, "the update manifest moves the Relay");
            *current = url.to_owned();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RelayError {
    /// Couldn't reach the Relay, or it went away.
    Unreachable(String),
    /// HTTP 429: too many connections for this box from here; wait before trying again.
    RateLimited,
    /// It refused us: `auth`, `not_paired` or another error code.
    Refused(String),
    /// The box is linked elsewhere: `usb` or `linked`, and by whom.
    Busy { reason: String, owner: String },
}

impl RelayError {
    /// How long to wait before the next try.
    pub fn backoff(&self, attempt: u32) -> Duration {
        let base = match self {
            Self::RateLimited => Duration::from_secs(30),
            Self::Refused(_) => Duration::from_secs(60),
            Self::Busy { .. } => Duration::from_secs(10),
            Self::Unreachable(_) => Duration::from_secs(2),
        };
        (base * 2u32.saturating_pow(attempt.min(5))).min(Duration::from_secs(300))
    }
}

pub fn box_url(base: &str, box_key: &[u8; 32]) -> String {
    format!("{}/v1/boxes/{}", base.trim_end_matches('/'), URL_SAFE_NO_PAD.encode(box_key))
}

pub fn signed_message(role: &str, box_id: &str, nonce: &str) -> Vec<u8> {
    format!("vibebuddy-relay-v1\n{role}\n{box_id}\n{nonce}").into_bytes()
}

/// TLS the way the rest of the daemon does it: rustls with ring and the system's certificate store.
fn connector() -> Result<Connector, RelayError> {
    use rustls_platform_verifier::BuilderVerifierExt;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| RelayError::Unreachable(error.to_string()))?
        .with_platform_verifier()
        .map_err(|error| RelayError::Unreachable(error.to_string()))?
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

/// Connects, proves this computer's key, and asks for the box. On success the returned stream carries
/// the VibeBuddy Protocol as bytes, as the cable does.
pub async fn link(config: &RelayConfig, box_key: &[u8; 32], takeover: bool) -> Result<DuplexStream, RelayError> {
    let url = box_url(&config.url(), box_key);
    let connected = tokio_tungstenite::connect_async_tls_with_config(&url, None, false, Some(connector()?));
    let (mut socket, _) = match tokio::time::timeout(HANDSHAKE_TIMEOUT, connected).await {
        Err(_) => return Err(RelayError::Unreachable("timed out".to_owned())),
        Ok(Err(tokio_tungstenite::tungstenite::Error::Http(response))) if response.status().as_u16() == 429 => {
            return Err(RelayError::RateLimited);
        }
        Ok(Err(error)) => return Err(RelayError::Unreachable(error.to_string())),
        Ok(Ok(connected)) => connected,
    };
    match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake(&mut socket, &config.identity, box_key, takeover)).await {
        Err(_) => Err(RelayError::Unreachable("handshake timed out".to_owned())),
        Ok(Err(error)) => {
            let _ = socket.close(None).await;
            Err(error)
        }
        Ok(Ok(())) => Ok(bridge(socket)),
    }
}

async fn handshake<S>(socket: &mut S, identity: &Identity, box_key: &[u8; 32], takeover: bool) -> Result<(), RelayError>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin,
{
    let box_id = URL_SAFE_NO_PAD.encode(box_key);
    let challenge = expect(socket, &["challenge"]).await?;
    let nonce = challenge["nonce"].as_str().unwrap_or_default().to_owned();
    let sig = URL_SAFE_NO_PAD.encode(identity.sign(&signed_message("computer", &box_id, &nonce)));
    send(socket, json!({"t": "hello", "role": "computer", "key": identity.public_key(), "sig": sig})).await?;
    expect(socket, &["welcome"]).await?;
    send(socket, json!({"t": if takeover { "takeover" } else { "link" }})).await?;
    let answer = expect(socket, &["linked", "busy"]).await?;
    if answer["t"] == "busy" {
        let text = |name: &str| answer[name].as_str().unwrap_or_default().to_owned();
        return Err(RelayError::Busy { reason: text("reason"), owner: text("owner") });
    }
    Ok(())
}

/// Reads until one of `wanted` arrives, skipping `status` and pings; an `error` or a close ends it.
async fn expect<S>(socket: &mut S, wanted: &[&str]) -> Result<Value, RelayError>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };
                let kind = value["t"].as_str().unwrap_or_default();
                if wanted.contains(&kind) {
                    return Ok(value);
                }
                if kind == "error" {
                    return Err(RelayError::Refused(value["code"].as_str().unwrap_or("error").to_owned()));
                }
            }
            Some(Ok(Message::Close(frame))) => return Err(closed(frame.as_ref())),
            Some(Ok(_)) => {}
            Some(Err(error)) => return Err(RelayError::Unreachable(error.to_string())),
            None => return Err(RelayError::Unreachable("closed".to_owned())),
        }
    }
}

fn closed(frame: Option<&CloseFrame>) -> RelayError {
    match frame.map(|frame| u16::from(frame.code)) {
        Some(4001) => RelayError::Refused("auth".to_owned()),
        Some(4003) => RelayError::Refused("not_paired".to_owned()),
        Some(1008) => RelayError::RateLimited,
        // Silent too long, as a sleeping laptop is: a drop like any other, and the box may still be ours.
        Some(4004) => RelayError::Unreachable("silent".to_owned()),
        Some(code) => RelayError::Unreachable(format!("closed with {code}")),
        None => RelayError::Unreachable("closed".to_owned()),
    }
}

async fn send<S>(socket: &mut S, value: Value) -> Result<(), RelayError>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    socket.send(Message::text(value.to_string())).await.map_err(|error| RelayError::Unreachable(error.to_string()))
}

/// Runs the linked socket behind a byte stream: lines written to the stream go out as `data`, `data`
/// from the box comes back as lines. The stream ends when the box is taken away, the Relay closes, or
/// it goes quiet for [`DEAD_AFTER`].
fn bridge(socket: Socket) -> DuplexStream {
    let (ours, theirs) = tokio::io::duplex(64 * 1024);
    tokio::spawn(run_bridge(socket, theirs));
    ours
}

async fn run_bridge<S>(socket: S, stream: DuplexStream)
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin,
{
    let (mut sink, mut source) = socket.split();
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = tokio::io::BufReader::new(read).lines();
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut heard = tokio::time::Instant::now();
    loop {
        tokio::select! {
            message = source.next() => {
                heard = tokio::time::Instant::now();
                match message {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };
                        match value["t"].as_str() {
                            Some("data") => {
                                let Some(line) = value["line"].as_str() else { continue };
                                if write.write_all(format!("{line}\n").as_bytes()).await.is_err() {
                                    break;
                                }
                            }
                            // Linked again while linked: the box reconnected to the Relay (it restarted, say), so
                            // the session starts over with hello, as on a new cable connection.
                            Some("linked") => {
                                let hello = vibebuddy_protocol::Event::named("device.hello");
                                let Ok(frame) = serde_json::to_string(&hello) else { continue };
                                if sink.send(Message::text(json!({"t": "data", "line": frame}).to_string())).await.is_err() {
                                    break;
                                }
                            }
                            Some("unlinked") => {
                                info!(reason = value["reason"].as_str().unwrap_or("?"), "the Relay link ended");
                                break;
                            }
                            Some("error") => warn!(code = value["code"].as_str().unwrap_or("?"), "the Relay reported an error"),
                            Some("status") => debug!(%value, "Relay status"),
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        info!(reason = ?closed(frame.as_ref()), "the Relay closed the link");
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        warn!(%error, "the Relay link failed");
                        break;
                    }
                    None => break,
                }
            }
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if sink.send(Message::text(json!({"t": "data", "line": line}).to_string())).await.is_err() {
                            break;
                        }
                    }
                    // The session let go of the stream: give the box up.
                    _ => {
                        let _ = sink.send(Message::text(json!({"t": "unlink"}).to_string())).await;
                        let _ = sink.close().await;
                        return;
                    }
                }
            }
            _ = ping.tick() => {
                if heard.elapsed() >= DEAD_AFTER {
                    warn!("the Relay went quiet, reconnecting");
                    break;
                }
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
        }
    }
    let _ = sink.close().await;
    // Dropping the write half ends the stream for the session, which then reconnects.
    let _ = write.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio_tungstenite::accept_async;

    fn identity() -> Identity {
        let mut suffix = [0u8; 4];
        getrandom::fill(&mut suffix).unwrap();
        let dir = std::env::temp_dir().join(format!("vibebuddy-relay-test-{}-{}", std::process::id(), u32::from_le_bytes(suffix)));
        Identity::load_or_create(&dir.join("identity"), "test".to_owned()).unwrap()
    }

    /// A Relay that checks the hello, then answers link with `answer` and echoes data back in upper case.
    async fn fake_relay(answer: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let say = |value: Value| Message::text(value.to_string());
            socket.send(say(json!({"t": "challenge", "nonce": "abc"}))).await.unwrap();
            let Some(Ok(Message::Text(hello))) = socket.next().await else { return };
            let hello: Value = serde_json::from_str(&hello).unwrap();
            let key = URL_SAFE_NO_PAD.decode(hello["key"].as_str().unwrap()).unwrap();
            let sig = URL_SAFE_NO_PAD.decode(hello["sig"].as_str().unwrap()).unwrap();
            let verifying = ed25519_dalek::VerifyingKey::from_bytes(&key.try_into().unwrap()).unwrap();
            let box_id = URL_SAFE_NO_PAD.encode([5u8; 32]);
            let valid = verifying
                .verify_strict(&signed_message("computer", &box_id, "abc"), &ed25519_dalek::Signature::from_bytes(&sig.try_into().unwrap()))
                .is_ok();
            if !valid {
                socket.send(say(json!({"t": "error", "code": "auth"}))).await.unwrap();
                return;
            }
            socket.send(say(json!({"t": "welcome"}))).await.unwrap();
            socket.send(say(json!({"t": "status", "box": true, "owner": null}))).await.unwrap();
            let Some(Ok(Message::Text(_link))) = socket.next().await else { return };
            match answer {
                "busy" => {
                    socket.send(say(json!({"t": "busy", "reason": "usb", "owner": "omarchy"}))).await.unwrap();
                }
                _ => {
                    socket.send(say(json!({"t": "linked"}))).await.unwrap();
                    while let Some(Ok(message)) = socket.next().await {
                        let Message::Text(text) = message else { continue };
                        let value: Value = serde_json::from_str(&text).unwrap();
                        if value["t"] == "data" {
                            let line = value["line"].as_str().unwrap();
                            if line.contains("device.hello") && line.contains("\"version\"") && !line.contains("VERSION") && answer == "restart" {
                                // The bridge said hello again after the box came back: end the test there.
                                socket.send(say(json!({"t": "data", "line": "REHELLO"}))).await.unwrap();
                                socket.send(say(json!({"t": "unlinked", "reason": "takeover"}))).await.unwrap();
                                continue;
                            }
                            socket.send(say(json!({"t": "data", "line": line.to_uppercase()}))).await.unwrap();
                            if answer == "restart" {
                                socket.send(say(json!({"t": "linked"}))).await.unwrap();
                            } else {
                                socket.send(say(json!({"t": "unlinked", "reason": "takeover"}))).await.unwrap();
                            }
                        }
                    }
                }
            }
        });
        format!("ws://{address}")
    }

    #[tokio::test]
    async fn lines_go_through_the_relay_both_ways_until_the_box_is_taken() {
        let config = RelayConfig::new(Arc::new(identity()), Some(fake_relay("linked").await));
        let mut stream = link(&config, &[5; 32], false).await.unwrap();
        stream.write_all(b"{\"version\":1,\"event\":\"device.hello\"}\n").await.unwrap();
        let mut received = String::new();
        stream.read_to_string(&mut received).await.unwrap();
        assert_eq!(received, "{\"VERSION\":1,\"EVENT\":\"DEVICE.HELLO\"}\n", "and then the stream ends on unlinked");
    }

    #[tokio::test]
    async fn the_box_coming_back_starts_the_session_over() {
        let config = RelayConfig::new(Arc::new(identity()), Some(fake_relay("restart").await));
        let mut stream = link(&config, &[5; 32], false).await.unwrap();
        stream.write_all(b"{\"version\":1,\"event\":\"x\"}\n").await.unwrap();
        let mut received = String::new();
        stream.read_to_string(&mut received).await.unwrap();
        assert!(received.ends_with("REHELLO\n"), "hello went out again after linked: {received}");
    }

    #[tokio::test]
    async fn busy_names_the_owner() {
        let config = RelayConfig::new(Arc::new(identity()), Some(fake_relay("busy").await));
        assert_eq!(
            link(&config, &[5; 32], false).await.err(),
            Some(RelayError::Busy { reason: "usb".to_owned(), owner: "omarchy".to_owned() })
        );
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        let error = RelayError::Unreachable(String::new());
        assert_eq!(error.backoff(0), Duration::from_secs(2));
        assert_eq!(error.backoff(1), Duration::from_secs(4));
        assert_eq!(RelayError::RateLimited.backoff(9), Duration::from_secs(300));
    }

    #[test]
    fn the_manifest_moves_the_relay_unless_it_was_given_by_hand() {
        let config = RelayConfig::new(Arc::new(identity()), None);
        assert_eq!(config.url(), DEFAULT_URL);
        config.offer("ws://plain.example");
        assert_eq!(config.url(), DEFAULT_URL, "never off TLS");
        config.offer("wss://relay2.korekore.ai");
        assert_eq!(config.url(), "wss://relay2.korekore.ai");
        let fixed = RelayConfig::new(Arc::new(identity()), Some("ws://127.0.0.1:8787".to_owned()));
        fixed.offer("wss://relay2.korekore.ai");
        assert_eq!(fixed.url(), "ws://127.0.0.1:8787");
    }

    #[test]
    fn the_url_names_the_box() {
        assert_eq!(box_url("wss://relay.korekore.ai/", &[0; 32]), "wss://relay.korekore.ai/v1/boxes/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    }
}
