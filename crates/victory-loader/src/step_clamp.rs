//! The longest step of game time a frame may take (`VICTORY_MAX_STEP_MS`,
//! off without it).
//!
//! What the title does (game_scarlett_master.exe 1.0.2.1, image offsets):
//! +0x2226040 makes a timer's step of the real time since the last frame
//! (`frame_rate` has the rest): `vmovss xmm3, [+0x6159ec0]` (1/15 s),
//! `vminss xmm0, xmm3, xmm1`, then the shortest step and the time scale.
//! A frame longer than 1/15 s moves game time 1/15 s only: below 15 frames
//! a second the game runs slower than the clock while its sound keeps the
//! clock (at 10 a second two thirds of its speed; in a 2-a-second stall,
//! an eighth), and cutscenes fall out of step with their sound.
//!
//! Here that one load is pointed at a value of ours, put in the padding
//! after the function's `ret` (int3 bytes nothing runs). The shared
//! constant is left as it is. The function's bytes, the padding and the
//! constant are compared first; anything else and nothing is written.

/// The function, from its start to its `ret`.
pub const AT: usize = 0x2226040;
pub const BYTES: [u8; 230] = [
    0x45, 0x84, 0xc9, 0x74, 0x51, 0xc5, 0xfa, 0x10, 0x1d, 0x73, 0x3e, 0xf3, 0x03, 0x80, 0x3d, 0x6c, 0xdc, 0x22,
    0x05, 0x00, 0xc5, 0xe2, 0x5d, 0xc1, 0x74, 0x42, 0xc5, 0xfa, 0x10, 0x25, 0xe2, 0x37, 0x62, 0x04, 0xc5, 0xf8,
    0x2e, 0x15, 0x6e, 0x0e, 0xda, 0x03, 0xc5, 0xf2, 0xc2, 0xcc, 0x01, 0xc4, 0xe3, 0x79, 0x4a, 0xc4, 0x10, 0xc5,
    0xfa, 0x59, 0xc2, 0x75, 0x02, 0x7b, 0x36, 0xc5, 0xfa, 0x10, 0x0d, 0xdb, 0x15, 0x62, 0x04, 0xc5, 0xe2, 0x5d,
    0xd0, 0xc5, 0xfa, 0xc2, 0xc1, 0x01, 0xc4, 0xe3, 0x69, 0x4a, 0xc1, 0x00, 0xeb, 0x1d, 0xc5, 0xea, 0x59, 0xc1,
    0xeb, 0x17, 0xc5, 0xfa, 0x10, 0x1d, 0xbc, 0x15, 0x62, 0x04, 0xc5, 0xf2, 0xc2, 0xcb, 0x01, 0xc4, 0xe3, 0x79,
    0x4a, 0xc3, 0x10, 0xc5, 0xfa, 0x59, 0xc2, 0xc5, 0xfa, 0x10, 0x49, 0x18, 0xc5, 0xfa, 0x10, 0x15, 0x68, 0x3b,
    0xf3, 0x03, 0xc4, 0xe2, 0x79, 0x99, 0x0d, 0xfb, 0x3d, 0xf3, 0x03, 0xc5, 0xf2, 0x59, 0xca, 0xc4, 0xe3, 0x71,
    0x0a, 0xd9, 0x0b, 0xc4, 0xe1, 0xfa, 0x2c, 0xc1, 0xc5, 0xf2, 0x5c, 0xcb, 0xc5, 0xfa, 0x11, 0x49, 0x18, 0xc5,
    0xfa, 0x59, 0xca, 0xc5, 0xfa, 0x11, 0x41, 0x0c, 0xc4, 0xe1, 0xfa, 0x2c, 0xd1, 0xc5, 0xfa, 0x10, 0x0d, 0xe1,
    0x0d, 0xda, 0x03, 0x89, 0x51, 0x10, 0xc5, 0xf2, 0x5e, 0xc0, 0xc4, 0xe2, 0x79, 0x58, 0x0d, 0x71, 0xd6, 0xd1,
    0x03, 0xc4, 0xe3, 0x71, 0x22, 0xc8, 0x00, 0xc5, 0xfa, 0x11, 0x41, 0x14, 0x48, 0x8b, 0x11, 0xc4, 0xe1, 0xf9,
    0x6e, 0xc2, 0x89, 0x51, 0x08, 0xc5, 0xf9, 0xfe, 0xc1, 0xc5, 0xf9, 0xd6, 0x01, 0xc3
];
/// The load of the longest step: `vmovss xmm3, [rip+disp32]`, its
/// displacement 4 bytes into it, the next instruction 8 bytes on.
pub const LOAD: usize = AT + 5;
/// Where our value goes: 4 bytes of the int3 padding after the `ret`
/// (0x2226126-0x222612f).
pub const SLOT: usize = 0x2226128;
const PADDING: std::ops::Range<usize> = 0x2226126..0x2226130;
/// The title's own longest step, 1/15 s.
pub const TITLE_MAX: f32 = 1.0 / 15.0;

/// `VICTORY_MAX_STEP_MS`: the longest step in milliseconds, 67 to 1000
/// (shorter than the title's own is not allowed); unset, 0 or malformed:
/// the title's 1/15 s.
pub fn asked_seconds() -> Option<f32> {
    let spec = std::env::var("VICTORY_MAX_STEP_MS").ok()?;
    parse_ms(&spec)
}

pub fn parse_ms(spec: &str) -> Option<f32> {
    let ms = spec.trim().parse::<f32>().ok().filter(|ms| ms.is_finite() && (67.0..=1000.0).contains(ms))?;
    Some(ms / 1000.0)
}

/// The load with its displacement pointed at [`SLOT`].
pub fn new_load() -> [u8; 8] {
    let disp = (SLOT as i64 - (LOAD as i64 + 8)) as i32;
    let d = disp.to_le_bytes();
    [0xc5, 0xfa, 0x10, 0x1d, d[0], d[1], d[2], d[3]]
}

/// What the title's load reads now (image offset).
pub fn old_target() -> usize {
    let disp = i32::from_le_bytes(BYTES[9..13].try_into().unwrap());
    (LOAD as i64 + 8 + i64::from(disp)) as usize
}

/// Points the load at `seconds`. A reason when nothing was written.
///
/// # Safety
/// The image of `size` bytes at `base` stays mapped and no thread runs
/// its code yet.
#[cfg(windows)]
pub unsafe fn apply(base: usize, size: usize, seconds: f32) -> Result<(), &'static str> {
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    if PADDING.end > size || old_target() + 4 > size {
        return Err("outside the image");
    }
    // SAFETY: inside the mapped image (checked above).
    let (code, padding, title_max) = unsafe {
        (
            std::slice::from_raw_parts((base + AT) as *const u8, BYTES.len()),
            std::slice::from_raw_parts((base + PADDING.start) as *const u8, PADDING.len()),
            ((base + old_target()) as *const f32).read_unaligned(),
        )
    };
    if code != BYTES {
        return Err("the title's bytes are not the ones expected (another build?)");
    }
    if padding.iter().any(|&b| b != 0xcc) {
        return Err("the padding after the function is not int3 bytes");
    }
    if (title_max - TITLE_MAX).abs() > 1e-6 {
        return Err("the title's longest step is not 1/15 s");
    }
    let (from, to) = (SLOT.min(LOAD), (SLOT + 4).max(LOAD + 8));
    let mut old = 0;
    // SAFETY: the function and its padding, made writable for this and put
    // back; nothing runs them yet. The value is written before the load
    // that reads it.
    unsafe {
        if VirtualProtect((base + from) as *const _, to - from, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return Err("Windows would not make it writable");
        }
        ((base + SLOT) as *mut f32).write_unaligned(seconds);
        std::ptr::copy_nonoverlapping(new_load().as_ptr(), (base + LOAD) as *mut u8, 8);
        VirtualProtect((base + from) as *const _, to - from, old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), (base + from) as *const _, to - from);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_load_is_of_the_longest_step() {
        // vmovss xmm3, [rip+disp32] at +5; vminss xmm0, xmm3, xmm1 at +0x14.
        assert_eq!(&BYTES[5..9], &[0xc5, 0xfa, 0x10, 0x1d]);
        assert_eq!(&BYTES[0x14..0x18], &[0xc5, 0xe2, 0x5d, 0xc1]);
        assert_eq!(old_target(), 0x6159ec0);
        assert_eq!(*BYTES.last().unwrap(), 0xc3);
        assert_eq!(AT + BYTES.len(), PADDING.start);
        let load = new_load();
        assert_eq!(&load[..4], &BYTES[5..9]);
        let disp = i32::from_le_bytes(load[4..8].try_into().unwrap());
        assert_eq!(LOAD as i64 + 8 + i64::from(disp), SLOT as i64);
        // cmp byte [rip+..], 0 follows the load.
        assert_eq!(&BYTES[LOAD - AT + 8..LOAD - AT + 10], &[0x80, 0x3d]);
    }

    #[test]
    fn milliseconds() {
        assert_eq!(parse_ms("125"), Some(0.125));
        assert_eq!(parse_ms("50"), None);
        assert_eq!(parse_ms("0"), None);
        assert_eq!(parse_ms("2000"), None);
        assert_eq!(parse_ms("x"), None);
    }
}
