//! Translates shader containers given as files (`.cdxil`, as the title has
//! them) and says what came of each: for working on a translator gap offline,
//! on any host. `VICTORY_HOST_WAVE=32` takes a host of 32-lane waves (`VICTORY_VIEW_LIMIT`: its
//! view heap's null slot, 999999 on NVIDIA);
//! `VICTORY_WAVE_WHY=1` says why each form of a wave64 program failed;
//! `VICTORY_ATOMIC_SUB_LOOP=1` as the layer sets it; `VICTORY_DXIL_OUT=<dir>`
//! writes each translation there (`<file name>.dxil`).
fn main() {
    if let Some(n) = std::env::var("VICTORY_HOST_WAVE").ok().and_then(|v| v.parse().ok()) {
        victory_shader::emit::set_host_max_wave(n);
    }
    if let Some(n) = std::env::var("VICTORY_VIEW_LIMIT").ok().and_then(|v| v.parse().ok()) {
        victory_shader::emit::set_view_heap_limit(n);
    }
    if std::env::var_os("VICTORY_ATOMIC_SUB_LOOP").is_some_and(|v| v == "1") {
        victory_shader::emit::set_atomic_sub_loops(true);
    }
    let out = std::env::var_os("VICTORY_DXIL_OUT").map(std::path::PathBuf::from);
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        match victory_shader::emit::translate(&bytes) {
            Ok(shader) => {
                println!("{path}: translated ({} bytes of DXIL)", shader.dxil.len());
                if let Some(dir) = &out {
                    let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let _ = std::fs::write(dir.join(format!("{name}.dxil")), &shader.dxil);
                }
            }
            Err(e) => println!("{path}: {e}"),
        }
    }
}
