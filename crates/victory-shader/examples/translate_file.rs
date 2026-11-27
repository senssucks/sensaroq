//! Translates shader containers given as files (`.cdxil`, as the title has
//! them) and says what came of each: for working on a translator gap offline,
//! on any host. `VICTORY_HOST_WAVE=32` takes a host of 32-lane waves (`VICTORY_VIEW_LIMIT`: its
//! view heap's null slot, 999999 on NVIDIA);
//! `VICTORY_WAVE_WHY=1` says why each form of a wave64 program failed.
fn main() {
    if let Some(n) = std::env::var("VICTORY_HOST_WAVE").ok().and_then(|v| v.parse().ok()) {
        victory_shader::emit::set_host_max_wave(n);
    }
    if let Some(n) = std::env::var("VICTORY_VIEW_LIMIT").ok().and_then(|v| v.parse().ok()) {
        victory_shader::emit::set_view_heap_limit(n);
    }
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        match victory_shader::emit::translate(&bytes) {
            Ok(_) => println!("{path}: translated"),
            Err(e) => println!("{path}: {e}"),
        }
    }
}
