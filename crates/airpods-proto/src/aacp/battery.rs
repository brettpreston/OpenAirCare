//! Battery report (opcode 0x04).
//!
//! Payload (starting at the opcode byte): `04 00 <count>` then `count`
//! entries of 5 bytes: `<component> <?> <level> <status> <?>`.

use super::{frame, opcode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryComponent {
    Headphone,
    Right,
    Left,
    Case,
    Unknown(u8),
}

impl BatteryComponent {
    fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::Headphone,
            0x02 => Self::Right,
            0x04 => Self::Left,
            0x08 => Self::Case,
            other => Self::Unknown(other),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Headphone => "Headphone",
            Self::Right => "Right",
            Self::Left => "Left",
            Self::Case => "Case",
            Self::Unknown(_) => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    NotCharging,
    Disconnected,
    Unknown(u8),
}

impl BatteryStatus {
    fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::Charging,
            0x02 => Self::NotCharging,
            0x04 => Self::Disconnected,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryInfo {
    pub component: BatteryComponent,
    pub level: u8,
    pub status: BatteryStatus,
}

/// Parse a framed battery packet. Returns `None` if it is not a battery
/// packet or is malformed.
pub fn parse(pkt: &[u8]) -> Option<Vec<BatteryInfo>> {
    let payload = frame::payload(pkt)?;
    if payload[0] != opcode::BATTERY_INFO || payload.len() < 3 {
        return None;
    }
    let count = payload[2] as usize;
    if payload.len() < 3 + count * 5 {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = 3 + i * 5;
        out.push(BatteryInfo {
            component: BatteryComponent::from_u8(payload[base]),
            level: payload[base + 2],
            status: BatteryStatus::from_u8(payload[base + 3]),
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_components() {
        // header, opcode 04 00, count 3, then L / R / Case entries
        let pkt = [
            0x04, 0x00, 0x04, 0x00, 0x04, 0x00, 0x03, //
            0x04, 0x01, 0x5A, 0x02, 0x01, // left 90% not charging
            0x02, 0x01, 0x64, 0x01, 0x01, // right 100% charging
            0x08, 0x01, 0x32, 0x04, 0x01, // case 50% disconnected
        ];
        let b = parse(&pkt).unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].component, BatteryComponent::Left);
        assert_eq!(b[0].level, 90);
        assert_eq!(b[1].status, BatteryStatus::Charging);
        assert_eq!(b[2].component, BatteryComponent::Case);
        assert_eq!(b[2].status, BatteryStatus::Disconnected);
    }

    #[test]
    fn rejects_short() {
        let pkt = [0x04, 0x00, 0x04, 0x00, 0x04, 0x00, 0x02, 0x04, 0x01];
        assert!(parse(&pkt).is_none());
    }
}
