//! Connection bring-up packets. AirPods ignore everything until the raw
//! handshake has been sent; the following packets enable notifications and
//! advertise host capabilities. Order and delays mirror the reference
//! implementations (`linux-rust/src/devices/airpods.rs`, Kotlin
//! `AirPodsService.connectToSocket`).

use core::time::Duration;

/// Raw handshake. NOTE: this one is *not* prefixed with the AACP header.
pub const HANDSHAKE: [u8; 16] = [
    0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// "Set specific features" / host feature flags (opcode 0x4D).
pub const SET_FEATURE_FLAGS: [u8; 14] = [
    0x04, 0x00, 0x04, 0x00, 0x4D, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Request all notifications (opcode 0x0F).
pub const REQUEST_NOTIFICATIONS: [u8; 10] =
    [0x04, 0x00, 0x04, 0x00, 0x0F, 0x00, 0xFF, 0xFF, 0xFF, 0xFF];

/// Host capabilities (opcode 0x29). The reference code calls this
/// "some packet, enables setting EQ".
pub const HOST_CAPABILITIES: [u8; 14] = [
    0x04, 0x00, 0x04, 0x00, 0x29, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// Prefix of the handshake acknowledgement sent by the AirPods. AirPods Pro 3
/// (firmware 8A) send `01 00 04 00 00 00 01 00 03 00 ...` (18 bytes), older
/// captures show the bare 4 bytes, so match with `starts_with`.
pub const HANDSHAKE_ACK: [u8; 4] = [0x01, 0x00, 0x04, 0x00];

/// The bring-up sequence: each packet and the delay to wait *after* it.
pub fn sequence() -> [(&'static [u8], Duration); 4] {
    [
        (&HANDSHAKE, Duration::from_millis(300)),
        (&SET_FEATURE_FLAGS, Duration::from_millis(300)),
        (&REQUEST_NOTIFICATIONS, Duration::from_millis(100)),
        (&HOST_CAPABILITIES, Duration::from_millis(100)),
    ]
}
