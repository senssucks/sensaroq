//! The loading cover: a plain gray picture instead of a half-made one.
//!
//! While many of a frame's draws go without their pipeline because it is
//! still being made (a new place, a cut to another scene, the first minute
//! of the world on a cold cache), the picture shows the ground without its
//! buildings, characters without bodies, the sky through the walls. The
//! cover shows plain gray in those frames instead, and lifts once few draws
//! wait, so what was the main of the place shows at once and whole.
//!
//! - `VICTORY_LOAD_COVER`: draws a frame waiting for their pipeline from
//!   which the cover goes up (100 by default; 0 never).
//! - It goes up after [`RISE`] such presents in a row and lifts after
//!   [`QUIET`] presents in a row with at most a quarter as many waiting.
//! - `VICTORY_LOAD_COVER_MAX_S`: the longest it stays up at once (20 s by
//!   default), and then not again for [`REST`]: it can never hide the game
//!   for long, even if some pipelines never come.
//!
//! Only draws whose pipeline is on its way count: pipelines that failed to
//! translate drop their draws every frame and are left out of this.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Presents in a row over the threshold before the cover goes up.
const RISE: u32 = 2;
/// Presents in a row under a quarter of the threshold before it lifts.
const QUIET: u32 = 6;
/// After it was up for its longest, how long it stays down.
const REST: Duration = Duration::from_secs(15);

/// The gray (in the swap chain's own values).
pub(crate) const GRAY: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

fn threshold() -> u64 {
    static N: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *N.get_or_init(|| std::env::var("VICTORY_LOAD_COVER").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(100))
}

fn longest() -> Duration {
    static S: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        Duration::from_secs_f64(
            std::env::var("VICTORY_LOAD_COVER_MAX_S").ok().and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(20.0).clamp(1.0, 600.0),
        )
    })
}

#[derive(Default)]
struct State {
    /// Since when it is up.
    up: Option<Instant>,
    /// Presents in a row over the threshold (down) or under its quarter (up).
    run: u32,
    /// Not up again before this.
    rest_until: Option<Instant>,
    /// Presents covered and the most waiting draws in a frame, this time up.
    covered: u64,
    most: u64,
}

/// What a present with `waiting` draws gone without their pipeline since
/// the last does next, at `now`: whether it is covered.
fn step(s: &mut State, waiting: u64, threshold: u64, longest: Duration, now: Instant) -> bool {
    if threshold == 0 {
        return false;
    }
    match s.up {
        Some(since) => {
            s.covered += 1;
            s.most = s.most.max(waiting);
            s.run = if waiting <= threshold / 4 { s.run + 1 } else { 0 };
            let timed_out = now.duration_since(since) >= longest;
            if s.run >= QUIET || timed_out {
                tracing::info!(
                    seconds = format_args!("{:.1}", now.duration_since(since).as_secs_f64()),
                    presents = s.covered,
                    most_waiting = s.most,
                    timed_out,
                    "the loading cover lifts (VICTORY_LOAD_COVER=0: never up)"
                );
                s.up = None;
                s.run = 0;
                if timed_out {
                    s.rest_until = Some(now + REST);
                }
                return false;
            }
            true
        }
        None => {
            if s.rest_until.is_some_and(|t| now < t) {
                return false;
            }
            s.run = if waiting >= threshold { s.run + 1 } else { 0 };
            if s.run >= RISE {
                tracing::info!(waiting, threshold, "the loading cover goes up: draws wait for pipelines being made");
                *s = State { up: Some(now), covered: 1, most: waiting, ..State::default() };
                return true;
            }
            false
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Whether this present is covered; call once per present.
pub(crate) fn this_present() -> bool {
    let waiting = crate::pipeline::take_not_made_yet();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    step(s.get_or_insert_with(State::default), waiting, threshold(), longest(), Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_on_a_run_of_waiting_frames_and_down_when_quiet() {
        let mut s = State::default();
        let t = Instant::now();
        let long = Duration::from_secs(20);
        assert!(!step(&mut s, 500, 100, long, t));
        assert!(step(&mut s, 500, 100, long, t));
        for _ in 0..QUIET - 1 {
            assert!(step(&mut s, 10, 100, long, t));
        }
        assert!(!step(&mut s, 10, 100, long, t));
        // One busy frame alone does not raise it.
        assert!(!step(&mut s, 500, 100, long, t));
        assert!(!step(&mut s, 0, 100, long, t));
    }

    #[test]
    fn never_up_for_longer_than_asked_then_rests() {
        let mut s = State::default();
        let t = Instant::now();
        let long = Duration::from_secs(20);
        step(&mut s, 500, 100, long, t);
        assert!(step(&mut s, 500, 100, long, t));
        assert!(!step(&mut s, 500, 100, long, t + long));
        assert!(!step(&mut s, 500, 100, long, t + long + Duration::from_secs(1)));
        assert!(!step(&mut s, 500, 100, long, t + long + Duration::from_secs(2)));
        let later = t + long + REST;
        assert!(!step(&mut s, 500, 100, long, later));
        assert!(step(&mut s, 500, 100, long, later));
    }

    #[test]
    fn zero_is_never() {
        let mut s = State::default();
        for _ in 0..10 {
            assert!(!step(&mut s, 10_000, 0, Duration::from_secs(20), Instant::now()));
        }
    }
}
