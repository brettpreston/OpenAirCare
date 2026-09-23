//! Raw ATT (Attribute Protocol) over a classic L2CAP channel. AirPods expose
//! the accessibility characteristics on fixed handles, so no GATT discovery
//! is performed.

pub mod handles;
pub mod pdu;

/// L2CAP PSM of the ATT channel.
pub const PSM: u16 = 0x001F;

pub use handles::*;
pub use pdu::{cccd_enable, parse, read_req, write_req, AttPdu};
