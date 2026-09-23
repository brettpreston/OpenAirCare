//! Hearing-aid configuration blob (ATT handle `0x2A`) and the derived,
//! UI-facing models.
//!
//! Layout (all little-endian IEEE-754 f32 unless noted):
//!
//! | offset | field |
//! |-------:|-------|
//! | 0      | `0x02` (opaque, preserved) |
//! | 1      | `0x00` on a fresh device, `0x02` once a profile is stored; **must be `0x02` on write** |
//! | 2      | `0x60` when read; **must be `0x64` on write** |
//! | 3      | `0x00` |
//! | 4..36  | left audiogram, 8 bands (dB HL) |
//! | 36     | left amplification |
//! | 40     | left tone |
//! | 44     | left conversation boost (1.0 / 0.0) |
//! | 48     | left ambient noise reduction |
//! | 52..84 | right audiogram, 8 bands |
//! | 84     | right amplification |
//! | 88     | right tone |
//! | 92     | right conversation boost |
//! | 96     | right ambient noise reduction |
//! | 100    | own-voice amplification |
//!
//! Writes are read-modify-write over the last buffer read from the device so
//! that unknown bytes are preserved.

use serde::{Deserialize, Serialize};

/// Minimum length of the hearing-aid characteristic value.
pub const MIN_LEN: usize = 104;

/// Byte 2 must carry this value when writing.
pub const WRITE_MARKER: u8 = 0x64;

/// Byte 1 must carry this value when writing. A never-configured AirPods
/// Pro 3 reports `0x00` here and silently discards (ACKs, then ignores) any
/// write that keeps the `0x00`; after one accepted write it reports `0x02`.
/// Verified on firmware 8A by writing and reading back.
pub const PROFILE_MARKER: u8 = 0x02;

/// Amplification range accepted by [`Adjustments::apply_to`]. Apple's UI
/// stops at 1.0, but the buds store and apply larger values (verified on
/// AirPods Pro 3: 1.5 is audibly louder than 1.0), and the transparency
/// blob documents the field as 0..2. Kept symmetric below at -1.
pub const AMP_MIN: f32 = -1.0;
pub const AMP_MAX: f32 = 2.0;

/// Audiogram band centre frequencies (Hz), in blob order.
pub const BANDS_HZ: [u16; 8] = [250, 500, 1000, 2000, 3000, 4000, 6000, 8000];

pub const OFF_LEFT_EQ: usize = 4;
pub const OFF_LEFT_AMP: usize = 36;
pub const OFF_LEFT_TONE: usize = 40;
pub const OFF_LEFT_CONV: usize = 44;
pub const OFF_LEFT_ANR: usize = 48;
pub const OFF_RIGHT_EQ: usize = 52;
pub const OFF_RIGHT_AMP: usize = 84;
pub const OFF_RIGHT_TONE: usize = 88;
pub const OFF_RIGHT_CONV: usize = 92;
pub const OFF_RIGHT_ANR: usize = 96;
pub const OFF_OWN_VOICE: usize = 100;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("hearing aid data too short: {0} bytes (need {MIN_LEN})")]
    TooShort(usize),
}

fn get_f32(buf: &[u8], off: usize) -> f32 {
    f32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn put_f32(buf: &mut [u8], off: usize, v: f32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// Per-ear parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EarParams {
    /// Audiogram in dB HL at [`BANDS_HZ`].
    pub eq: [f32; 8],
    pub amplification: f32,
    pub tone: f32,
    pub conversation_boost: bool,
    pub ambient_noise_reduction: f32,
}

impl Default for EarParams {
    fn default() -> Self {
        Self {
            eq: [0.0; 8],
            amplification: 0.0,
            tone: 0.0,
            conversation_boost: false,
            ambient_noise_reduction: 0.0,
        }
    }
}

impl EarParams {
    fn decode(buf: &[u8], eq_off: usize, amp: usize, tone: usize, conv: usize, anr: usize) -> Self {
        let mut eq = [0.0f32; 8];
        for (i, e) in eq.iter_mut().enumerate() {
            *e = get_f32(buf, eq_off + i * 4);
        }
        Self {
            eq,
            amplification: get_f32(buf, amp),
            tone: get_f32(buf, tone),
            conversation_boost: get_f32(buf, conv) > 0.5,
            ambient_noise_reduction: get_f32(buf, anr),
        }
    }

    fn encode(&self, buf: &mut [u8], eq_off: usize, amp: usize, tone: usize, conv: usize, anr: usize) {
        for (i, e) in self.eq.iter().enumerate() {
            put_f32(buf, eq_off + i * 4, *e);
        }
        put_f32(buf, amp, self.amplification);
        put_f32(buf, tone, self.tone);
        put_f32(buf, conv, if self.conversation_boost { 1.0 } else { 0.0 });
        put_f32(buf, anr, self.ambient_noise_reduction);
    }
}

/// Full decoded hearing-aid configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct HearingAidData {
    pub left: EarParams,
    pub right: EarParams,
    pub own_voice_amplification: f32,
}

impl HearingAidData {
    /// Decode from the characteristic value.
    pub fn decode(buf: &[u8]) -> Result<Self, CodecError> {
        if buf.len() < MIN_LEN {
            return Err(CodecError::TooShort(buf.len()));
        }
        Ok(Self {
            left: EarParams::decode(buf, OFF_LEFT_EQ, OFF_LEFT_AMP, OFF_LEFT_TONE, OFF_LEFT_CONV, OFF_LEFT_ANR),
            right: EarParams::decode(buf, OFF_RIGHT_EQ, OFF_RIGHT_AMP, OFF_RIGHT_TONE, OFF_RIGHT_CONV, OFF_RIGHT_ANR),
            own_voice_amplification: get_f32(buf, OFF_OWN_VOICE),
        })
    }

    /// Encode into an existing buffer previously read from the device
    /// (read-modify-write). Sets the write marker at byte 2.
    pub fn encode_into(&self, buf: &mut [u8]) -> Result<(), CodecError> {
        if buf.len() < MIN_LEN {
            return Err(CodecError::TooShort(buf.len()));
        }
        buf[1] = PROFILE_MARKER;
        buf[2] = WRITE_MARKER;
        self.left.encode(buf, OFF_LEFT_EQ, OFF_LEFT_AMP, OFF_LEFT_TONE, OFF_LEFT_CONV, OFF_LEFT_ANR);
        self.right.encode(buf, OFF_RIGHT_EQ, OFF_RIGHT_AMP, OFF_RIGHT_TONE, OFF_RIGHT_CONV, OFF_RIGHT_ANR);
        put_f32(buf, OFF_OWN_VOICE, self.own_voice_amplification);
        Ok(())
    }

    /// Encode into a fresh 104-byte buffer with the header `02 02 64 00`. Prefer [`encode_into`](Self::encode_into) when a
    /// device buffer is available.
    pub fn encode_fresh(&self) -> Vec<u8> {
        let mut buf = vec![0u8; MIN_LEN];
        buf[0] = 0x02;
        self.encode_into(&mut buf).expect("buffer is MIN_LEN");
        buf
    }

    pub fn audiogram(&self) -> Audiogram {
        Audiogram { left: self.left.eq, right: self.right.eq }
    }

    pub fn adjustments(&self) -> Adjustments {
        Adjustments::from_data(self)
    }
}

/// Audiogram (hearing loss in dB HL per band, per ear).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Audiogram {
    pub left: [f32; 8],
    pub right: [f32; 8],
}

impl Audiogram {
    pub fn apply_to(&self, data: &mut HearingAidData) {
        data.left.eq = self.left;
        data.right.eq = self.right;
    }
}

/// UI-facing adjustments. Amplification is [`AMP_MIN`]`..=`[`AMP_MAX`],
/// balance/tone are `-1..=1`, ambient noise reduction is `0..=1`. Tone, ANR and conversation boost are applied
/// symmetrically to both ears; balance shifts amplification between ears.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Adjustments {
    pub amplification: f32,
    pub balance: f32,
    pub tone: f32,
    pub ambient_noise_reduction: f32,
    pub conversation_boost: bool,
}

impl Default for Adjustments {
    fn default() -> Self {
        Self::RESET
    }
}

impl Adjustments {
    /// Values written by the "Reset adjustments" action.
    pub const RESET: Adjustments = Adjustments {
        amplification: 0.0,
        balance: 0.0,
        tone: 0.0,
        ambient_noise_reduction: 0.0,
        conversation_boost: false,
    };

    /// Derive from decoded device data. This is the exact inverse of
    /// [`apply_to`](Self::apply_to): the ear the balance leans *away* from
    /// carries the base amplification, the other ear has the balance added.
    /// (The Android app averages the two ears instead, which is not a fixed
    /// point of its own encoder: with a non-zero balance every UI -> device
    /// -> UI round trip would bump amplification by `|balance| / 2`.)
    pub fn from_data(d: &HearingAidData) -> Self {
        let base = d.left.amplification.min(d.right.amplification);
        let diff = d.right.amplification - d.left.amplification;
        Self {
            amplification: base.clamp(AMP_MIN, AMP_MAX),
            balance: diff.clamp(-1.0, 1.0),
            tone: d.left.tone,
            ambient_noise_reduction: d.left.ambient_noise_reduction,
            conversation_boost: d.left.conversation_boost,
        }
    }

    /// Write into device data (Android `HearingAidAdjustmentsScreen.kt`
    /// formula: the ear on the side the balance leans to gets the extra gain).
    pub fn apply_to(&self, d: &mut HearingAidData) {
        let amp = self.amplification.clamp(AMP_MIN, AMP_MAX);
        let bal = self.balance.clamp(-1.0, 1.0);
        d.left.amplification = (amp + if bal < 0.0 { -bal } else { 0.0 }).min(AMP_MAX);
        d.right.amplification = (amp + if bal > 0.0 { bal } else { 0.0 }).min(AMP_MAX);
        d.left.tone = self.tone.clamp(-1.0, 1.0);
        d.right.tone = d.left.tone;
        d.left.ambient_noise_reduction = self.ambient_noise_reduction.clamp(0.0, 1.0);
        d.right.ambient_noise_reduction = d.left.ambient_noise_reduction;
        d.left.conversation_boost = self.conversation_boost;
        d.right.conversation_boost = self.conversation_boost;
    }
}

/// Helpers for the customized-transparency blob (handle `0x18`), which has
/// the same body as the hearing-aid blob but starts with an `enabled` f32
/// instead of the 4-byte header. Only the enabled flag is needed here: the
/// reference app disables customized transparency when enabling hearing aid.
pub mod transparency {
    pub const MIN_LEN: usize = 100;

    pub fn is_enabled(buf: &[u8]) -> Option<bool> {
        if buf.len() < 4 {
            return None;
        }
        Some(f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) > 0.5)
    }

    /// Set the enabled flag in place. Returns false if the buffer is too
    /// short to be a transparency blob.
    pub fn set_enabled(buf: &mut [u8], enabled: bool) -> bool {
        if buf.len() < MIN_LEN {
            return false;
        }
        let v: f32 = if enabled { 1.0 } else { 0.0 };
        buf[0..4].copy_from_slice(&v.to_le_bytes());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // Simulates a device read: header 02 02 60 00, then floats.
        let mut buf = vec![0u8; MIN_LEN];
        buf[0] = 0x02;
        buf[1] = 0x02;
        buf[2] = 0x60;
        let d = HearingAidData {
            left: EarParams {
                eq: [30.0, 35.0, 40.0, 40.0, 35.0, 30.0, 40.0, 40.0],
                amplification: 0.25,
                tone: -0.5,
                conversation_boost: true,
                ambient_noise_reduction: 0.75,
            },
            right: EarParams {
                eq: [50.0, 55.0, 60.0, 60.0, 55.0, 50.0, 60.0, 60.0],
                amplification: 0.75,
                tone: -0.5,
                conversation_boost: true,
                ambient_noise_reduction: 0.75,
            },
            own_voice_amplification: 0.5,
        };
        d.encode_into(&mut buf).unwrap();
        buf[2] = 0x60; // as read from the device
        buf
    }

    #[test]
    fn decode_reads_all_fields() {
        let d = HearingAidData::decode(&fixture()).unwrap();
        assert_eq!(d.left.eq[0], 30.0);
        assert_eq!(d.right.eq[7], 60.0);
        assert_eq!(d.left.amplification, 0.25);
        assert_eq!(d.right.amplification, 0.75);
        assert_eq!(d.left.tone, -0.5);
        assert!(d.left.conversation_boost);
        assert_eq!(d.right.ambient_noise_reduction, 0.75);
        assert_eq!(d.own_voice_amplification, 0.5);
    }

    #[test]
    fn known_bytes_at_known_offsets() {
        // 1.0f32 LE = 00 00 80 3F ; -1.0 = 00 00 80 BF (values from the RE gist)
        let mut d = HearingAidData::default();
        d.left.amplification = 1.0;
        d.right.amplification = -1.0;
        d.left.conversation_boost = true;
        let buf = d.encode_fresh();
        assert_eq!(&buf[0..4], &[0x02, 0x02, 0x64, 0x00]);
        assert_eq!(&buf[OFF_LEFT_AMP..OFF_LEFT_AMP + 4], &[0x00, 0x00, 0x80, 0x3F]);
        assert_eq!(&buf[OFF_RIGHT_AMP..OFF_RIGHT_AMP + 4], &[0x00, 0x00, 0x80, 0xBF]);
        assert_eq!(&buf[OFF_LEFT_CONV..OFF_LEFT_CONV + 4], &[0x00, 0x00, 0x80, 0x3F]);
        assert_eq!(&buf[OFF_RIGHT_CONV..OFF_RIGHT_CONV + 4], &[0x00, 0x00, 0x00, 0x00]);
        assert_eq!(buf.len(), MIN_LEN);
    }

    #[test]
    fn encode_into_preserves_unknown_bytes_and_sets_marker() {
        let mut buf = fixture();
        buf.push(0xAB); // extra trailing byte the device might send
        buf[0] = 0x07; // unknown header byte: must be preserved
        buf[1] = 0x00; // fresh device: must become PROFILE_MARKER or the buds drop the write
        let d = HearingAidData::decode(&buf).unwrap();
        d.encode_into(&mut buf).unwrap();
        assert_eq!(buf[0], 0x07);
        assert_eq!(buf[1], PROFILE_MARKER);
        assert_eq!(buf[2], WRITE_MARKER);
        assert_eq!(buf[3], 0x00);
        assert_eq!(*buf.last().unwrap(), 0xAB);
        assert_eq!(HearingAidData::decode(&buf).unwrap(), d);
    }

    #[test]
    fn too_short_is_an_error() {
        assert_eq!(HearingAidData::decode(&[0; 50]), Err(CodecError::TooShort(50)));
        let mut b = vec![0; 50];
        assert!(HearingAidData::default().encode_into(&mut b).is_err());
    }

    #[test]
    fn adjustments_round_trip() {
        let mut d = HearingAidData::default();
        let a = Adjustments {
            amplification: 0.4,
            balance: 0.3,
            tone: -0.2,
            ambient_noise_reduction: 0.6,
            conversation_boost: true,
        };
        a.apply_to(&mut d);
        assert!((d.left.amplification - 0.4).abs() < 1e-6);
        assert!((d.right.amplification - 0.7).abs() < 1e-6);
        let back = Adjustments::from_data(&d);
        assert!((back.amplification - 0.4).abs() < 1e-6, "amplification must survive the round trip");
        assert!((back.balance - 0.3).abs() < 1e-6);
        assert!((back.tone + 0.2).abs() < 1e-6);
        assert!(back.conversation_boost);

        let mut d2 = HearingAidData::default();
        Adjustments { balance: -0.5, amplification: 0.0, ..Adjustments::RESET }.apply_to(&mut d2);
        assert_eq!(d2.left.amplification, 0.5);
        assert_eq!(d2.right.amplification, 0.0);
        assert_eq!(Adjustments::from_data(&d2).balance, -0.5);
        assert_eq!(Adjustments::from_data(&d2).amplification, 0.0);
    }

    #[test]
    fn extended_amplification_survives() {
        let mut d = HearingAidData::default();
        Adjustments { amplification: 1.5, balance: 0.0, ..Adjustments::RESET }.apply_to(&mut d);
        assert_eq!(d.left.amplification, 1.5);
        assert_eq!(d.right.amplification, 1.5);
        assert_eq!(Adjustments::from_data(&d).amplification, 1.5);
        // balance on top of an extended value is capped per ear
        Adjustments { amplification: 1.8, balance: 0.5, ..Adjustments::RESET }.apply_to(&mut d);
        assert_eq!(d.left.amplification, 1.8);
        assert_eq!(d.right.amplification, AMP_MAX);
    }

    #[test]
    fn adjustments_round_trip_is_a_fixed_point() {
        // Repeatedly re-deriving and re-applying must not drift (the UI does
        // exactly this on every snapshot while a slider is dragged).
        let start = Adjustments { amplification: -0.3, balance: 0.6, tone: 0.0, ambient_noise_reduction: 0.0, conversation_boost: false };
        let mut d = HearingAidData::default();
        start.apply_to(&mut d);
        for _ in 0..10 {
            let a = Adjustments::from_data(&d);
            assert!((a.amplification - start.amplification).abs() < 1e-6);
            assert!((a.balance - start.balance).abs() < 1e-6);
            a.apply_to(&mut d);
        }
        assert!((d.left.amplification + 0.3).abs() < 1e-6);
        assert!((d.right.amplification - 0.3).abs() < 1e-6);
    }

    #[test]
    fn adjustments_clamp() {
        let mut d = HearingAidData::default();
        Adjustments { amplification: 5.0, balance: 0.0, tone: -9.0, ambient_noise_reduction: 2.0, conversation_boost: false }
            .apply_to(&mut d);
        assert_eq!(d.left.amplification, AMP_MAX);
        assert_eq!(d.left.tone, -1.0);
        assert_eq!(d.right.ambient_noise_reduction, 1.0);
    }

    #[test]
    fn audiogram_json_round_trip() {
        let a = Audiogram { left: [10.0; 8], right: [20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0] };
        let s = serde_json::to_string(&a).unwrap();
        let b: Audiogram = serde_json::from_str(&s).unwrap();
        assert_eq!(a, b);
        let mut d = HearingAidData::default();
        b.apply_to(&mut d);
        assert_eq!(d.right.eq[7], 55.0);
        assert_eq!(d.audiogram(), a);
    }

    #[test]
    fn transparency_flag() {
        let mut buf = vec![0u8; transparency::MIN_LEN];
        assert_eq!(transparency::is_enabled(&buf), Some(false));
        assert!(transparency::set_enabled(&mut buf, true));
        assert_eq!(&buf[0..4], &[0x00, 0x00, 0x80, 0x3F]);
        assert_eq!(transparency::is_enabled(&buf), Some(true));
        assert!(!transparency::set_enabled(&mut [0u8; 10], true));
    }
}
