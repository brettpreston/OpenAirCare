//! Pure-tone generator for the hearing test, rendered on Makepad's audio
//! output thread. `ToneShared` is the UI-side handle: it holds the current
//! command behind a mutex the audio callback polls once per buffer.
//!
//! Level model: the audiogram bands are in dB HL; the tone is rendered in
//! dBFS. `db_hl_to_amp` converts using an *estimated* reference table for
//! AirPods Pro at 100 % system volume plus a user offset, and caps the
//! result at [`MAX_TONE_AMP`] no matter what. There is no calibrated path
//! from a Linux host to the buds, so results are approximate by design.

use std::f32::consts::{PI, TAU};
use std::sync::{Arc, Mutex};

use makepad_widgets::*;

/// Which channel(s) a tone goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ear {
    #[default]
    Left,
    Right,
    Both,
}

/// One tone burst: Apple's pattern of three short beeps.
pub const BEEPS: u32 = 3;
pub const BEEP_ON_SECS: f32 = 0.25;
pub const BEEP_OFF_SECS: f32 = 0.25;
/// Raised-cosine ramp at each end of a beep, so there is no click.
pub const RAMP_SECS: f32 = 0.02;
/// Total burst length (last beep's off-time included).
pub const BURST_SECS: f32 = BEEPS as f32 * (BEEP_ON_SECS + BEEP_OFF_SECS);

/// Hard ceiling on the rendered amplitude (about -6 dBFS), applied after the
/// calibration offset. Nothing the UI does can push a tone above this.
pub const MAX_TONE_AMP: f32 = 0.5;

/// Estimated dBFS that plays at 0 dB HL on AirPods Pro over A2DP with the
/// system volume at 100 %, per audiogram band (250 Hz .. 8 kHz).
///
/// Built as insert-earphone RETSPL (ISO 389-2, dB SPL at threshold) minus an
/// assumed ~100 dB SPL at 0 dBFS. It is an estimate: the level-offset slider
/// on the Hearing Test tab exists to correct it, and levels below roughly
/// 10 dB HL sit near the Bluetooth codec's noise floor anyway.
pub const REF_DBFS_AT_0_HL: [f32; 8] = [-86.0, -94.5, -100.0, -97.0, -96.5, -94.5, -98.0, -100.0];

/// Linear amplitude for `db_hl` at audiogram band `band`, with the user's
/// level offset in dB. Always finite and within `0..=MAX_TONE_AMP`.
pub fn db_hl_to_amp(band: usize, db_hl: f32, offset_db: f32) -> f32 {
    let dbfs = REF_DBFS_AT_0_HL[band.min(7)] + db_hl + offset_db;
    if !dbfs.is_finite() {
        return 0.0;
    }
    10f32.powf(dbfs / 20.0).min(MAX_TONE_AMP)
}

/// Burst envelope (0..=1) at `t` seconds after the burst started.
pub fn envelope(t: f32) -> f32 {
    if t < 0.0 || t >= BURST_SECS {
        return 0.0;
    }
    let period = BEEP_ON_SECS + BEEP_OFF_SECS;
    let in_beep = t % period;
    if in_beep >= BEEP_ON_SECS {
        return 0.0;
    }
    // Raised-cosine in and out; flat in the middle.
    let edge = in_beep.min(BEEP_ON_SECS - in_beep);
    if edge >= RAMP_SECS {
        1.0
    } else {
        0.5 - 0.5 * (PI * edge / RAMP_SECS).cos()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToneCmd {
    pub freq_hz: f32,
    pub amp: f32,
    pub ear: Ear,
    /// Bumped by every `play`; the synth restarts the burst when it changes.
    pub serial: u64,
    /// Repeat the 3-beep burst until `stop` (manual mode).
    pub looping: bool,
}

/// UI-side handle, cloned into the audio callback.
#[derive(Clone, Default)]
pub struct ToneShared(Arc<Mutex<ToneCmd>>);

impl ToneShared {
    /// Start a new burst. `amp` is a linear amplitude (see [`db_hl_to_amp`]).
    pub fn play(&self, freq_hz: f32, amp: f32, ear: Ear) {
        self.start(freq_hz, amp, ear, false);
    }

    /// Start a burst that repeats until [`stop`](Self::stop); the level can
    /// be changed on the fly with [`set_amp`](Self::set_amp).
    pub fn play_looping(&self, freq_hz: f32, amp: f32, ear: Ear) {
        self.start(freq_hz, amp, ear, true);
    }

    fn start(&self, freq_hz: f32, amp: f32, ear: Ear, looping: bool) {
        if let Ok(mut c) = self.0.lock() {
            c.serial = c.serial.wrapping_add(1);
            c.freq_hz = freq_hz;
            c.amp = amp.clamp(0.0, MAX_TONE_AMP);
            c.ear = ear;
            c.looping = looping;
        }
    }

    /// Change the level of the current tone without restarting the beep
    /// pattern (the synth smooths the step).
    pub fn set_amp(&self, amp: f32) {
        if let Ok(mut c) = self.0.lock() {
            c.amp = amp.clamp(0.0, MAX_TONE_AMP);
        }
    }

    /// Silence, with a short fade handled by the synth.
    pub fn stop(&self) {
        if let Ok(mut c) = self.0.lock() {
            c.amp = 0.0;
        }
    }
}

/// Renders `ToneCmd`s into sample buffers. Pure, so it is unit-testable.
#[derive(Default)]
pub struct ToneSynth {
    phase: f32,
    /// Samples since the current burst started.
    burst_pos: u64,
    serial: u64,
    /// Smoothed gain, so a `stop` mid-beep fades instead of clicking.
    gain: f32,
}

impl ToneSynth {
    pub fn render(&mut self, cmd: &ToneCmd, sample_rate: f32, left: &mut [f32], right: &mut [f32]) {
        if cmd.serial != self.serial {
            self.serial = cmd.serial;
            self.burst_pos = 0;
        }
        let sr = sample_rate.max(1.0);
        // Loop in whole samples: `burst_pos` grows without bound and an f32
        // modulo would lose precision after a few minutes.
        let burst_len = ((BURST_SECS * sr) as u64).max(1);
        // ~5 ms one-pole smoothing on the gain target.
        let coef = 1.0 - (-1.0 / (0.005 * sr)).exp();
        let step = TAU * cmd.freq_hz / sr;
        let (to_l, to_r) = match cmd.ear {
            Ear::Left => (true, false),
            Ear::Right => (false, true),
            Ear::Both => (true, true),
        };
        for i in 0..left.len().min(right.len()) {
            let pos = if cmd.looping { self.burst_pos % burst_len } else { self.burst_pos };
            let target = cmd.amp * envelope(pos as f32 / sr);
            self.gain += (target - self.gain) * coef;
            let s = self.phase.sin() * self.gain;
            left[i] = if to_l { s } else { 0.0 };
            right[i] = if to_r { s } else { 0.0 };
            self.phase += step;
            if self.phase >= TAU {
                self.phase -= TAU;
            }
            self.burst_pos += 1;
        }
    }
}

/// Hook the synth onto audio output 0 (whatever `use_audio_outputs` chose).
pub fn install(cx: &mut Cx, shared: ToneShared) {
    let mut synth = ToneSynth::default();
    let mut cmd = ToneCmd::default();
    let mut scratch_l: Vec<f32> = Vec::new();
    let mut scratch_r: Vec<f32> = Vec::new();
    cx.audio_output(0, move |info, buf| {
        // Never block the audio thread: keep the last command if the UI
        // happens to hold the lock.
        if let Ok(c) = shared.0.try_lock() {
            cmd = c.clone();
        }
        buf.zero();
        let n = buf.frame_count();
        let ch = buf.channel_count();
        if n == 0 || ch == 0 {
            return;
        }
        scratch_l.resize(n, 0.0);
        scratch_r.resize(n, 0.0);
        synth.render(&cmd, info.sample_rate as f32, &mut scratch_l, &mut scratch_r);
        if ch == 1 {
            // Mono device: the ear selection cannot be honoured, so mix down.
            let out = buf.channel_mut(0);
            for i in 0..n {
                out[i] = scratch_l[i] + scratch_r[i];
            }
        } else {
            buf.channel_mut(0).copy_from_slice(&scratch_l);
            buf.channel_mut(1).copy_from_slice(&scratch_r);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_is_capped_whatever_the_offset() {
        for band in 0..8 {
            assert!(db_hl_to_amp(band, 120.0, 60.0) <= MAX_TONE_AMP);
            assert!(db_hl_to_amp(band, -10.0, -20.0) > 0.0);
        }
        assert_eq!(db_hl_to_amp(2, f32::NAN, 0.0), 0.0);
    }

    #[test]
    fn one_khz_at_zero_hl_is_minus_100_dbfs() {
        let a = db_hl_to_amp(2, 0.0, 0.0);
        assert!((20.0 * a.log10() + 100.0).abs() < 1e-3);
    }

    #[test]
    fn envelope_starts_and_ends_each_beep_at_zero() {
        assert_eq!(envelope(-0.1), 0.0);
        assert_eq!(envelope(0.0), 0.0);
        assert!((envelope(BEEP_ON_SECS / 2.0) - 1.0).abs() < 1e-6);
        assert_eq!(envelope(BEEP_ON_SECS + 0.01), 0.0);
        assert_eq!(envelope(BURST_SECS), 0.0);
        assert_eq!(envelope(BURST_SECS + 1.0), 0.0);
        let period = BEEP_ON_SECS + BEEP_OFF_SECS;
        for k in 0..BEEPS {
            assert!(envelope(k as f32 * period + BEEP_ON_SECS / 2.0) > 0.99);
        }
    }

    #[test]
    fn left_only_leaves_right_silent() {
        let mut s = ToneSynth::default();
        let cmd = ToneCmd { freq_hz: 1000.0, amp: 0.3, ear: Ear::Left, serial: 1, looping: false };
        let mut l = vec![0.0; 4800];
        let mut r = vec![0.0; 4800];
        s.render(&cmd, 48000.0, &mut l, &mut r);
        assert!(r.iter().all(|v| *v == 0.0));
        let peak = l.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.25 && peak <= 0.3 + 1e-4, "peak {peak}");
    }

    #[test]
    fn looping_repeats_the_burst() {
        let n = (2.0 * BURST_SECS * 48000.0) as usize;
        let probe = ((BURST_SECS + BEEP_ON_SECS / 2.0) * 48000.0) as usize;
        for looping in [true, false] {
            let mut s = ToneSynth::default();
            let cmd = ToneCmd { freq_hz: 1000.0, amp: 0.3, ear: Ear::Both, serial: 1, looping };
            let mut l = vec![0.0; n];
            let mut r = vec![0.0; n];
            s.render(&cmd, 48000.0, &mut l, &mut r);
            let peak = l[probe - 200..probe + 200].iter().fold(0f32, |m, v| m.max(v.abs()));
            if looping {
                assert!(peak > 0.25, "looping burst should be audible in its second repeat, peak {peak}");
            } else {
                assert!(peak < 1e-3, "one-shot burst should be over, peak {peak}");
            }
        }
    }

    #[test]
    fn amp_change_keeps_burst_position() {
        let mut s = ToneSynth::default();
        let mut cmd = ToneCmd { freq_hz: 1000.0, amp: 0.3, ear: Ear::Both, serial: 1, looping: true };
        let mut l = vec![0.0; 4800];
        let mut r = vec![0.0; 4800];
        s.render(&cmd, 48000.0, &mut l, &mut r);
        cmd.amp = 0.1;
        let mut l2 = vec![0.0; 480];
        let mut r2 = vec![0.0; 480];
        s.render(&cmd, 48000.0, &mut l2, &mut r2);
        assert_eq!(s.burst_pos, 4800 + 480);
        // 100 ms in we are inside the first beep; a restart would re-enter
        // the 20 ms zero ramp.
        let peak = l2.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.05 && peak <= 0.3 + 1e-4, "peak {peak}");
    }

    #[test]
    fn set_amp_does_not_restart() {
        let t = ToneShared::default();
        t.play_looping(1000.0, 0.2, Ear::Left);
        let serial = t.0.lock().unwrap().serial;
        t.set_amp(0.4);
        let c = t.0.lock().unwrap().clone();
        assert_eq!(c.serial, serial);
        assert!((c.amp - 0.4).abs() < 1e-6);
        assert!(c.looping);
        t.play(500.0, 0.1, Ear::Both);
        assert!(!t.0.lock().unwrap().looping, "play must clear looping");
    }

    #[test]
    fn stop_fades_to_silence() {
        let mut s = ToneSynth::default();
        let mut cmd = ToneCmd { freq_hz: 1000.0, amp: 0.3, ear: Ear::Both, serial: 1, looping: false };
        let mut l = vec![0.0; 4800];
        let mut r = vec![0.0; 4800];
        s.render(&cmd, 48000.0, &mut l, &mut r);
        cmd.amp = 0.0;
        s.render(&cmd, 48000.0, &mut l, &mut r);
        assert!(l[2400..].iter().all(|v| v.abs() < 1e-3));
    }
}
