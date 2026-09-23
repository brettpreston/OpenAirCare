//! AACP packet framing.
//!
//! Every AACP packet after the handshake starts with the 4-byte header
//! `04 00 04 00`, followed by a little-endian u16 opcode and the payload.
//! One L2CAP SDU normally carries exactly one packet, but control-command
//! echoes have been observed concatenated in a single SDU, so
//! [`split_packets`] scans for repeated headers.

/// Fixed AACP header.
pub const HEADER: [u8; 4] = [0x04, 0x00, 0x04, 0x00];

/// Build a packet: header + opcode (LE16) + payload.
pub fn build(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + payload.len());
    out.extend_from_slice(&HEADER);
    out.push(opcode);
    out.push(0x00);
    out.extend_from_slice(payload);
    out
}

/// Returns true if `pkt` starts with the AACP header.
pub fn has_header(pkt: &[u8]) -> bool {
    pkt.len() >= HEADER.len() && pkt[..HEADER.len()] == HEADER
}

/// Opcode (low byte) of a framed packet, if it is long enough.
pub fn opcode(pkt: &[u8]) -> Option<u8> {
    if has_header(pkt) && pkt.len() >= 6 {
        Some(pkt[4])
    } else {
        None
    }
}

/// Payload after the header + opcode. Convention (same as the reference
/// implementations): the returned slice starts *at* the opcode byte, so
/// `payload[0]` is the opcode and `payload[2..]` the data.
pub fn payload(pkt: &[u8]) -> Option<&[u8]> {
    if has_header(pkt) && pkt.len() >= 6 {
        Some(&pkt[4..])
    } else {
        None
    }
}

/// Split an SDU that may contain several concatenated framed packets.
/// A packet boundary is wherever the header re-appears (at index >= 4).
/// Packets without a leading header are returned as a single item so the
/// caller can log/ignore them.
pub fn split_packets(sdu: &[u8]) -> Vec<&[u8]> {
    if !has_header(sdu) {
        return vec![sdu];
    }
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = HEADER.len();
    while i + HEADER.len() <= sdu.len() {
        if sdu[i..i + HEADER.len()] == HEADER {
            out.push(&sdu[start..i]);
            start = i;
            i += HEADER.len();
        } else {
            i += 1;
        }
    }
    out.push(&sdu[start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_adds_header_and_opcode() {
        let p = build(0x0F, &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(p, [0x04, 0x00, 0x04, 0x00, 0x0F, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(opcode(&p), Some(0x0F));
        assert_eq!(payload(&p).unwrap()[0], 0x0F);
    }

    #[test]
    fn split_handles_single_and_concatenated() {
        let a = build(0x09, &[0x2C, 0x01, 0x01, 0x00, 0x00]);
        let b = build(0x09, &[0x33, 0x01, 0x00, 0x00, 0x00]);
        let joined = [a.clone(), b.clone()].concat();
        let parts = split_packets(&joined);
        assert_eq!(parts, vec![a.as_slice(), b.as_slice()]);
        assert_eq!(split_packets(&a), vec![a.as_slice()]);
    }

    #[test]
    fn split_passes_through_unframed() {
        let raw = [0x01, 0x02, 0x03];
        assert_eq!(split_packets(&raw), vec![&raw[..]]);
    }
}
