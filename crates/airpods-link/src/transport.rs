//! Transport abstraction: something that can find AirPods and open a
//! packet-oriented (L2CAP SEQPACKET) channel to a given PSM.
//!
//! A transport pumps the socket into a pair of tokio channels so that the
//! session loop can `select!` over several links without caring how the
//! bytes move. The `rx` side closes when the remote hangs up.

use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("no paired AirPods found (pair them via Bluetooth settings first)")]
    NoDevice,
    #[error("adapter error: {0}")]
    Adapter(String),
    #[error("connect to PSM {psm:#06x} failed: {msg}")]
    Connect { psm: u16, msg: String },
    #[error("connect to PSM {0:#06x} timed out")]
    Timeout(u16),
    #[error("link closed")]
    Closed,
    #[error("ATT request timed out")]
    AttTimeout,
    #[error("ATT error response: request {request_opcode:#04x} handle {handle:#06x} code {code:#04x}")]
    AttError { request_opcode: u8, handle: u16, code: u8 },
    #[error("not connected")]
    NotConnected,
    #[error("{0}")]
    Other(String),
}

/// A bonded device as seen by the transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    /// Advertises the AACP service UUID.
    pub is_airpods: bool,
}

/// One open L2CAP channel.
pub struct Link {
    pub tx: mpsc::Sender<Vec<u8>>,
    pub rx: mpsc::Receiver<Vec<u8>>,
}

impl Link {
    pub async fn send(&self, sdu: &[u8]) -> Result<(), LinkError> {
        self.tx.send(sdu.to_vec()).await.map_err(|_| LinkError::Closed)
    }
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Transport: Send + Sync {
    /// Human readable backend name for the UI ("BlueZ", "mock").
    fn name(&self) -> &'static str;

    /// List bonded devices, flagging AirPods (AACP UUID present).
    fn devices(&self) -> BoxFuture<'_, Result<Vec<DeviceEntry>, LinkError>>;

    /// Open an L2CAP SEQPACKET channel to `mac` on `psm`.
    fn connect<'a>(&'a self, mac: &'a str, psm: u16) -> BoxFuture<'a, Result<Link, LinkError>>;

    /// Drop the whole ACL link to `mac` (all channels). AirPods keep
    /// refusing PSM 31 once they closed it themselves until the link is
    /// re-established; the session calls this as a last resort.
    fn bounce<'a>(&'a self, mac: &'a str) -> BoxFuture<'a, Result<(), LinkError>>;
}
