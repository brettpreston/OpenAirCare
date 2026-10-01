//! Pure-tone audiometry logic, no I/O: which ear and band comes next, what
//! level to present, and the modified Hughson-Westlake staircase that turns
//! heard / not-heard answers into a threshold per band.
//!
//! Sequence per ear (Apple's order, with a 1 kHz retest): 1k, 2k, 3k, 4k,
//! 6k, 8k, 1k, 500, 250. Left ear first, then right.

use airpods_proto::hearing::{Audiogram, BANDS_HZ, DB_HL_MAX, DB_HL_MIN};

use crate::tone::Ear;

/// Levels are stepped in 5 dB and bounded here (dB HL).
pub const MIN_DB: f32 = -10.0;
pub const MAX_DB: f32 = 80.0;
pub const START_DB: f32 = 40.0;
const STEP_DOWN: f32 = 10.0;
const STEP_UP: f32 = 5.0;
/// Safety valve so a random clicker cannot run a band forever.
const MAX_TRIALS_PER_BAND: u32 = 30;

/// Band indices into `BANDS_HZ`, in presentation order for one ear.
const SEQUENCE: [usize; 9] = [2, 3, 4, 5, 6, 7, 2, 1, 0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presentation {
    pub ear: Ear,
    pub band: usize,
    pub db_hl: f32,
}

impl Presentation {
    pub fn freq_hz(&self) -> f32 {
        BANDS_HZ[self.band] as f32
    }
}

/// Modified Hughson-Westlake for a single band.
///
/// Start at 40 dB HL. Down 10 after a response, up 5 after a miss. The
/// threshold is the lowest level answered on at least two ascending
/// presentations. A miss at `MAX_DB` ends the band at `MAX_DB` with the
/// `ceiling` flag; two responses at `MIN_DB` end it at `MIN_DB`.
#[derive(Debug, Clone)]
struct Staircase {
    level: f32,
    /// The previous answer was a miss, so this presentation is ascending.
    ascending: bool,
    /// Ascending responses per 5 dB slot from `MIN_DB`.
    asc_heard: [u8; 19],
    floor_heard: u8,
    trials: u32,
    done: Option<(f32, bool)>,
}

impl Staircase {
    fn new() -> Self {
        Self { level: START_DB, ascending: false, asc_heard: [0; 19], floor_heard: 0, trials: 0, done: None }
    }

    fn slot(level: f32) -> usize {
        (((level - MIN_DB) / STEP_UP).round() as usize).min(18)
    }

    fn respond(&mut self, heard: bool) {
        if self.done.is_some() {
            return;
        }
        self.trials += 1;
        if heard {
            if self.ascending {
                let s = Self::slot(self.level);
                self.asc_heard[s] += 1;
                if self.asc_heard[s] >= 2 {
                    self.done = Some((self.level, false));
                    return;
                }
            }
            if self.level <= MIN_DB {
                self.floor_heard += 1;
                if self.floor_heard >= 2 {
                    self.done = Some((MIN_DB, false));
                    return;
                }
            }
            self.level = (self.level - STEP_DOWN).max(MIN_DB);
            self.ascending = false;
        } else {
            if self.level >= MAX_DB {
                self.done = Some((MAX_DB, true));
                return;
            }
            self.level = (self.level + STEP_UP).min(MAX_DB);
            self.ascending = true;
        }
        if self.trials >= MAX_TRIALS_PER_BAND {
            // Best available answer: the lowest level heard on an ascent.
            let best = self
                .asc_heard
                .iter()
                .position(|n| *n > 0)
                .map(|s| MIN_DB + s as f32 * STEP_UP)
                .unwrap_or(MAX_DB);
            self.done = Some((best, best >= MAX_DB));
        }
    }
}

#[derive(Debug, Clone)]
struct BandResult {
    ear: Ear,
    band: usize,
    db_hl: f32,
    ceiling: bool,
}

#[derive(Debug, Clone)]
pub struct TestRunner {
    steps: Vec<(Ear, usize)>,
    idx: usize,
    stair: Staircase,
    results: Vec<BandResult>,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunner {
    pub fn new() -> Self {
        let mut steps = Vec::with_capacity(SEQUENCE.len() * 2);
        for ear in [Ear::Left, Ear::Right] {
            steps.extend(SEQUENCE.iter().map(|b| (ear, *b)));
        }
        Self { steps, idx: 0, stair: Staircase::new(), results: Vec::new() }
    }

    /// What to play now, or `None` once every band is done.
    pub fn current(&self) -> Option<Presentation> {
        let (ear, band) = *self.steps.get(self.idx)?;
        Some(Presentation { ear, band, db_hl: self.stair.level })
    }

    /// Record the answer to the current presentation.
    pub fn respond(&mut self, heard: bool) {
        let Some((ear, band)) = self.steps.get(self.idx).copied() else { return };
        self.stair.respond(heard);
        if let Some((db_hl, ceiling)) = self.stair.done {
            self.results.push(BandResult { ear, band, db_hl, ceiling });
            self.idx += 1;
            self.stair = Staircase::new();
        }
    }

    /// (bands finished, bands total).
    pub fn progress(&self) -> (usize, usize) {
        (self.idx, self.steps.len())
    }

    pub fn is_done(&self) -> bool {
        self.idx >= self.steps.len()
    }

    /// The audiogram (clamped to the AirPods' 0..120 dB HL range) plus
    /// human-readable notes, once the run is complete.
    pub fn result(&self) -> Option<(Audiogram, Vec<String>)> {
        if !self.is_done() {
            return None;
        }
        let mut ag = Audiogram::default();
        let mut notes = Vec::new();
        for ear in [Ear::Left, Ear::Right] {
            let name = match ear {
                Ear::Left => "Left",
                _ => "Right",
            };
            for band in 0..8 {
                let hits: Vec<&BandResult> = self.results.iter().filter(|r| r.ear == ear && r.band == band).collect();
                if hits.is_empty() {
                    continue;
                }
                let mean = hits.iter().map(|r| r.db_hl).sum::<f32>() / hits.len() as f32;
                // Keep the 5 dB grid after averaging the 1 kHz pair.
                let v = ((mean / STEP_UP).round() * STEP_UP).clamp(DB_HL_MIN, DB_HL_MAX);
                if hits.len() == 2 && (hits[0].db_hl - hits[1].db_hl).abs() > 10.0 {
                    notes.push(format!(
                        "{name} {} Hz: the two measurements differ by {:.0} dB; consider retesting.",
                        BANDS_HZ[band],
                        (hits[0].db_hl - hits[1].db_hl).abs()
                    ));
                }
                if hits.iter().any(|r| r.ceiling) {
                    notes.push(format!("{name} {} Hz: not heard at {MAX_DB:.0} dB HL (recorded as {MAX_DB:.0}+).", BANDS_HZ[band]));
                }
                match ear {
                    Ear::Left => ag.left[band] = v,
                    _ => ag.right[band] = v,
                }
            }
        }
        Some((ag, notes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic listener: hears anything at or above `threshold`.
    fn run(threshold: impl Fn(Ear, usize) -> f32) -> (Audiogram, Vec<String>, usize) {
        let mut r = TestRunner::new();
        let mut n = 0;
        while let Some(p) = r.current() {
            r.respond(p.db_hl >= threshold(p.ear, p.band));
            n += 1;
            assert!(n < 2000, "did not terminate");
        }
        let (ag, notes) = r.result().unwrap();
        (ag, notes, n)
    }

    #[test]
    fn flat_20_db_loss() {
        let (ag, notes, _) = run(|_, _| 20.0);
        assert!(ag.left.iter().chain(ag.right.iter()).all(|v| *v == 20.0), "{ag:?}");
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn sloping_high_frequency_loss_right_ear() {
        let (ag, _, _) = run(|ear, band| if ear == Ear::Right { 10.0 + band as f32 * 10.0 } else { 5.0 });
        for b in 0..8 {
            assert_eq!(ag.left[b], 5.0);
            assert_eq!(ag.right[b], 10.0 + b as f32 * 10.0);
        }
    }

    #[test]
    fn deaf_ear_hits_ceiling_and_is_noted() {
        let (ag, notes, _) = run(|ear, _| if ear == Ear::Left { 200.0 } else { 0.0 });
        assert!(ag.left.iter().all(|v| *v == MAX_DB));
        assert!(ag.right.iter().all(|v| *v == 0.0));
        assert_eq!(notes.iter().filter(|n| n.starts_with("Left")).count(), 8);
    }

    #[test]
    fn normal_hearing_clamps_to_zero() {
        let (ag, _, _) = run(|_, _| -10.0);
        assert!(ag.left.iter().chain(ag.right.iter()).all(|v| *v == 0.0));
    }

    #[test]
    fn progress_counts_bands() {
        let mut r = TestRunner::new();
        assert_eq!(r.progress(), (0, 18));
        assert_eq!(r.current().unwrap(), Presentation { ear: Ear::Left, band: 2, db_hl: START_DB });
        // heard 40 -> 30 -> 20 -> miss -> 25 heard(1) -> 15 miss -> 20 miss -> 25 heard(2)
        for h in [true, true, false, true, false, false, true] {
            r.respond(h);
        }
        assert_eq!(r.progress(), (1, 18));
        assert_eq!(r.current().unwrap().band, 3);
    }

    #[test]
    fn random_clicker_terminates() {
        let mut r = TestRunner::new();
        let mut x: u32 = 12345;
        let mut n = 0;
        while r.current().is_some() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            r.respond(x & 1 == 1);
            n += 1;
            assert!(n < 18 * MAX_TRIALS_PER_BAND as usize + 1);
        }
        assert!(r.result().is_some());
    }
}
