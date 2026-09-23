//! Control commands (opcode 0x09).
//!
//! Wire form is always 11 bytes:
//! `04 00 04 00 09 00 <id> <d1> <d2> <d3> <d4>`.
//! The device echoes the same frame back whenever a value changes, which is
//! how the current state is learned.

use super::frame;
use super::opcode;

/// Control command identifiers (from `docs/control_commands.md`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlCommandId {
    MicMode = 0x01,
    ButtonSendMode = 0x05,
    OwnsConnection = 0x06,
    EarDetectionConfig = 0x0A,
    ListeningMode = 0x0D,
    VoiceTrigger = 0x12,
    SingleClickMode = 0x14,
    DoubleClickMode = 0x15,
    ClickHoldMode = 0x16,
    DoubleClickInterval = 0x17,
    ClickHoldInterval = 0x18,
    ListeningModeConfigs = 0x1A,
    OneBudAncMode = 0x1B,
    CrownRotationDirection = 0x1C,
    AutoAnswerMode = 0x1E,
    ChimeVolume = 0x1F,
    AutomaticConnectionConfig = 0x20,
    VolumeSwipeInterval = 0x23,
    CallManagementConfig = 0x24,
    VolumeSwipeMode = 0x25,
    AdaptiveVolumeConfig = 0x26,
    SoftwareMuteConfig = 0x27,
    ConversationDetectConfig = 0x28,
    Ssl = 0x29,
    /// Two bytes: `[enrolled, enabled]`, `0x01` = true, `0x02` = false.
    HearingAid = 0x2C,
    AutoAncStrength = 0x2E,
    /// "Swipe to control amplification": `0x01` on, `0x02` off.
    HpsGainSwipe = 0x2F,
    HrmState = 0x30,
    InCaseToneConfig = 0x31,
    SiriMultitoneConfig = 0x32,
    /// Hearing assist config: `0x01` on, `0x02` off.
    HearingAssistConfig = 0x33,
    AllowOffOption = 0x34,
    SleepDetectionConfig = 0x35,
    AllowAutoConnect = 0x36,
    PpeToggleConfig = 0x37,
    PpeCapLevelConfig = 0x38,
    StemConfig = 0x39,
    HearingAidGenericConfig = 0x3D,
    UplinkEqBudConfig = 0x3E,
    UplinkEqSourceConfig = 0x3F,
}

impl ControlCommandId {
    pub fn from_u8(v: u8) -> Option<Self> {
        use ControlCommandId::*;
        Some(match v {
            0x01 => MicMode,
            0x05 => ButtonSendMode,
            0x06 => OwnsConnection,
            0x0A => EarDetectionConfig,
            0x0D => ListeningMode,
            0x12 => VoiceTrigger,
            0x14 => SingleClickMode,
            0x15 => DoubleClickMode,
            0x16 => ClickHoldMode,
            0x17 => DoubleClickInterval,
            0x18 => ClickHoldInterval,
            0x1A => ListeningModeConfigs,
            0x1B => OneBudAncMode,
            0x1C => CrownRotationDirection,
            0x1E => AutoAnswerMode,
            0x1F => ChimeVolume,
            0x20 => AutomaticConnectionConfig,
            0x23 => VolumeSwipeInterval,
            0x24 => CallManagementConfig,
            0x25 => VolumeSwipeMode,
            0x26 => AdaptiveVolumeConfig,
            0x27 => SoftwareMuteConfig,
            0x28 => ConversationDetectConfig,
            0x29 => Ssl,
            0x2C => HearingAid,
            0x2E => AutoAncStrength,
            0x2F => HpsGainSwipe,
            0x30 => HrmState,
            0x31 => InCaseToneConfig,
            0x32 => SiriMultitoneConfig,
            0x33 => HearingAssistConfig,
            0x34 => AllowOffOption,
            0x35 => SleepDetectionConfig,
            0x36 => AllowAutoConnect,
            0x37 => PpeToggleConfig,
            0x38 => PpeCapLevelConfig,
            0x39 => StemConfig,
            0x3D => HearingAidGenericConfig,
            0x3E => UplinkEqBudConfig,
            0x3F => UplinkEqSourceConfig,
            _ => return None,
        })
    }
}

/// Listening (noise control) mode values for `ControlCommandId::ListeningMode`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListeningMode {
    Off = 0x01,
    NoiseCancellation = 0x02,
    Transparency = 0x03,
    Adaptive = 0x04,
}

impl ListeningMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::Off,
            0x02 => Self::NoiseCancellation,
            0x03 => Self::Transparency,
            0x04 => Self::Adaptive,
            _ => return None,
        })
    }

    pub const ALL: [ListeningMode; 4] = [
        Self::Off,
        Self::NoiseCancellation,
        Self::Transparency,
        Self::Adaptive,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::NoiseCancellation => "Noise Cancellation",
            Self::Transparency => "Transparency",
            Self::Adaptive => "Adaptive",
        }
    }
}

/// Boolean encoding used by most control commands.
pub const ON: u8 = 0x01;
pub const OFF: u8 = 0x02;

pub fn bool_byte(v: bool) -> u8 {
    if v {
        ON
    } else {
        OFF
    }
}

/// A received (or to-be-sent) control command with its trimmed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCommand {
    pub id: ControlCommandId,
    /// Value bytes with trailing zeros trimmed (at least one byte).
    pub value: Vec<u8>,
}

/// Build the 11-byte wire frame for a control command. `data` may be
/// shorter than 4 bytes; it is zero padded.
pub fn build(id: ControlCommandId, data: &[u8]) -> [u8; 11] {
    let mut out = [0u8; 11];
    out[..4].copy_from_slice(&frame::HEADER);
    out[4] = opcode::CONTROL_COMMAND;
    out[5] = 0x00;
    out[6] = id as u8;
    for (i, b) in data.iter().take(4).enumerate() {
        out[7 + i] = *b;
    }
    out
}

/// Parse a framed control-command packet (header + opcode 0x09).
/// Returns `None` for other opcodes, short packets or unknown identifiers
/// (unknown ids are returned as `Err(raw_id)` via [`parse_raw`]).
pub fn parse(pkt: &[u8]) -> Option<ControlCommand> {
    match parse_raw(pkt)? {
        (Some(id), value) => Some(ControlCommand { id, value }),
        (None, _) => None,
    }
}

/// Like [`parse`] but also returns values for unknown identifiers as
/// `(None, value)`; the raw id can be read from `pkt[6]`.
pub fn parse_raw(pkt: &[u8]) -> Option<(Option<ControlCommandId>, Vec<u8>)> {
    let payload = frame::payload(pkt)?;
    if payload[0] != opcode::CONTROL_COMMAND || payload.len() < 7 {
        return None;
    }
    let id = ControlCommandId::from_u8(payload[2]);
    let value_bytes = &payload[3..7];
    let value = match value_bytes.iter().rposition(|&b| b != 0) {
        Some(i) => value_bytes[..=i].to_vec(),
        None => vec![0],
    };
    Some((id, value))
}

// ---- Hearing-aid specific helpers -------------------------------------

/// Data bytes for `HearingAid` (0x2C): `[enrolled, enabled]`. The reference
/// Android app always keeps `enrolled = 0x01` and toggles `enabled`.
pub fn hearing_aid(enabled: bool) -> [u8; 4] {
    [ON, bool_byte(enabled), 0, 0]
}

/// Data bytes for `HearingAssistConfig` (0x33).
pub fn hearing_assist(enabled: bool) -> [u8; 4] {
    [bool_byte(enabled), 0, 0, 0]
}

/// Data bytes for `HpsGainSwipe` (0x2F).
pub fn gain_swipe(enabled: bool) -> [u8; 4] {
    [bool_byte(enabled), 0, 0, 0]
}

/// Data bytes for `ListeningMode` (0x0D).
pub fn listening_mode(mode: ListeningMode) -> [u8; 4] {
    [mode as u8, 0, 0, 0]
}

/// Whether hearing aid is currently on, given the last echoed values of
/// `HearingAid` (0x2C) and `HearingAssistConfig` (0x33). Mirrors the
/// Android app: `aid[1] == 0x01 && assist[0] == 0x01`.
pub fn is_hearing_aid_enabled(hearing_aid: Option<&[u8]>, assist: Option<&[u8]>) -> bool {
    let aid_on = hearing_aid.and_then(|v| v.get(1)).copied() == Some(ON);
    let assist_on = assist.and_then(|v| v.first()).copied() == Some(ON);
    aid_on && assist_on
}

/// Whether the AirPods report hearing aid as *enrolled* (first byte of 0x2C).
pub fn is_hearing_aid_enrolled(hearing_aid: Option<&[u8]>) -> bool {
    hearing_aid.and_then(|v| v.first()).copied() == Some(ON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hearing_aid_enable_frame() {
        let f = build(ControlCommandId::HearingAid, &hearing_aid(true));
        assert_eq!(
            f,
            [0x04, 0x00, 0x04, 0x00, 0x09, 0x00, 0x2C, 0x01, 0x01, 0x00, 0x00]
        );
        let f = build(ControlCommandId::HearingAid, &hearing_aid(false));
        assert_eq!(&f[6..], &[0x2C, 0x01, 0x02, 0x00, 0x00]);
        let f = build(ControlCommandId::HearingAssistConfig, &hearing_assist(true));
        assert_eq!(&f[6..], &[0x33, 0x01, 0x00, 0x00, 0x00]);
        let f = build(ControlCommandId::ListeningMode, &listening_mode(ListeningMode::Transparency));
        assert_eq!(&f[6..], &[0x0D, 0x03, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn parse_trims_trailing_zeros() {
        let f = build(ControlCommandId::HearingAid, &[0x01, 0x02]);
        let c = parse(&f).unwrap();
        assert_eq!(c.id, ControlCommandId::HearingAid);
        assert_eq!(c.value, vec![0x01, 0x02]);

        let f = build(ControlCommandId::ListeningMode, &[0x00]);
        assert_eq!(parse(&f).unwrap().value, vec![0x00]);
    }

    #[test]
    fn parse_rejects_other_opcodes_and_unknown_ids() {
        assert!(parse(&frame::build(0x04, &[0, 0, 0])).is_none());
        let mut f = build(ControlCommandId::HearingAid, &[1, 1]);
        f[6] = 0xEE;
        assert!(parse(&f).is_none());
        assert_eq!(parse_raw(&f).unwrap().0, None);
    }

    #[test]
    fn enabled_state_requires_both_commands() {
        assert!(is_hearing_aid_enabled(Some(&[1, 1]), Some(&[1])));
        assert!(!is_hearing_aid_enabled(Some(&[1, 2]), Some(&[1])));
        assert!(!is_hearing_aid_enabled(Some(&[1, 1]), Some(&[2])));
        assert!(!is_hearing_aid_enabled(None, Some(&[1])));
        // trimmed `[1]` (from `01 00`) must not read as enabled
        assert!(!is_hearing_aid_enabled(Some(&[1]), Some(&[1])));
        assert!(is_hearing_aid_enrolled(Some(&[1, 2])));
    }
}
