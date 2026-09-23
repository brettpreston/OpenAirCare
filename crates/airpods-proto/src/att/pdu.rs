//! ATT PDU building and parsing (subset used by the hearing-aid feature).

pub const OP_ERROR_RSP: u8 = 0x01;
pub const OP_EXCHANGE_MTU_REQ: u8 = 0x02;
pub const OP_EXCHANGE_MTU_RSP: u8 = 0x03;
pub const OP_READ_REQ: u8 = 0x0A;
pub const OP_READ_RSP: u8 = 0x0B;
pub const OP_WRITE_REQ: u8 = 0x12;
pub const OP_WRITE_RSP: u8 = 0x13;
pub const OP_HANDLE_VALUE_NTF: u8 = 0x1B;

/// `0A <handle LE16>`
pub fn read_req(handle: u16) -> [u8; 3] {
    let [lo, hi] = handle.to_le_bytes();
    [OP_READ_REQ, lo, hi]
}

/// `12 <handle LE16> <value>`
pub fn write_req(handle: u16, value: &[u8]) -> Vec<u8> {
    let [lo, hi] = handle.to_le_bytes();
    let mut out = Vec::with_capacity(3 + value.len());
    out.extend_from_slice(&[OP_WRITE_REQ, lo, hi]);
    out.extend_from_slice(value);
    out
}

/// Write `01 00` to the CCCD handle (`characteristic handle + 1`).
pub fn cccd_enable(cccd_handle: u16) -> Vec<u8> {
    write_req(cccd_handle, &[0x01, 0x00])
}

/// `02 <mtu LE16>`
pub fn exchange_mtu_req(mtu: u16) -> [u8; 3] {
    let [lo, hi] = mtu.to_le_bytes();
    [OP_EXCHANGE_MTU_REQ, lo, hi]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttPdu {
    ReadRsp(Vec<u8>),
    WriteRsp,
    Notification { handle: u16, value: Vec<u8> },
    Error { request_opcode: u8, handle: u16, code: u8 },
    ExchangeMtuRsp(u16),
    /// Any other opcode; the full PDU is kept for logging.
    Other(Vec<u8>),
}

impl AttPdu {
    /// True for PDUs that answer a request (i.e. not a notification).
    pub fn is_response(&self) -> bool {
        !matches!(self, AttPdu::Notification { .. })
    }
}

/// Parse an inbound ATT PDU.
pub fn parse(pdu: &[u8]) -> Option<AttPdu> {
    let (&op, rest) = pdu.split_first()?;
    Some(match op {
        OP_READ_RSP => AttPdu::ReadRsp(rest.to_vec()),
        OP_WRITE_RSP => AttPdu::WriteRsp,
        OP_HANDLE_VALUE_NTF if rest.len() >= 2 => AttPdu::Notification {
            handle: u16::from_le_bytes([rest[0], rest[1]]),
            value: rest[2..].to_vec(),
        },
        OP_ERROR_RSP if rest.len() >= 4 => AttPdu::Error {
            request_opcode: rest[0],
            handle: u16::from_le_bytes([rest[1], rest[2]]),
            code: rest[3],
        },
        OP_EXCHANGE_MTU_RSP if rest.len() >= 2 => {
            AttPdu::ExchangeMtuRsp(u16::from_le_bytes([rest[0], rest[1]]))
        }
        _ => AttPdu::Other(pdu.to_vec()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::att::handles::*;

    #[test]
    fn builds_requests() {
        assert_eq!(read_req(HEARING_AID), [0x0A, 0x2A, 0x00]);
        assert_eq!(write_req(HEARING_AID, &[0xAA]), vec![0x12, 0x2A, 0x00, 0xAA]);
        assert_eq!(cccd_enable(HEARING_AID_CCCD), vec![0x12, 0x2B, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn parses_responses() {
        assert_eq!(parse(&[0x0B, 1, 2, 3]), Some(AttPdu::ReadRsp(vec![1, 2, 3])));
        assert_eq!(parse(&[0x13]), Some(AttPdu::WriteRsp));
        assert_eq!(
            parse(&[0x1B, 0x2A, 0x00, 9, 9]),
            Some(AttPdu::Notification { handle: 0x2A, value: vec![9, 9] })
        );
        assert_eq!(
            parse(&[0x01, 0x12, 0x2A, 0x00, 0x03]),
            Some(AttPdu::Error { request_opcode: 0x12, handle: 0x2A, code: 0x03 })
        );
        assert_eq!(parse(&[]), None);
        assert!(matches!(parse(&[0x99, 1]), Some(AttPdu::Other(_))));
    }
}
