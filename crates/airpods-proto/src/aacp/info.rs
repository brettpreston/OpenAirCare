//! Device information (opcode 0x1D), sent by the AirPods once after connect.
//! After 4 payload bytes the data is a sequence of NUL-separated strings:
//! name, model number, manufacturer, serial, version1, version2, hardware
//! revision, updater id, left serial, right serial, version3.

use super::{frame, opcode};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: String,
    pub model_number: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub firmware_version: String,
    pub raw_strings: Vec<String>,
}

/// Parse a framed information packet.
pub fn parse(pkt: &[u8]) -> Option<DeviceInfo> {
    let payload = frame::payload(pkt)?;
    if payload[0] != opcode::INFORMATION || payload.len() < 6 {
        return None;
    }
    let data = &payload[4..];
    // The reference implementation skips the first (binary) run up to the
    // first NUL and then drops the first collected string.
    let mut strings: Vec<String> = data
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    if strings.is_empty() {
        return None;
    }
    strings.remove(0);
    let get = |i: usize| strings.get(i).cloned().unwrap_or_default();
    Some(DeviceInfo {
        name: get(0),
        model_number: get(1),
        manufacturer: get(2),
        serial_number: get(3),
        firmware_version: get(4),
        raw_strings: strings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strings() {
        let mut p = vec![0x1D, 0x00, 0x00, 0x00, 0x00, 0x00];
        p.extend_from_slice(b"\x01\x02\x00My AirPods\x00A3048\x00Apple Inc.\x00SN123\x007B21\x00");
        let pkt = [frame::HEADER.to_vec(), p].concat();
        let info = parse(&pkt).unwrap();
        assert_eq!(info.name, "My AirPods");
        assert_eq!(info.model_number, "A3048");
        assert_eq!(info.manufacturer, "Apple Inc.");
        assert_eq!(info.serial_number, "SN123");
        assert_eq!(info.firmware_version, "7B21");
    }
}
