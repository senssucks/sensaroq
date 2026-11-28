//! The loading cover: a plain black picture instead of a half-made one,
//! with the game held still behind it.
//!
//! While many of a frame's draws go without their pipeline because it is
//! still being made (a new place, a cut to another scene, the first minute
//! of the world on a cold cache), the picture shows the ground without its
//! buildings, characters without bodies, the sky through the walls. The
//! cover shows plain gray in those frames instead, and lifts once few draws
//! wait, so what was the main of the place shows at once and whole.
//!
//! - `VICTORY_LOAD_COVER`: draws a frame waiting for their pipeline from
//!   which the cover goes up (0 by default: never; 1000 a good start), and only if that is
//!   also [`SPIKE`] times as many as usual (the running mean of frames it
//!   was down): play keeps a few hundred waiting a frame while it streams
//!   in new things (session 2b: 170 dropped a frame), and 100 kept a gray
//!   cover over the game.
//! - It goes up after [`RISE`] such presents in a row and lifts after
//!   [`QUIET`] presents in a row with at most a quarter of the threshold
//!   (or twice the usual, if more) waiting.
//! - `VICTORY_LOAD_COVER_MAX_S`: the longest it stays up at once (20 s by
//!   default), and then not again for [`REST`]: it can never hide the game
//!   for long, even if some pipelines never come.
//!
//! - Held (the default): while it is up, the title's thread that presents
//!   waits at its next present until every pipeline a draw has asked for is
//!   made (or the cover's time is out), so the game, its cutscenes with it,
//!   stands still behind a black picture, and goes on when the place is
//!   whole. Game time does not jump on: a frame moves it at most 125 ms
//!   (`step_clamp.rs`). The game's sound is muted meanwhile (what it had
//!   started plays on unheard; `VICTORY_LOAD_COVER_MUTE=0`: not muted).
//!   `VICTORY_LOAD_COVER_HOLD=0`: a gray picture only, the game going on.
//!
//! - Once (the default): only the first time it goes up in a run, the
//!   game's first big load; never again after it lifts.
//!   `VICTORY_LOAD_COVER_ONCE=0`: at every load spike.
//!
//! Only draws whose pipeline is on its way count: pipelines that failed to
//! translate drop their draws every frame and are left out of this.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How many times the usual waiting draws a frame raise the cover.
const SPIKE: f64 = 4.0;

/// Presents in a row over the threshold before the cover goes up.
const RISE: u32 = 2;
/// Presents in a row under a quarter of the threshold before it lifts.
const QUIET: u32 = 6;
/// After it was up for its longest, how long it stays down.
const REST: Duration = Duration::from_secs(30);
/// Pipelines still being made that no longer hold the game.
const STRAGGLERS: u64 = 2;

/// The cover's color (in the swap chain's own values): black held, gray not.
pub(crate) fn color() -> [f32; 4] {
    if hold() { [0.0, 0.0, 0.0, 1.0] } else { [0.2, 0.2, 0.2, 1.0] }
}

fn once() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_LOAD_COVER_ONCE").is_none_or(|v| v != "0"))
}

/// It went up and lifted once (with [`once`]: never up again).
static DONE: AtomicBool = AtomicBool::new(false);

fn hold() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_LOAD_COVER_HOLD").is_none_or(|v| v != "0"))
}

/// Whether the cover is up (for [`hold_title`]).
static UP: AtomicBool = AtomicBool::new(false);
/// When it went up (for [`hold_title`]'s limit).
static UP_SINCE: Mutex<Option<Instant>> = Mutex::new(None);

fn threshold() -> u64 {
    static N: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *N.get_or_init(|| std::env::var("VICTORY_LOAD_COVER").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(0))
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
    /// The running mean of waiting draws a frame while down.
    usual: f64,
}

/// What a present with `waiting` draws gone without their pipeline since
/// the last does next, at `now`: whether it is covered.
#[cfg(test)]
fn step(s: &mut State, waiting: u64, threshold: u64, longest: Duration, now: Instant) -> bool {
    step_with(s, waiting, threshold, longest, now, QUIET)
}

fn step_with(s: &mut State, waiting: u64, threshold: u64, longest: Duration, now: Instant, quiet: u32) -> bool {
    if threshold == 0 {
        return false;
    }
    match s.up {
        Some(since) => {
            s.covered += 1;
            s.most = s.most.max(waiting);
            let calm = (threshold / 4).max((2.0 * s.usual) as u64);
            s.run = if waiting <= calm { s.run + 1 } else { 0 };
            let timed_out = now.duration_since(since) >= longest;
            if s.run >= quiet || timed_out {
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
            let spike = waiting >= threshold && waiting as f64 >= SPIKE * s.usual;
            s.run = if spike { s.run + 1 } else { 0 };
            if s.run >= RISE {
                let usual = s.usual;
                tracing::info!(waiting, threshold, usual = format_args!("{usual:.0}"), "the loading cover goes up: draws wait for pipelines being made");
                *s = State { up: Some(now), covered: 1, most: waiting, usual, ..State::default() };
                return true;
            }
            if !spike {
                s.usual += (waiting as f64 - s.usual) * 0.05;
            }
            false
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Whether this present is covered; call once per present.
pub(crate) fn this_present() -> bool {
    let waiting = crate::pipeline::take_not_made_yet();
    if DONE.load(Ordering::Relaxed) && once() {
        return false;
    }
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let s = s.get_or_insert_with(State::default);
    // Held, the frame after the hold was made with what it waited for.
    let quiet = if hold() { 1 } else { QUIET };
    let up = step_with(s, waiting, threshold(), longest(), Instant::now(), quiet);
    *UP_SINCE.lock().unwrap_or_else(|e| e.into_inner()) = s.up;
    if UP.swap(up, Ordering::Relaxed) && !up {
        DONE.store(true, Ordering::Relaxed);
    }
    up
}

/// At the title's present: while the cover is up, waits until every
/// pipeline asked for is made, or the cover's time is out.
pub(crate) fn hold_title() {
    if !hold() || !UP.load(Ordering::Relaxed) {
        return;
    }
    let Some(since) = *UP_SINCE.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let until = since + longest();
    let began = Instant::now();
    let left_at_first = crate::pipeline::requested_left();
    let muted = left_at_first > STRAGGLERS && mute_wanted() && {
        mute(true);
        true
    };
    // A straggler or two (a program that takes tens of seconds) does not
    // keep the game still: what is left of them fills in after.
    while crate::pipeline::requested_left() > STRAGGLERS && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    let held = began.elapsed();
    if muted {
        mute(false);
    }
    if held >= Duration::from_millis(20) {
        tracing::info!(
            seconds = format_args!("{:.2}", held.as_secs_f64()),
            pipelines = left_at_first,
            left = crate::pipeline::requested_left(),
            "the game was held behind the loading cover while its pipelines were made (VICTORY_LOAD_COVER_HOLD=0: not held)"
        );
    }
}

fn mute_wanted() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_LOAD_COVER_MUTE").is_none_or(|v| v != "0"))
}

/// Mutes or unmutes this process's sound (its default audio session on the
/// default output), on a thread of its own so the title's thread keeps
/// its COM state, and waits for it.
fn mute(on: bool) {
    let done = std::thread::Builder::new().name("victory cover mute".into()).spawn(move || {
        use windows::Win32::Media::Audio::{IAudioSessionManager, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
        use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
        // SAFETY: plain COM calls on this thread's own apartment.
        let muted = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            (|| -> windows::core::Result<()> {
                let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
                let sessions: IAudioSessionManager = device.Activate(CLSCTX_ALL, None)?;
                sessions.GetSimpleAudioVolume(None, 0)?.SetMute(on, std::ptr::null())
            })()
        };
        if let Err(e) = muted {
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                tracing::warn!(error = %e, on, "the loading cover could not mute the game's sound");
            }
        }
    });
    if let Ok(done) = done {
        let _ = done.join();
    }
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
    fn steady_play_with_many_waiting_is_not_covered() {
        // Play that has gone on a while with 400 waiting a frame.
        let mut s = State { usual: 400.0, ..State::default() };
        let t = Instant::now();
        let long = Duration::from_secs(20);
        for _ in 0..200 {
            assert!(!step(&mut s, 400, 100, long, t));
        }
        // Up only on a jump well past the usual.
        assert!(!step(&mut s, 2000, 100, long, t));
        assert!(step(&mut s, 2000, 100, long, t));
        // And down again once back to the usual.
        for _ in 0..QUIET - 1 {
            assert!(step(&mut s, 400, 100, long, t));
        }
        assert!(!step(&mut s, 400, 100, long, t));
    }

    #[test]
    fn zero_is_never() {
        let mut s = State::default();
        for _ in 0..10 {
            assert!(!step(&mut s, 10_000, 0, Duration::from_secs(20), Instant::now()));
        }
    }
}
