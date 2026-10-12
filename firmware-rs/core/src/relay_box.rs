//! The box's side of the Relay protocol (docs/relay-protocol.md), without the socket: what to answer,
//! what to report and which lines to act on. The device's Relay task and the simulator's `--relay`
//! mode each move text frames; this decides what is in them.
//!
//! The box signs the challenge with its key, then keeps the Relay told who it is paired with and
//! what it is linked to on its own (`linked` via usb or lan, or `unlinked`), and carries a computer's
//! lines once the Relay links one to it.

use alloc::string::String;
use alloc::vec::Vec;

use serde_json::{Value, json};

use crate::firmware::{Board, Firmware, LocalLink};
use crate::link;

/// What the box last told the Relay, so it says only what changed.
#[derive(Default)]
pub struct RelayBox {
    welcomed: bool,
    paired: Option<Vec<(String, String)>>,
    local: Option<LocalLink>,
    /// A computer is linked through the Relay: the box's output goes to it.
    peer: bool,
}

/// The Relay protocol version the box speaks (docs/relay-protocol.md).
pub const PROTOCOL_VERSION: u32 = 1;

/// The exact text the box sends every 30 s as a sign of life; the Relay answers it without waking.
pub const PING: &str = r#"{"t":"ping"}"#;

/// What the box signs to prove its key (docs/relay-protocol.md).
pub fn signed_message(role: &str, box_id: &str, nonce: &str) -> Vec<u8> {
    alloc::format!("vibebuddy-relay-v1\n{role}\n{box_id}\n{nonce}").into_bytes()
}

fn text(value: Value) -> String {
    serde_json::to_string(&value).unwrap_or_default()
}

impl RelayBox {
    pub fn new() -> Self {
        Self::default()
    }

    /// A computer is linked through the Relay.
    pub fn has_peer(&self) -> bool {
        self.peer
    }

    /// A new socket's challenge: the hello to send, or None without a key.
    pub fn hello(&mut self, firmware: &Firmware, nonce: &str) -> Option<String> {
        *self = Self::new();
        let link = firmware.link()?;
        let box_id = link::encode(&link.public_key());
        let sig = link.sign(&signed_message("box", &box_id, nonce));
        Some(text(json!({"t": "hello", "role": "box", "key": box_id, "sig": link::encode(&sig), "v": PROTOCOL_VERSION})))
    }

    /// A message from the Relay other than the challenge.
    pub fn message<B: Board>(&mut self, firmware: &mut Firmware, board: &mut B, value: &Value) {
        match value["t"].as_str() {
            Some("welcome") => self.welcomed = true,
            Some("linked") => {
                let key = value["computer"].as_str().and_then(link::decode_key);
                firmware.relay_linked(board.now_ms(), key);
                self.peer = key.is_some();
            }
            Some("unlinked") => self.gone(firmware, board.now_ms()),
            Some("data") => {
                if let Some(line) = value["line"].as_str() {
                    firmware.relay_line(board, line.as_bytes());
                }
            }
            _ => {}
        }
    }

    /// The socket closed: nobody is linked through it, and everything is said again on the next welcome.
    pub fn gone(&mut self, firmware: &mut Firmware, now: u32) {
        firmware.relay_linked(now, None);
        *self = Self::new();
    }

    /// What has changed since the last report: the paired list, and what the box is linked to.
    pub fn report(&mut self, firmware: &Firmware, now: u32) -> Vec<String> {
        let mut out = Vec::new();
        if !self.welcomed {
            return out;
        }
        let Some(link) = firmware.link() else { return out };
        let paired: Vec<(String, String)> = link.paired.iter().map(|paired| (link::encode(&paired.key), paired.name.clone())).collect();
        if self.paired.as_ref() != Some(&paired) {
            let computers: Vec<Value> = paired.iter().map(|(key, name)| json!({"key": key, "name": name})).collect();
            out.push(text(json!({"t": "paired", "computers": computers})));
            self.paired = Some(paired);
        }
        let local = firmware.local_link(now);
        if self.local != Some(local) {
            // Nothing to undo on a fresh socket that was never linked.
            if self.local.is_some() || local != LocalLink::None {
                out.push(text(match local {
                    LocalLink::Usb(key) => json!({"t": "linked", "via": "usb", "computer": key.map(|key| link::encode(&key))}),
                    LocalLink::Lan(key) => json!({"t": "linked", "via": "lan", "computer": link::encode(&key)}),
                    LocalLink::None => json!({"t": "unlinked"}),
                }));
            }
            self.local = Some(local);
        }
        out
    }
}

/// The box's output on its way to a computer linked through the Relay: one data message per whole line.
/// Kept apart from [`RelayBox`] because the board owns it: the firmware writes through the board.
#[derive(Default)]
pub struct RelayLines {
    partial: Vec<u8>,
}

impl RelayLines {
    pub fn lines(&mut self, bytes: &[u8]) -> Vec<String> {
        self.partial.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(end) = self.partial.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.partial.drain(..=end).collect();
            out.push(text(json!({"t": "data", "line": String::from_utf8_lossy(&line[..end])})));
        }
        out
    }

    pub fn clear(&mut self) {
        self.partial.clear();
    }
}
