//! The console's audio DSP (`chad.dll`): streams that convolve blocks of
//! float samples with an impulse response (reverb), or run other block
//! transforms, on a coprocessor.
//!
//! A title connects once with a memory block for the DSP, activates
//! streams, and submits one command per block (gains, an input and an
//! output buffer). Activation hands back a status block whose second word is
//! the ticket of the last completed command and whose fourth is its result;
//! the title checks it one block later and treats a late command as an
//! overrun. Commands complete when submitted here.
//!
//! Mono convolutions of 1024-frame blocks, the only kind the title has been
//! seen to use, are computed here ([`convolve`]): reverb, and the speakers in
//! the world (TVs, phone calls), whose sound goes through the DSP alone.
//! Only with `VICTORY_DSP_CONVOLVE=1` (trustno1-coder #77; off by default here).
// shortcut: stereo and 512-frame convolutions and the other block transforms
// are not computed; their outputs are zeros.
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::c_void;
use std::sync::Mutex;

use super::xasync::E_INVALIDARG;
use super::xgameruntime::{HRESULT, S_OK};
use crate::{HleModule, hle_export};

mod convolve;

/// Whether mono convolutions are computed (only with `VICTORY_DSP_CONVOLVE=1`).
fn convolve_on() -> bool {
    // (The tests drive the computed path.)
    if cfg!(test) {
        return true;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_DSP_CONVOLVE").is_some_and(|v| v == "1"))
}

/// What the computed convolutions are scaled by: `VICTORY_DSP_GAIN`, else 2.
/// The hardware's own scaling is not known. With the textbook 1/2048 of the
/// inverse FFT, TVs and phone calls sounded quiet next to the rest of the
/// mix; twice that sounded right by ear.
const GAIN: f32 = 2.0;

fn gain() -> f32 {
    static GAIN_SET: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *GAIN_SET.get_or_init(|| {
        std::env::var("VICTORY_DSP_GAIN").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(GAIN)
    })
}

const E_POINTER: HRESULT = 0x8000_4003_u32 as i32;
const E_OUTOFMEMORY: HRESULT = 0x8007_000E_u32 as i32;

const CONVOLUTION: u32 = 1;
/// Option bits that need a stereo stream, and one convolution refuses.
const STEREO_OPTIONS: u32 = 2 | 8;
const NOT_FOR_CONVOLUTION: u32 = 4;

pub struct Dsp {
    max_streams: u32,
    active: Mutex<u32>,
}

/// What activation describes.
#[repr(C, packed)]
pub struct StreamDesc {
    kind: u32,
    options: u32,
    frames: u32,
    channels: u32,
    ir_floats: u32,
    ir: *const f32,
}

/// The status block a stream's owner polls.
#[repr(C)]
pub struct Status {
    submitted: u32,
    completed: u32,
    reserved: u32,
    result: HRESULT,
}

pub struct Stream {
    dsp: *mut Dsp,
    kind: u32,
    frames: u32,
    channels: u32,
    status: Box<Status>,
    /// Set for a stream whose convolution is computed.
    convolver: Option<convolve::Convolver>,
}

/// One block of work.
#[repr(C)]
pub struct Command {
    gains: [f32; 2],
    input: *const f32,
    output: *mut f32,
    reserved: u64,
}

pub extern "system" fn XDspConnectWithMaximumStreamLimit(
    base: *mut c_void,
    base_length: u32,
    ir_seconds: u32,
    max_streams: u32,
    max_channels: u32,
    out: *mut *mut Dsp,
) -> HRESULT {
    if base.is_null()
        || !(0x1_0000..=0x1F40_0000).contains(&base_length)
        || ir_seconds > 0x80
        || max_streams > 0x100
        || max_channels > 2
    {
        return E_INVALIDARG;
    }
    if out.is_null() {
        return E_POINTER;
    }
    tracing::info!(
        base_length,
        ir_seconds,
        max_streams,
        max_channels,
        "audio DSP connected"
    );
    let dsp = Box::new(Dsp {
        max_streams,
        active: Mutex::new(0),
    });
    // SAFETY: checked non-null.
    unsafe { out.write(Box::into_raw(dsp)) };
    S_OK
}

pub extern "system" fn XDspDisconnect(dsp: *mut Dsp) -> HRESULT {
    if dsp.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: made by `XDspConnectWithMaximumStreamLimit`; disconnected
    // once, after its streams.
    drop(unsafe { Box::from_raw(dsp) });
    S_OK
}

fn valid(desc: &StreamDesc) -> bool {
    let (kind, options, frames, channels) = (desc.kind, desc.options, desc.frames, desc.channels);
    let shape = if kind == CONVOLUTION {
        (1..=2).contains(&channels)
            && matches!(frames, 0x200 | 0x400)
            && options & NOT_FOR_CONVOLUTION == 0
    } else {
        channels == 1 && matches!(frames, 0x200 | 0x400 | 0x800)
    };
    shape && (options & STEREO_OPTIONS == 0 || channels == 2)
}

pub extern "system" fn XDspActivate(
    dsp: *mut Dsp,
    desc: *const StreamDesc,
    status: *mut *const Status,
    stream: *mut *mut Stream,
) -> HRESULT {
    if status.is_null() || stream.is_null() {
        return E_POINTER;
    }
    if dsp.is_null() || desc.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: checked non-null; the title passes its description.
    let desc = unsafe { &*desc };
    if !valid(desc) {
        return E_INVALIDARG;
    }
    let (kind, options, frames, channels) = (desc.kind, desc.options, desc.frames, desc.channels);
    let (ir, ir_floats) = (desc.ir, desc.ir_floats);
    let convolver = if convolve_on() && kind == CONVOLUTION && channels == 1 && frames == 0x400 && !ir.is_null() {
        // SAFETY: the title's description says the impulse response holds this many floats.
        convolve::Convolver::new(unsafe { std::slice::from_raw_parts(ir, ir_floats as usize) })
    } else {
        None
    };
    {
        static TOLD: Mutex<u32> = Mutex::new(0);
        let mut told = TOLD.lock().unwrap_or_else(|e| e.into_inner());
        if *told < 32 {
            *told += 1;
                        tracing::info!(
                kind,
                options,
                frames,
                channels,
                ir_floats,
                computed = convolver.is_some(),
                dry = dry(),
                "audio DSP stream activated (first 32; its output is silence unless computed or VICTORY_DSP_DRY=1)"
            );
        }
        // `VICTORY_DSP_IR_DUMP=<dir>` (off by default): the first 4 impulse
        // responses as the title hands them (little-endian f32), to work out
        // the coprocessor's layout offline. Their sizes are whole multiples
        // of 2048 floats (66 to 95 of them in session 32), which looks like
        // partitions of a 2048-point transform of 1024-frame blocks.
        if let Some(dir) = std::env::var_os("VICTORY_DSP_IR_DUMP") {
            static DUMPED: Mutex<u32> = Mutex::new(0);
            let mut dumped = DUMPED.lock().unwrap_or_else(|e| e.into_inner());
            let ir = desc.ir;
            let ir_floats = desc.ir_floats as usize;
            if *dumped < 4 && !ir.is_null() && ir_floats > 0 {
                // SAFETY: the title's impulse response of `ir_floats` floats.
                let floats = unsafe { std::slice::from_raw_parts(ir.cast::<u8>(), ir_floats * 4) };
                let dir = std::path::PathBuf::from(dir);
                let _ = std::fs::create_dir_all(&dir);
                let name = format!("ir-{}-f{frames}-c{channels}-n{ir_floats}.f32", *dumped);
                match std::fs::write(dir.join(&name), floats) {
                    Ok(()) => tracing::info!(name, "audio DSP impulse response dumped (VICTORY_DSP_IR_DUMP)"),
                    Err(e) => tracing::warn!(error = %e, "audio DSP impulse response not dumped"),
                }
                *dumped += 1;
            }
        }
    }
    {
        // SAFETY: a connection from `XDspConnectWithMaximumStreamLimit`.
        let dsp = unsafe { &*dsp };
        let mut active = dsp.active.lock().unwrap_or_else(|e| e.into_inner());
        if *active >= dsp.max_streams {
            return E_OUTOFMEMORY;
        }
        *active += 1;
    }
    let new = Box::new(Stream {
        dsp,
        kind: desc.kind,
        frames: desc.frames,
        channels: desc.channels,
        status: Box::new(Status {
            submitted: 0,
            completed: 0,
            reserved: 0,
            result: S_OK,
        }),
        convolver,
    });
    // SAFETY: checked non-null.
    unsafe {
        status.write(&*new.status);
        stream.write(Box::into_raw(new));
    }
    S_OK
}

pub extern "system" fn XDspDeactivate(stream: *mut Stream) -> HRESULT {
    if stream.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: made by `XDspActivate`; deactivated once.
    let stream = unsafe { Box::from_raw(stream) };
    // SAFETY: the connection outlives its streams.
    let dsp = unsafe { &*stream.dsp };
    *dsp.active.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
    S_OK
}

/// `VICTORY_DSP_DRY=1` (default 0): a convolution's output is its input
/// times the command's first gain, as if the impulse response were one
/// click, so a voice sent only through the DSP is heard (dry, without its
/// reverb or filter) instead of being silent. Assumes the input block has
/// the output's layout.
fn dry() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("VICTORY_DSP_DRY").is_ok_and(|v| v.trim() == "1"))
}

/// What the streams were given, every 30 s: blocks by kind, and how many
/// had a sound in them (a peak over 1/1000), which is what we leave silent.
struct Stats {
    since: Option<std::time::Instant>,
    /// Convolution, other transforms: blocks, blocks with sound, loudest.
    blocks: [u32; 2],
    heard: [u32; 2],
    peak: [f32; 2],
    gains: [f32; 2],
}

static STATS: Mutex<Stats> = Mutex::new(Stats {
    since: None,
    blocks: [0; 2],
    heard: [0; 2],
    peak: [0.0; 2],
    gains: [0.0; 2],
});

fn count(kind: u32, input: &[f32], gains: [f32; 2]) {
    let peak = input
        .iter()
        .fold(0f32, |m, &x| if x.is_finite() { m.max(x.abs()) } else { m });
    let mut s = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let k = usize::from(kind != CONVOLUTION);
    s.blocks[k] += 1;
    s.heard[k] += u32::from(peak > 1e-3);
    s.peak[k] = s.peak[k].max(peak);
    s.gains[k] = s.gains[k].max(gains[0].abs().max(gains[1].abs()));
    let since = *s.since.get_or_insert_with(std::time::Instant::now);
    if since.elapsed().as_secs() >= 30 {
        tracing::info!(
            convolution_blocks = s.blocks[0],
            with_sound = s.heard[0],
            peak = s.peak[0],
            gain = s.gains[0],
            other_blocks = s.blocks[1],
            other_with_sound = s.heard[1],
            other_peak = s.peak[1],
            dry = dry(),
            "audio DSP, last 30 s (outputs are silence unless VICTORY_DSP_DRY=1)"
        );
        *s = Stats {
            since: Some(std::time::Instant::now()),
            blocks: [0; 2],
            heard: [0; 2],
            peak: [0.0; 2],
            gains: [0.0; 2],
        };
    }
}

/// Completes the command at once: its output block is the convolution where
/// it is computed, silence otherwise.
pub extern "system" fn XDspSubmitCommand(
    stream: *mut Stream,
    command: *const Command,
    ticket: *mut u32,
) -> HRESULT {
    if stream.is_null() || command.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: a stream from `XDspActivate`, the title's command.
    let (stream, command) = unsafe { (&mut *stream, &*command) };
    if !command.input.is_null() {
        // SAFETY: the title's input holds at least one channel's block.
        let input = unsafe { std::slice::from_raw_parts(command.input, stream.frames as usize) };
        count(stream.kind, input, command.gains);
    }
    let samples = (stream.frames * stream.channels) as usize;
    if !command.output.is_null() && !command.input.is_null() && let Some(convolver) = stream.convolver.as_mut() {
        // SAFETY: the title's input and output buffers each hold one block of the stream.
        let (input, output) = unsafe {
            (std::slice::from_raw_parts(command.input, samples), std::slice::from_raw_parts_mut(command.output, samples))
        };
        convolver.process(input, output, command.gains[0] * gain());
    } else if !command.output.is_null() {
        if dry() && stream.kind == CONVOLUTION && !command.input.is_null() {
            let gain = command.gains[0];
            for n in 0..samples {
                // SAFETY: input and output blocks of the stream, assumed the
                // same layout (see `dry`); read before written, so one
                // buffer for both is fine.
                unsafe {
                    let i = command.input.add(n).read_unaligned();
                    command.output.add(n).write_unaligned(if i.is_finite() {
                        i * gain
                    } else {
                        0.0
                    });
                }
            }
        } else {
            // SAFETY: the title's output buffer holds one block of the stream.
            unsafe { std::ptr::write_bytes(command.output, 0, samples) };
        }
    }
    let status = &mut *stream.status;
    let next = status.submitted.wrapping_add(1);
    // SAFETY: the status block is read by title code on other threads.
    unsafe {
        std::ptr::write_volatile(&mut status.submitted, next);
        std::ptr::write_volatile(&mut status.result, S_OK);
        std::ptr::write_volatile(&mut status.completed, next);
    }
    if !ticket.is_null() {
        // SAFETY: optional out parameter.
        unsafe { ticket.write(next) };
    }
    S_OK
}

pub(crate) fn module() -> HleModule {
    HleModule {
        name: "dsp",
        dlls: &["chad.dll"],
        exports: vec![
            hle_export!(XDspConnectWithMaximumStreamLimit),
            hle_export!(XDspDisconnect),
            hle_export!(XDspActivate),
            hle_export!(XDspDeactivate),
            hle_export!(XDspSubmitCommand),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convolution_stream_as_the_title_drives_it() {
        let mut base = vec![0u8; 0x10_0000];
        let mut dsp = std::ptr::null_mut();
        assert_eq!(
            XDspConnectWithMaximumStreamLimit(
                base.as_mut_ptr().cast(),
                0x10_0000,
                24,
                0x80,
                1,
                &mut dsp
            ),
            S_OK
        );
        let ir = vec![0f32; 0x800];
        // Stereo convolution with option 2, as the title asks for.
        let desc = StreamDesc {
            kind: CONVOLUTION,
            options: 2,
            frames: 0x400,
            channels: 2,
            ir_floats: 0x800,
            ir: ir.as_ptr(),
        };
        let (mut status, mut stream) = (std::ptr::null(), std::ptr::null_mut());
        assert_eq!(XDspActivate(dsp, &desc, &mut status, &mut stream), S_OK);
        let mut output = vec![1f32; 0x800];
        let input = vec![0.5f32; 0x800];
        let command = Command {
            gains: [1.0, 1.0],
            input: input.as_ptr(),
            output: output.as_mut_ptr(),
            reserved: 0,
        };
        let mut ticket = 0;
        assert_eq!(XDspSubmitCommand(stream, &command, &mut ticket), S_OK);
        // SAFETY: the stream's status block.
        let (completed, result) = unsafe { ((*status).completed, (*status).result) };
        assert_eq!((completed, result), (ticket, S_OK));
        assert!(output.iter().all(|&s| s == 0.0));
        assert_eq!(XDspDeactivate(stream), S_OK);

        // Mono streams refuse stereo options; transforms are mono only.
        let bad = StreamDesc {
            channels: 1,
            ..desc
        };
        assert_eq!(
            XDspActivate(dsp, &bad, &mut status, &mut stream),
            E_INVALIDARG
        );
        let transform = StreamDesc {
            kind: 2,
            options: 0,
            channels: 1,
            ir_floats: 0,
            ir: std::ptr::null(),
            ..desc
        };
        assert_eq!(
            XDspActivate(dsp, &transform, &mut status, &mut stream),
            S_OK
        );
        assert_eq!(XDspDeactivate(stream), S_OK);
        assert_eq!(XDspDisconnect(dsp), S_OK);
    }

    #[test]
    fn a_mono_convolution_is_computed() {
        let mut base = vec![0u8; 0x10_0000];
        let mut dsp = std::ptr::null_mut();
        assert_eq!(XDspConnectWithMaximumStreamLimit(base.as_mut_ptr().cast(), 0x10_0000, 24, 0x80, 1, &mut dsp), S_OK);
        // a response of one sample, 0.5, two samples late
        let ir = convolve::layout(&[0.0, 0.0, 0.5]);
        let desc = StreamDesc {
            kind: CONVOLUTION,
            options: 0,
            frames: 0x400,
            channels: 1,
            ir_floats: ir.len() as u32,
            ir: ir.as_ptr(),
        };
        let (mut status, mut stream) = (std::ptr::null(), std::ptr::null_mut());
        assert_eq!(XDspActivate(dsp, &desc, &mut status, &mut stream), S_OK);
        let input: Vec<f32> = (0..0x400).map(|i| (i % 7) as f32 / 7.0).collect();
        let mut output = vec![0f32; 0x400];
        let command = Command { gains: [1.0, 1.0], input: input.as_ptr(), output: output.as_mut_ptr(), reserved: 0 };
        let mut ticket = 0;
        assert_eq!(XDspSubmitCommand(stream, &command, &mut ticket), S_OK);
        for n in 2..0x400 {
            let want = input[n - 2] * 0.5 * gain();
            assert!((output[n] - want).abs() < 1e-4, "sample {n}: {} vs {want}", output[n]);
        }
        assert_eq!(XDspDeactivate(stream), S_OK);
        assert_eq!(XDspDisconnect(dsp), S_OK);
    }
}
