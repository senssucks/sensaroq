<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/hero-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/readme/hero-light.svg">
  <img src="assets/readme/hero-light.svg" alt="Kelvaroq: Xbox Series compatibility runtime for Windows, written in Rust on DirectX 12">
</picture>

# Kelvaroq

[![Discord: live updates](https://img.shields.io/badge/Discord-live%20updates-5865F2?logo=discord&logoColor=white)](https://discord.gg/5Hvk5RFB3D) ![License](https://img.shields.io/github/license/stellarr-studio/kelvaroq) ![Platform: Windows x64](https://img.shields.io/badge/platform-Windows%20x64-lightgrey) ![Language: Rust](https://img.shields.io/badge/language-Rust-orange) ![Status: playable · ~10 fps](https://img.shields.io/badge/status-playable%20%C2%B7%20~10%20fps-green) ![Target: 30 fps @ 3840×2160](https://img.shields.io/badge/target-30%20fps%20%40%203840%C3%972160-blue) ![Tested on RTX 5080](https://img.shields.io/badge/tested%20on-RTX%205080-lightgrey) [![Built with Moon AI](https://img.shields.io/badge/built%20with-Moon%20AI-7c3aed)](https://realmoon.ai) ![Ranked #1 on a public tracker](https://img.shields.io/badge/public%20tracker-%231%20of%2044-gold)

Kelvaroq is an Xbox Series X|S compatibility runtime for Windows x64, written in Rust. Its native engine is built on VIctory and victory-fork. The goal is a playable scene at 3840×2160 and 30 rendered fps, with correct textures and steady frame pacing on NVIDIA and AMD GPUs.

> [!IMPORTANT]
> **You need your own copy of the game.** Kelvaroq contains and distributes no game files, no game code, no assets, no keys and no download links. It only runs a game that you own and that you installed yourself from your own lawful copy. We do not help anyone obtain a game: requests for game files, keys, dumps or download links are refused and removed, on GitHub and on Discord.

> **Built entirely with [Moon AI](https://realmoon.ai).** All of the code, the research, the reverse-engineering notes, the measurements and the documentation in this project are produced with the Moon AI platform. Kelvaroq is developed by [Stellarr Studio SAS](https://stellarrstudio.com).
>
> [![Join the Kelvaroq Discord](https://img.shields.io/badge/Join%20the%20Discord-follow%20every%20announcement%20live-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/5Hvk5RFB3D)
>
> **Ranked #1 of 44 projects** on an independent public tracker of PC compatibility efforts for this console title (check of 25 November 2026, 21:02 UTC): furthest along, two steps short of its "playable" bar of 20 fps and 30 minutes without a crash.

## Highlights

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/highlights-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/readme/highlights-light.svg">
  <img src="assets/readme/highlights-light.svg" alt="Unified memory, in testing: 100% of the game's graphics memory shared with the GPU from allocation; 97.4% of GPU memory views fit directly in shared memory; 64 million GPU lookups served with no copy in one live session; 121 million descriptor rows checked against real gameplay with zero errors; 300 times less memory bookkeeping per heavy frame (about 18 s to about 60 ms on an RTX 5080); 0 GB left behind after closing the game, down from about 19 GB.">
</picture>

- **100%** of the game's graphics memory is shared with the GPU from the moment it is allocated.
- **97.4%** of the GPU's memory views fit directly in that shared memory, measured on real gameplay.
- **64 million** GPU lookups served straight from the game's memory, with no copy, in one live session. That morning: zero.
- **Translate once, like the console:** each GPU descriptor is now translated once, when the game writes it, instead of being re-sorted on every submission. Per-submission work dropped **under 1 ms**.
- **121 million** descriptor rows checked against recorded real gameplay, with **zero errors**.
- **300× less** memory bookkeeping per heavy frame on real gameplay: **~18 s → ~60 ms** on an RTX 5080.
- **0 GB** left behind when the game closes, down from about 19 GB per session, with a clean shutdown in about one second.
- **Native DirectX 12 ray tracing (DXR)** is now implemented and working: the game's ray-tracing structures are built and traced through DXR on the GPU.

The unified memory layer is **fully functional and in testing**, and the remaining legacy components are being removed. It goes public as soon as the build is stable and clean. Follow it live on [Discord](https://discord.gg/5Hvk5RFB3D).

## The wall every project hits

A console game is native x64 code: it runs on a PC processor as it is. We measured it. With nothing drawn, the game's own code holds a locked **30 fps** in its heaviest prologue scene on our test PC (5,331 heavy frames, 99th percentile 33.9 ms). Draw the same scene through a compatibility layer and it takes about **130 ms** a frame.

Those extra 100 ms are not the game, and they are not the GPU, which sits idle most of the frame. About two thirds of them go to keeping two copies of memory in sync, every frame: watch the pages, copy what changed up to the GPU, copy what the GPU wrote back. The Xbox has one memory. A PC has two.

Every public project we have measured on the same machine is built on that copy loop, and most of them keep iterating on it: AI-generated switch sets, more threads, one more patch to the copy path. Each pass moves a few milliseconds, breaks something else, and the open world still runs at 2 to 12 fps. A loop that should not exist cannot be tuned away.

We traced how the game really uses its memory, from the first instruction of the process to the open world, and found why the earlier attempts to remove the loop failed. The blocker was never the game.

**Kelvaroq's memory layer is in testing now, and the first results are wild:** in the first scenes we tested, the memory copied every frame fell from over 100 MB to under 8 MB. We are tuning it for the open world before it ships.

**Update, 26 November 2026: the new memory management system is now fully functional.** In a live test session, the GPU found more than half of the memory it needed, about 64 million of 115 million lookups, directly in the game's own memory instead of a copy; earlier the same day, it found none. On recorded game data, translating each descriptor once, when the game writes it, brought the work per heavy frame from about 18 seconds down to about 60 ms, and the target is **5 ms per frame**. We are now removing the remaining components of the legacy system. Native DirectX 12 ray tracing (DXR) is implemented as well. The next priority is performance optimization, with a focus on lower-end hardware. The new layer is in testing and goes public as soon as the build is stable and clean. Follow it live on [Discord](https://discord.gg/5Hvk5RFB3D).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/memory-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/readme/memory-light.svg">
  <img src="assets/readme/memory-light.svg" alt="Simplified view: the usual approach copies the game's memory to the GPU and back every frame; Kelvaroq gives the GPU the game's memory as the console does. The new memory layer is in testing.">
</picture>

## Where this is going

- **Goal:** the game runs from boot to credits exactly as it does on an Xbox Series X, with what a PC adds on top: DLSS super resolution, frame generation, higher resolutions and an unlocked frame rate.
- **Modest PCs too, next priority:** the copy loop is what eats the processor today, and that is what shuts smaller machines out. Removing it lowers the bar for everyone. Once the legacy system is gone, performance optimization comes next, with the aim of a game that also runs well on mid-range and older PCs, not only on high-end ones. Minimum requirements will be published once they are measured.
- **Release policy:** the memory layer goes public as soon as its build is stable and clean and every test suite passes. DLSS, frame generation and the remaining visual effects follow. Until then, the public build stays on the current engine while the new layer is tested.

## Status

The figures below are those of the current public build.

**The prologue is playable at about 10 rendered fps in typical views and 7 to 8 fps in its heaviest scenes.** All measurements below are from the prologue at 3840×2160 on an RTX 5080. A heavy frame is a present with at least 3,000 draws.

Verified in play: the menus work, the character can be controlled, the HUD is shown, the game pauses, and sessions have run without a crash.

Patches 0041–0042 cut median heavy-frame time from 198.3 to 119.8 ms, and patches 0044–0048 to 108.5 ms. The thread that prepares and submits GPU work now limits heavy frames: the GPU waits for it. About 7% of draws are still dropped in this public build, mostly because ray-tracing structures are not built yet; native DirectX 12 ray tracing (DXR) is now implemented and working in the build under test. Textures wait for space when VRAM is full.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/charts/milestones-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/charts/milestones-light.svg">
  <img src="assets/charts/milestones-light.svg" alt="Median heavy-frame time fell from 254.9 ms before the port to 119.8 ms with patches 0041–0042 and 108.5 ms with patches 0044–0048. The 30 fps target is 33.3 ms.">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/charts/ab-0041-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/charts/ab-0041-light.svg">
  <img src="assets/charts/ab-0041-light.svg" alt="Median, 95th percentile and mean heavy-frame times all fell with patches 0041–0042.">
</picture>

The before and after runs used the same save and the same warm pipeline cache for 300 s each. Captures from both runs were checked for rendering errors. Frame times and mean fps in the table cover heavy frames. [Measurement protocol](documentation.md#mesure-de-la-scène-lourde) · [Mesh launch patch](documentation.md#travail-de-la-carte--dessins-mesh-lancés-par-la-carte-correctif-0041)

| Stage | Median | 95th percentile | Mean | Mean fps | Heavy frames | GPU busy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Experimental branch before the port | 254.9 ms | — | — | — | — | — |
| Port to victory-fork, 0001–0037 | ~235 ms | — | — | — | — | — |
| Memory and submission, 0038–0040 (before) | 198.3 ms | 312.4 ms | 221.1 ms | 4.5 | 959 | ~78% |
| Mesh launch and wide hash, 0041–0042 (after) | 119.8 ms | 207.8 ms | 137.0 ms | 7.3 | 1,564 | ~35% |
| In-process copies, changed pages, async present, 0044–0048 | 108.5 ms | 261.5 ms | 132.2 ms | 7.6 | 1,648 | ~30% |
| 30 fps target | 33.3 ms | — | — | — | — | — |

The three runs of the initial victory-fork port had heavy-frame medians of 215, 235 and 255 ms.

### External playtest: one 36-minute session

A tester played a build from this public repository and monitored it, on a Ryzen 9 7950X, an RTX 4090 and 32 GB of DDR5 RAM at 2560×1440. The session ran for **36 min 46 s without a crash**. The game was still open and responsive when monitoring stopped, and no new `*.crash.log` file appeared.

| Metric | Value |
| --- | ---: |
| Monitoring time | 36 min 46 s |
| Samples | 68 |
| Mean fps | 10 |
| Median fps | 11 |
| Max / min fps | 20 / 1 |
| Steady state, after 10 min | 10.9 fps mean, about 9 minimum |
| Frame time, steady state | about 92 ms |
| CPU use, steady state | about 15 % overall |

- **0 to 9 min, loading:** fps swung between about 1 and 20, with six samples at 4 fps or less. The last dip came at 8 min 45 s.
- **After 10 min:** fps stayed between about 9 and 12, with no further marked dip, for the remaining 26 min 46 s.
- **RAM:** a 15.5 GB peak at the start, then about 10.5 GB until 33 min, then about 7 GB until the end. Memory did not keep rising during this session, although one session cannot rule out a leak.

**Tester's verdict:** stable for 36 minutes, about 10 fps overall and 11 fps after loading, still far from smooth. An overall CPU use of about 15 % does not locate the bottleneck. A single saturated thread, the GPU or render synchronisation could each be the limit. Our own measurements, described above and in the documentation, point to the thread that prepares and submits GPU work.

**Limits:** one session only, covering the prologue and the first mission. The monitor was restarted once, and the first monitoring targeted the launcher process.

## Milestones

- [x] Inspect PE files and run the title's startup in a Unicorn-based CPU reference engine.
- [x] Run the title and render its first rooms with the native engine built on VIctory.
- [x] Pin victory-fork to `f85101b` and apply patches 0001–0037 for texture uploads and placement, parallel refresh and pipeline waits, among other changes.
- [x] Apply patches 0038–0040 to read table constants from the GPU copy, place read-only copies in CPU-visible VRAM through Resizable BAR, submit work on a dedicated thread and run copies on the copy engine.
- [x] Apply patches 0041–0042 so the GPU launches mesh draws through an amplification stage, eliminating the clamp pass and full waits between mesh draws. GPU hashing uses 64 threads per block. Together these changes cut median heavy-frame time by 40%.
- [x] Display the Kelvaroq name in the engine window and release the project as free software under the GPL.
- [x] Copy game memory in place instead of through the kernel (patch 0044): 7–12% less frame time at 5,000 draws and more; frame-time spikes over one second are gone.
- [x] Ask Windows for changed pages once per reservation and submission (patch 0045): heavy-frame median 129 → 110 ms in an A/B.
- [x] Present asynchronously, at most two frames ahead (patches 0046, 0048): the game no longer waits in present; p95 568 → 262 ms.
- [x] Shader preloading screen in the launcher, and a live performance monitor (`Kelvaroq.exe --monitor`).
- [ ] Remove the memory synchronisation that costs about two thirds of every heavy frame. *The new layer is fully functional and in testing (copies per frame down from over 100 MB to under 8 MB in the first scenes); the remaining legacy components are being removed.*
- [x] Native DirectX 12 ray tracing (DXR): ray-tracing structures are built and traced on the GPU, which removes the draws previously dropped for lack of them. Implemented and working in the build under test.
- [ ] Run 64-lane compute shaders on 32-lane GPUs.
- [ ] Address the remaining ~1,400 clamped indirect calls per frame across non-mesh draws and compute dispatches.
- [ ] Address the VRAM budget and missing host pipelines.
- [ ] DLSS super resolution and frame generation on NVIDIA RTX GPUs.
- [ ] Reach 30 rendered fps in the heavy prologue scene, then the whole game from boot to credits.

## Play (Windows package)

**Before you start:** you need a game you own, installed from your own lawful copy. Kelvaroq does not include, download or unlock any game.

1. Extract the entire Windows x64 ZIP into a folder.
2. Open **Kelvaroq.exe** and select the game folder or its executable. You can also drag either onto Kelvaroq.exe.
3. The launcher uses the native engine in the ZIP. The engine opens a rendering window and a Kelvaroq session log.

`Kelvaroq.exe --experimental-render` (or `kelvaroq play --experimental-render <folder>`) starts an older experimental profile that reserves up to 256 MiB for VRAM transfers and 128 MiB for small read-only resources. It is not faster on the RTX 5080; the default settings are recommended. User-defined `VICTORY_*` settings take precedence. [Memory and submission settings](documentation.md#placement-mémoire-et-soumission)

The native engine uses VIctory's loader, audio and input. Kelvaroq's Rust crate prepares textures and measures frame pacing. The package also includes the reference engine and its interpreter. You can move the folder as long as its files stay together.

When you select a folder, the workspace reads the Series executable declared in `MicrosoftGame.config`. If several executables are declared, select the intended executable directly. The original picker and CLI also accept a folder containing a single `.exe` when the manifest is missing.

The Series X profile presents to a 3840×2160 surface and fits the window to the desktop. Intermediate rendering passes can use other resolutions.

### Desktop launcher

Kelvaroq.exe opens the desktop workspace by default. Set `VICTORY_LAUNCHER_V2=0` to open the original picker. The workspace accepts the executable declared in MicrosoftGame.Config, including a declared executable in a subfolder. Drop a folder or executable onto the window to change the selection. `Kelvaroq.exe --ui-demo` opens the synthetic workspace. Add `--capture <directory>` to render the launcher, preload and monitor screens in English and French, in both themes.

Settings are stored in `%APPDATA%\Kelvaroq\settings` (schema 2). Storage moves create a dedicated child folder in the selected destination, verify its contents and switch the engine to the new location. Clearing shaders removes the translated shader directory. Clearing title data takes a recovery snapshot first.

Save history stores ZIP archives in `%APPDATA%\Kelvaroq\snapshots`. Each version 1 manifest records the launcher version, engine build, UTC creation time, optional label, directories, file sizes and SHA-256 digests. Restore and import validate the archive and snapshot the current data before replacing it. The default retention keeps the eight latest snapshots and the newest snapshot on each of the last fourteen UTC calendar days. Settings lets you change both limits.

Startup, Settings and About share one update check per 24 hours, including offline attempts. Settings remembers your choice to disable checks. A newer release appears in the workspace with its notes and a link to GitHub. The check sends only a standard HTTPS request with the launcher version in its User-Agent.

These switches default to enabled in the desktop workspace.

| Switch | Effect of setting it to 0 |
| --- | --- |
| `VICTORY_LAUNCHER_UPDATES` | Disable release checks and hide update controls. |
| `VICTORY_LAUNCHER_STORAGE` | Use the engine's standard storage locations or its inherited folder overrides. Hide storage actions. |
| `VICTORY_SAVE_HISTORY` | Hide save management and disable automatic snapshots. |
| `VICTORY_SAVE_SNAPSHOTS` | Skip pre-launch and clean-exit snapshots. Manual snapshots and recovery before restore remain available. |
| `VICTORY_LAUNCHER_DROP` | Disable drag-and-drop selection. |
| `VICTORY_LAUNCHER_I18N` | Use French and hide the language selector. |
| `VICTORY_LAUNCHER_NATIVE_STYLE` | Use the original auxiliary-window styling. |

The monitor starts disabled and attaches to the launched engine's process ID when selected. `VICTORY_LAUNCHER_MONITOR=0` suppresses it. Shader preload starts enabled and honours the existing `VICTORY_PIPELINE_PRECOMPILE=0` override. The desktop form uses the .NET Framework 4.8 WPF assemblies supplied by Windows. The package ships the desktop companion alongside the Rust entry point.

### Hardware profiles

The workspace shows the high-performance hardware adapter, its driver and dedicated VRAM, the CPU and its physical and logical core counts, RAM, and the Windows build. It checks feature level 12.0, Shader Model 6.6, mesh shaders, resource binding tier 3 and root signature 1.1. The engine builds directly indexed descriptor heaps. Settings also reports wave widths, enhanced barriers and GPU upload heap support. On discrete GPUs the upload heap query reports whether the engine can use CPU-visible VRAM. A separate advisory appears below 16 GiB of system memory because loading reached 15.5 GB in the measured session above.

Automatic uses the following limits for the preferred compatible discrete GPU. The memory classes use total physical memory reported by Windows, rounded to the nearest GiB. Texture residency continues to follow the current DXGI budget, and the engine already trims idle pipeline library pages according to available RAM.

| Engine setting | Smaller configurations | At least 10 GiB VRAM and 16 GiB RAM | At least 16 GiB VRAM and 32 GiB RAM |
|---|---:|---:|---:|
| `VICTORY_TEXTURE_STAGE_MB` per frame | 64 | 128 | 192 |
| `VICTORY_CARD_BUFFERS_MB` | 256 | 512 | 1024 |
| `VICTORY_QUEUE2_LANDING_MB` | 32 | 64 | 96 |

`VICTORY_WIDE_THREADS` uses half the physical core count, with a minimum of one worker and a maximum of twelve. Custom mode offers `VICTORY_WAVE64_PAIRED` on adapters that support 32 lanes and report a maximum of 32. It defaults to zero pending physical GPU validation. The launcher leaves inherited environment values in control and displays them in Settings.

On multi-GPU PCs the launcher profiles the preferred discrete device. Windows selects the adapter for each executable. Select **High performance** for `kelvaroq-engine.exe` in Windows Graphics settings to use that device. The engine checks the active adapter's wave width again before pairing. An unsupported or unverified graphics requirement leaves the engine settings at their inherited values.

The engine itself now makes its device on the first hardware adapter in DXGI's high-performance order that supports mesh shaders (patch 0065), so a PC with integrated graphics, such as a Ryzen G APU with the monitor on the motherboard, no longer renders the world on the integrated GPU. `VICTORY_ADAPTER` overrides the choice with an index in that order or part of the adapter's name, for example `VICTORY_ADAPTER=5060`.

Every 5 seconds the session log has a `dstorage: last 5.0 s` line with the streaming requests completed, failed requests, the slowest request and the requests still waiting (patch 0066). A world that stays gray while `waiting` grows or `failed` is not zero is starved of data rather than of shaders. `VICTORY_DSTORAGE_STATS=0` moves the line back to debug level.

Set `VICTORY_LAUNCHER_PROFILES=0` for detection alone. Set `VICTORY_LAUNCHER_HARDWARE=0` to restore the previous workspace and launch environment.

## Build from source

On Windows, use stable Rust with the x64 MSVC target, Visual Studio Build Tools with the Windows SDK, and 64-bit Python 3.11 or later. Run these commands in PowerShell from the repository root.

```powershell
cargo build --release --locked
python scripts/build_native.py
python -m venv .venv
.\.venv\Scripts\python -m pip install -e './research[dev]'
git config core.hooksPath .githooks
```

The native build script fetches the pinned revision, applies the rendering patches and adds `native/kelvaroq-render` to the build. To use an existing clone, run `python scripts/build_native.py --source <clone>`. The native engine can use VIctory's local saves and cache.

To rebuild the release ZIP on Windows, run `python scripts/package_windows.py`. The script includes third-party notices, verifies checksums and tests the extracted archive outside the repository. It writes the ZIP to `target/dist/`. The reference engine uses Unicorn under GPLv2. The package includes its notice and corresponding upstream source. [Unicorn license](https://github.com/unicorn-engine/unicorn/blob/2.1.4/COPYING)

## Command line

`inspect` reports a PE file's hash, entry point, sections and imports. With `--json`, it also reports the addresses needed for startup. `boot` executes instructions in guest memory. Replace the example path with the path to your local file.

```powershell
$image = 'D:\images\example.exe'
.\target\release\kelvaroq inspect $image
.\target\release\kelvaroq boot $image --python .\.venv\Scripts\python.exe
.\target\release\kelvaroq play $image
.\target\release\kelvaroq gpu
```

`boot` writes a JSON report. Exit code 0 means the entry point returned, 2 means guest execution stopped with a diagnostic, and 1 indicates a tool or loading error.

`play` uses the native engine installed at `native/kelvaroq-engine.exe` beside the CLI, with a fallback to the previous executable name. `play --reference` opens the debugger. The CLI also opens the debugger in a development build without a native backend.

Both commands default to the Series X profile and accept `--console series-s`. The manifest selects the SMT setting. To compare CPU discovery results, `--host-topology` exposes the host PC's Windows topology records and `--processor-info <capture.bin>` replays raw x64 records. Keep `kelvaroq_simd.dll` beside the CLI. The standalone graphics probe accepts `gpu --adapter 0`.

## Shader check on the RTX 5080

The reference check builds two pinned revisions of the GFX10 translator, runs the programs on the named hardware adapter and compares the values read back. It covers eight sign-extension cases and two programs that select different textures per lane. On the development RTX 5080, all eight integer results and all 64 texture samples matched.

```powershell
python scripts/check_shaders.py --adapter 'RTX 5080'
```

Git and Cargo are required. The first run fetches the revisions and caches the Git objects under `target/shader-reference/`. The `--source` option accepts a local clone containing both commits. The check validates the external translator on controlled programs. During native execution, the backend also translates shaders when it creates pipelines.

## Documentation

These documents are written in French.

- [Architecture, compatibility criteria and measurement protocol](documentation.md)
- [Contribution guidelines for building, reviewing and testing changes](CONTRIBUTING.md)

## Community

Announcements, test results and new builds are posted first on the Kelvaroq Discord.

[![Join the Kelvaroq Discord](https://img.shields.io/badge/Discord-join%20the%20server-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/5Hvk5RFB3D)

## Contributing

Contributions are welcome here: open an issue or a pull request against this repository. Please contribute upstream instead of republishing copies of the code. Split efforts are what keep every project in this space at a few frames per second.

Kelvaroq is free software under the GPL-3.0-or-later, and that licence allows forks. It does not cover the **Kelvaroq name and logo**: a fork or a redistributed build must not use them, nor present itself as an official Kelvaroq release. Official builds come only from this repository and from Stellarr Studio SAS.

## Credits

Kelvaroq is developed by **Stellarr Studio SAS**, a French company ([stellarrstudio.com](https://stellarrstudio.com)). All of its code, research and documentation are produced with the [Moon AI](https://realmoon.ai) platform.

- **VIctory**, by Marvisak and contributors, provides the native engine's loader, runtime services, Direct3D 12 renderer and shader recompiler. Its license is GPL-3.0-or-later. [VIctory](https://github.com/Marvisak/VIctory)
- **victory-fork**, by fxceless and contributors, provides the pinned revision to which Kelvaroq applies `native/patches`. Its license is GPL-3.0-or-later. [victory-fork](https://github.com/fxceless/victory-fork)
- **Unicorn** runs the CPU reference engine and is licensed under GPLv2. [Unicorn](https://github.com/unicorn-engine/unicorn)

Kelvaroq provides the launcher, the `native/kelvaroq-render` rendering crate, the patches in `native/patches`, the reference engine and the measurement tools.

## License

Kelvaroq is free software. Copyright © 2026 Stellarr Studio SAS and the Kelvaroq contributors.

- The repository is licensed under **GPL-3.0-or-later**, the license of VIctory, from which the patches and the native engine in the Windows package derive. [Repository license](LICENSE)
- The `research/` directory is licensed under **GPL-2.0-or-later** because the reference engine imports Unicorn, which is licensed under GPLv2 only. [Research license](research/LICENSE)

The Windows package includes the native engine source in `sources/native.zip` and the third-party notices. The program comes without warranty to the extent permitted by the license.

## Disclaimer

This repository contains no game files, game code, game assets, encryption keys or links to them. You supply the game from your own lawfully obtained copy; Kelvaroq does not decrypt, download or unlock games and does not bypass any copy protection. Kelvaroq is not affiliated with or endorsed by any console maker, game developer or publisher. All trademarks belong to their owners.

Issues, pull requests and Discord messages that share or ask for game files, keys, dumps or download links are removed, and their authors may be banned.

## Legal Notice — Software Interoperability and European Union Law

This project is an independent, unaffiliated, open-source software interoperability initiative maintained by Stellarr Studio SAS, a company registered in France ([stellarrstudio.com](https://stellarrstudio.com)), and by developers residing in the European Union.

Its purpose is to research, document and independently implement software interfaces and compatibility mechanisms that enable interoperability between independently developed software and existing computer programs.

### European Legal Framework

The project acknowledges and relies, where applicable, on the statutory rights and exceptions established by Directive 2009/24/EC of the European Parliament and of the Council of 23 April 2009 on the legal protection of computer programs, particularly:

- **Article 1(2):** Exclusion of underlying ideas, principles and interface principles from copyright protection as such.
- **Article 5(2) and (3):** Statutory exceptions concerning necessary backup copies and the observation, study and testing of computer programs.
- **Article 6:** Decompilation where indispensable to achieve the interoperability of independently created computer programs, subject to the conditions established therein.
- **Article 8(2):** Invalidity of contractual provisions contrary to the aforementioned mandatory exceptions.

Where French law applies, these principles are implemented notably through Articles L.122-6-1 and L.331-5 of the French Intellectual Property Code.

Relevant case law includes:

- CJEU, *SAS Institute Inc. v World Programming Ltd*, C-406/10, 2 May 2012.
- CJEU, *Nintendo Co. Ltd and Others v PC Box Srl and 9Net Srl*, C-355/12, 23 January 2014.
- CJEU, *Top System SA v État belge*, C-13/20, 6 October 2021.
- CJEU, *Sony Computer Entertainment Europe Ltd v Datel Design and Development Ltd and Others*, C-159/23, 17 October 2024.

### Intellectual Property and Lawful Use

This project does not claim ownership of any third-party intellectual property.

The repository does not distribute proprietary game binaries, copyrighted game assets, proprietary source code, unauthorized cryptographic keys, or unauthorized copies of third-party software.

Users must independently obtain and hold a lawful right to use any third-party software required by this project. Any files supplied to the project must be lawfully obtained and used in accordance with applicable legislation.

The use of leaked, stolen, or otherwise unlawfully distributed software is not supported or endorsed.

This project does not authorize or encourage copyright infringement, unauthorized distribution, or unlawful circumvention of technological protection measures.

### Independent Development

Contributions must respect applicable intellectual property laws. Reverse engineering and decompilation activities must remain within the scope of applicable statutory exceptions.

The project's open-source license applies exclusively to original contributions and material for which the project has the necessary distribution rights.

All third-party trademarks and intellectual property remain the property of their respective owners. No affiliation, endorsement, or sponsorship is implied.

### Rights Holder Inquiries

Rights holders who believe that specific repository content infringes their intellectual property rights are invited to identify the relevant files, code sections, protected works, and legal grounds for their claim through the project's designated contact channel ([stellarrstudio.com](https://stellarrstudio.com)).

The maintainers are committed to reviewing legitimate intellectual property concerns in good faith while reserving all rights and statutory exceptions available under applicable law.

Takedown notices, including notices under the U.S. Digital Millennium Copyright Act (DMCA), can be sent to Stellarr Studio SAS through [stellarrstudio.com](https://stellarrstudio.com) or through GitHub's own process. Please contact us first: we review every notice promptly and remove any content that turns out to belong to a third party.

Nothing in this notice is intended to exclude applicable laws or confer rights beyond those recognized by the relevant jurisdiction.

### Legal References

- Directive 2009/24/EC: https://eur-lex.europa.eu/eli/dir/2009/24/oj
- Directive 2001/29/EC: https://eur-lex.europa.eu/eli/dir/2001/29/oj
- Regulation (EU) 2022/2065 — Digital Services Act: https://eur-lex.europa.eu/eli/reg/2022/2065/oj
- French Intellectual Property Code: https://www.legifrance.gouv.fr/codes/id/LEGITEXT000006069414/
