//! AACP (Apple Accessory Communication Protocol) framing and packet codecs.

pub mod battery;
pub mod control;
pub mod frame;
pub mod handshake;
pub mod info;

/// L2CAP PSM used by AACP.
pub const PSM: u16 = 0x1001;

/// SDP service UUID advertised by AirPods for the AACP channel. Used to find
/// the AirPods among bonded devices.
pub const SERVICE_UUID: &str = "74ec2172-0bad-4d01-8f77-997b2be0722a";

/// AACP opcodes (low byte; the wire opcode is little-endian u16).
pub mod opcode {
    pub const BATTERY_INFO: u8 = 0x04;
    pub const EAR_DETECTION: u8 = 0x06;
    pub const CONTROL_COMMAND: u8 = 0x09;
    pub const REQUEST_NOTIFICATIONS: u8 = 0x0F;
    pub const STEM_PRESS: u8 = 0x19;
    pub const RENAME: u8 = 0x1A;
    pub const INFORMATION: u8 = 0x1D;
    pub const HOST_CAPABILITIES: u8 = 0x29;
    pub const CONNECTED_DEVICES: u8 = 0x2E;
    pub const PROXIMITY_KEYS_REQ: u8 = 0x30;
    pub const PROXIMITY_KEYS_RSP: u8 = 0x31;
    pub const CONVERSATION_AWARENESS: u8 = 0x4B;
    pub const SET_FEATURE_FLAGS: u8 = 0x4D;
    pub const EQ_DATA: u8 = 0x53;
}
