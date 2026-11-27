//! `victory warm`: host pipelines made ahead of a run (`known_pipelines`), in
//! processes of their own: a compiler that crashes or hangs costs one pipeline.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use victory_hle::known_pipelines as known;

/// Pipelines a process makes before another takes its place: the compiler's
/// and the translations' memory goes with it.
const AT_ONCE: usize = 40;
/// A pipeline that takes longer is taken for one the compiler hangs on.
const LONGEST: Duration = Duration::from_secs(180);

/// As the runs count one that takes long.
const SLOW_MS: f64 = 1000.0;

fn folder() -> Result<PathBuf> {
    let cfg = victory_core::Config::load()?;
    Ok(cfg.cache_subdir("shaders")?.with_file_name("pipelines"))
}

pub fn run(jobs: Option<usize>, limit: Option<usize>, again: bool, used: bool, these: Option<String>, minutes: Option<f64>, recent_minutes: Option<f64>, count: bool) -> Result<()> {
    let dir = folder()?;
    if let Some(these) = these {
        return make(&dir, &these);
    }
    let began = Instant::now();
    let until = minutes.map(|m| began + Duration::from_secs_f64(m.max(0.0) * 60.0));
    victory_hle::known_pipelines::use_translations(dir.with_file_name("shaders"));
    let fingerprint = known::fingerprint();
    let done_file = dir.join("warm-done.txt");
    let bad_file = dir.join("warm-bad.txt");
    let keys = known::keys(&dir);
    let listed = std::fs::read_to_string(&done_file).unwrap_or_default();
    let mut done: HashSet<&str> = HashSet::new();
    for line in listed.lines() {
        if let Some((of, key)) = line.split_once(' ')
            && of == fingerprint
        {
            done.insert(key);
        }
    }
    let bad = std::fs::read_to_string(&bad_file).unwrap_or_default();
    let bad: HashSet<&str> = bad.lines().filter_map(|l| l.split(' ').next()).collect();
    let early = std::fs::read_to_string(dir.join("early-pipelines.txt")).unwrap_or_default();
    let early: HashSet<&str> = early.lines().collect();
    // What the last run drew with last, the latest first: the place the game
    // was left in, where the save goes on.
    let recent = std::fs::read_to_string(dir.join("recent-pipelines.txt")).unwrap_or_default();
    // "<key> <seconds before the end>" (or the key alone, from older runs).
    let recent: std::collections::HashMap<&str, (usize, u64)> = recent
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let mut parts = line.split_whitespace();
            let key = parts.next()?;
            Some((key, (i, parts.next().and_then(|s| s.parse().ok()).unwrap_or(0))))
        })
        .collect();
    let within = recent_minutes.map(|m| (m.max(0.0) * 60.0) as u64);
    // Those earlier runs drew with first, the others in no order (by key).
    let mut all: Vec<usize> = (0..keys.len())
        .filter(|&n| {
            let key = keys[n].as_str();
            (again || !done.contains(key))
                && !bad.contains(key)
                && (!used || early.contains(key))
                && within.is_none_or(|w| recent.get(key).is_some_and(|&(_, age)| age <= w))
        })
        .collect();
    all.sort_by_key(|&n| {
        let key = keys[n].as_str();
        (recent.get(key).map_or(usize::MAX, |&(i, _)| i), !early.contains(key), &keys[n])
    });
    all.truncate(limit.unwrap_or(usize::MAX));
    let rate_file = dir.join("warm-rate.txt");
    if count {
        // "<how many> <seconds they would take>", for the launcher's menu: at the
        // rate the last warm made them (pipelines a second), else Skylar's first
        // measured one (720 in 108 s).
        let rate = std::fs::read_to_string(&rate_file).ok().and_then(|s| s.trim().parse::<f64>().ok()).filter(|r| *r > 0.0).unwrap_or(6.7);
        println!("{} {:.0}", all.len(), all.len() as f64 / rate);
        return Ok(());
    }
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let jobs = jobs.unwrap_or(cores * 2 / 3).max(1);
    println!(
        "{} pipelines known, {} made before ({fingerprint}), {} bad; {} to make, {jobs} processes",
        keys.len(),
        done.len(),
        bad.len(),
        all.len()
    );
    let chunks: Vec<&[usize]> = all.chunks(AT_ONCE).collect();
    let next = AtomicUsize::new(0);
    let state = Mutex::new((Vec::<f64>::new(), 0usize, 0usize));
    let done_out = Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(&done_file)?);
    // (Read back by the runs: these are made beside the others, not before.)
    let slow_out = Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(dir.join("slow-pipelines.txt"))?);
    let bad_out = Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(&bad_file)?);
    std::thread::scope(|s| {
        for _ in 0..jobs.min(chunks.len()) {
            s.spawn(|| {
                while until.is_none_or(|t| Instant::now() < t)
                    && let Some(&chunk) = chunks.get(next.fetch_add(1, Ordering::Relaxed))
                {
                    let mut rest = chunk;
                    while !rest.is_empty() {
                        let (made, ended) = child(rest, |n, ms, there| {
                            if ms >= SLOW_MS {
                                let _ = writeln!(lock(&slow_out), "\n{} {ms:.0}", keys[n]);
                            }
                            let mut state = lock(&state);
                            state.0.push(ms);
                            state.1 += usize::from(!there);
                            let count = state.0.len();
                            bar(count, all.len(), began, until);
                            drop(state);
                            let _ = writeln!(lock(&done_out), "{fingerprint} {}", keys[n]);
                        });
                        rest = &rest[made.min(rest.len())..];
                        match (ended, rest.split_first()) {
                            (Some(why), Some((&n, after))) => {
                                // The one it was at.
                                println!("pipeline {n} {}: {why}", keys[n]);
                                let _ = writeln!(lock(&bad_out), "{} {why}", keys[n]);
                                lock(&state).2 += 1;
                                rest = after;
                            }
                            _ => break,
                        }
                    }
                }
            });
        }
    });
    println!();
    if until.is_some_and(|t| Instant::now() >= t) {
        println!("time is up: the rest are made while the game runs (and by the next warm)");
    }
    let (mut times, refused, crashed) = state.into_inner().unwrap_or_else(|e| e.into_inner());
    let spent = began.elapsed().as_secs_f64();
    if times.len() >= 50 && spent >= 10.0 {
        let _ = std::fs::write(&rate_file, format!("{:.2}", times.len() as f64 / spent));
    }
    times.sort_by(f64::total_cmp);
    let at = |q: f64| times.get((times.len() as f64 * q) as usize).copied().unwrap_or(0.0);
    println!(
        "{} made in {:.0} s ({refused} have no host pipeline, {crashed} ended their process); ms each: median {:.1}, p90 {:.1}, p99 {:.1}, most {:.0}, sum {:.0} s",
        times.len(),
        began.elapsed().as_secs_f64(),
        at(0.5),
        at(0.9),
        at(0.99),
        times.last().copied().unwrap_or(0.0),
        times.iter().sum::<f64>() / 1e3
    );
    Ok(())
}

/// Seconds as "1m 05s" (or "42s").
fn min_sec(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    if s >= 60 { format!("{}m {:02}s", s / 60, s % 60) } else { format!("{s}s") }
}

/// A line with a bar of how far it is, written over itself.
fn bar(count: usize, all: usize, began: Instant, until: Option<Instant>) {
    const WIDE: usize = 30;
    let step = (all / 200).max(1);
    if count % step != 0 && count != all {
        return;
    }
    let done = count as f64 / all.max(1) as f64;
    let filled = ((done * WIDE as f64) as usize).min(WIDE);
    let spent = began.elapsed().as_secs_f64();
    let left = if count > 0 { spent / count as f64 * (all - count) as f64 } else { 0.0 };
    let cap = until.map_or(String::new(), |t| format!(", stops in {}", min_sec(t.saturating_duration_since(Instant::now()).as_secs_f64())));
    print!(
        "\rbuilding shaders [{}{}] {:>3.0}%  {count}/{all}  about {} left{cap}    ",
        "#".repeat(filled),
        ".".repeat(WIDE - filled),
        done * 100.0,
        min_sec(left)
    );
    let _ = std::io::stdout().flush();
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Has a process make `these`; how many it made, and how it ended if not well.
fn child(these: &[usize], mut each: impl FnMut(usize, f64, bool)) -> (usize, Option<String>) {
    let list: Vec<String> = these.iter().map(usize::to_string).collect();
    let spawned = std::env::current_exe().and_then(|exe| {
        use std::os::windows::process::CommandExt;
        const IDLE_PRIORITY_CLASS: u32 = 0x40;
        Command::new(exe)
            .args(["warm", "--these", &list.join(",")])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(IDLE_PRIORITY_CLASS)
            .spawn()
    });
    let Ok(mut process) = spawned else { return (these.len(), None) };
    let Some(out) = process.stdout.take() else { return (these.len(), None) };
    let process = Mutex::new(process);
    let (heard, over) = (Mutex::new(Instant::now()), AtomicBool::new(false));
    let hung = AtomicBool::new(false);
    let mut made = 0;
    std::thread::scope(|s| {
        s.spawn(|| {
            while !over.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(500));
                if lock(&heard).elapsed() > LONGEST {
                    hung.store(true, Ordering::Relaxed);
                    let _ = lock(&process).kill();
                    return;
                }
            }
        });
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            *lock(&heard) = Instant::now();
            let mut words = line.split(' ');
            if words.next() == Some("made")
                && let (Some(n), Some(ms), Some(there)) = (words.next(), words.next(), words.next())
                && let (Ok(n), Ok(ms)) = (n.parse(), ms.parse())
            {
                made += 1;
                each(n, ms, there == "1");
            }
        }
        over.store(true, Ordering::Relaxed);
    });
    let status = lock(&process).wait();
    let well = status.as_ref().is_ok_and(|s| s.success());
    if well {
        return (these.len(), None);
    }
    let why = if hung.load(Ordering::Relaxed) {
        format!("hung for {} s", LONGEST.as_secs())
    } else {
        status.map_or_else(|e| e.to_string(), |s| format!("{s}"))
    };
    (made, Some(why))
}

/// What such a process does.
fn make(dir: &Path, these: &str) -> Result<()> {
    let these: Vec<usize> = these.split(',').map(|n| n.parse().context("--these")).collect::<Result<_>>()?;
    let cfg = victory_core::Config::load()?;
    victory_hle::known_pipelines::use_translations(cfg.cache_subdir("shaders")?);
    let mut out = std::io::stdout();
    known::make(dir, &these, |n, came| {
        let told = match came {
            None => writeln!(out, "at {n}"),
            Some((ms, there)) => writeln!(out, "made {n} {ms:.2} {}", u8::from(there)),
        };
        if told.and_then(|()| out.flush()).is_err() {
            // Whoever asked is gone.
            std::process::exit(1);
        }
    });
    // Without what a run's end does (nothing to save, the host's threads).
    let _ = out.flush();
    std::process::exit(0)
}
