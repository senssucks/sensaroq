# Credits

sensaroq is built from the work of many forks of
[VIctory](https://github.com/Marvisak/VIctory). Its base is
[trustno1-coder/VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA),
which builds on [readmet3xt-lab/foxhound](https://github.com/readmet3xt-lab/foxhound),
[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX),
[fxceless/victory-fork](https://github.com/fxceless/victory-fork) and the projects their READMEs
name. Before that it was a copy of
[stellarr-studio/kelvaroq](https://github.com/stellarr-studio/kelvaroq).

Every project below is licensed GPL-3.0 (VIctory and its forks: GPL-3.0-or-later),
and so is sensaroq: see [LICENSE](LICENSE). Code taken from them keeps its
authors in the git history (cherry-picked commits keep their author and name
the source commit with `cherry picked from commit ...`). Code from
[XCL](https://github.com/ivall/xcl) (GPL-2.0-only) is **not** copied here, because
that licence cannot be combined with GPL-3.0. XCL is used as a reference only.

## Changes ported into sensaroq, by source

Each change is in sensaroq's history with the source commit or pull request it came from.

### avatarmanz/VIctory-7900XTX

| Source | What | Author |
|---|---|---|
| [4fd34fdd](https://github.com/avatarmanz/VIctory-7900XTX/commit/4fd34fdd) | Tiled cascade shadows translated exactly on 32-lane GPUs | sawetco (with Bruno Magalhães) |
| [b4ba4bbb](https://github.com/avatarmanz/VIctory-7900XTX/commit/b4ba4bbb), [9576c45f](https://github.com/avatarmanz/VIctory-7900XTX/commit/9576c45f) | The light stand-in settles by the clock (`VICTORY_AMBIENT_SETTLE`), hand-ported | avatarmanz |
| [77137b98](https://github.com/avatarmanz/VIctory-7900XTX/commit/77137b98) | The light stand-in with ray tracing, in a band (`VICTORY_AMBIENT_RT`) | avatarmanz |
| [631400f5](https://github.com/avatarmanz/VIctory-7900XTX/commit/631400f5), [3e04ca0a](https://github.com/avatarmanz/VIctory-7900XTX/commit/3e04ca0a), [15c3d0ea](https://github.com/avatarmanz/VIctory-7900XTX/commit/15c3d0ea) | In-place indirect arguments clamped in order (`VICTORY_PORT_CONSTANTS_ORDER`) | avatarmanz |
| [e151e71f](https://github.com/avatarmanz/VIctory-7900XTX/commit/e151e71f), [45cad4c5](https://github.com/avatarmanz/VIctory-7900XTX/commit/45cad4c5), [4d7185ed](https://github.com/avatarmanz/VIctory-7900XTX/commit/4d7185ed) | Output size and render scale (`VICTORY_OUTPUT`, `VICTORY_RENDER_SCALE`) | avatarmanz |
| [#112](https://github.com/avatarmanz/VIctory-7900XTX/issues/112), [#143](https://github.com/avatarmanz/VIctory-7900XTX/issues/143) | `VICTORY_THIN` off on NVIDIA; `VICTORY_FAST_FLOAT` (lead 2 of #143) | the reporters of those issues |

### fxceless/victory-fork

| Source | What | Author |
|---|---|---|
| [PR #501](https://github.com/fxceless/victory-fork/pull/501) | No blending on integer render targets | Yorivel |
| [PR #683](https://github.com/fxceless/victory-fork/pull/683) | A loop limit for compute programs alone | Danigamer05 |
| [#1379](https://github.com/fxceless/victory-fork/issues/1379) | The pipeline skip list as a set | the reporter of that issue |
| [PR #1044](https://github.com/fxceless/victory-fork/pull/1044) | DirectStorage workers per physical core (`VICTORY_DSTORAGE_PHYSICAL`, off by default here) | Key |
| [PR #1224](https://github.com/fxceless/victory-fork/pull/1224) | Retired host objects dropped off the landing thread (`VICTORY_BURY_ASIDE`, off by default), from Steenax6/VIctory-NVID | Steenax6 |

### Peppacodes/playablegames-peppa

| Source | What | Author |
|---|---|---|
| [6035addd](https://github.com/Peppacodes/playablegames-peppa/commit/6035addd) | Card-copy census; `VICTORY_SKIP_WC_BACK` / `VICTORY_SKIP_WC_COMPARE` | Chipsrain |
| [9aa92ae2](https://github.com/Peppacodes/playablegames-peppa/commit/9aa92ae2) | Placed-copy fills read ahead, gather reads (`VICTORY_PLACED_FORWARD`, `VICTORY_CARD_GATHER`) | Diogo Sousa (SousaTrashBin) |
| [f9f62a4a](https://github.com/Peppacodes/playablegames-peppa/commit/f9f62a4a) | R9G9B9E5 views read the R32_UINT texture itself (`VICTORY_ALIAS_CAST`) | Diogo Sousa (SousaTrashBin) |
| [cb412956](https://github.com/Peppacodes/playablegames-peppa/commit/cb412956) | Landed lists reset on their own thread (`VICTORY_LAND_ASIDE`) | Peppacodes |
| [7f5d8261](https://github.com/Peppacodes/playablegames-peppa/commit/7f5d8261) | Windows never written stop being declared (`VICTORY_QUIET_WINDOWS`) | Peppacodes |

### trustno1-coder/VIctory-NVIDIA (pull requests after sensaroq was based on it)

| Source | What | Author |
|---|---|---|
| [PR #57](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/57) | Pooled upload buffers for list copies; pressure keeps 128 MiB of spares (`VICTORY_UPLOAD_POOL_MB`, off by default here) | Yorivel |
| [PR #59](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/59) | Texture reads under an epoch check, not the mapping lock (`VICTORY_REFRESH_LOCKLESS`); one mapping read for up to eight textures (`VICTORY_REFRESH_BATCH`) | superboynick |
| [PR #87](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/87) | The mapping lock in 2 MiB address stripes (`VICTORY_MAPPING_STRIPES`) | superboynick |
| [PR #44](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/44) | Physical pages from sections of at most n MiB (`VICTORY_SECTION_MB`) | CLHelper |
| [PR #81](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/81) | Library saves wait out bursts and memory pressure (`VICTORY_LIBRARY_SAVE_LATER`, off by default here) | harrisotiropoulos-jpg |
| [PR #127](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/127) | A submission's one asking begun beside its first steps (`VICTORY_ASK_FIRST`, off by default) | 1mosce |
| [PR #139](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/139) | Retired objects in sixteen queues (`VICTORY_RETIRED_SHARDS`, off by default here); the kept-thread count settable (`VICTORY_WIDE_THREADS`) | dcastell1 |
| [PR #126](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/126) | Refreshes handed to the kept threads several at a time (`VICTORY_WIDE_CHUNK`) | 1mosce |
| [PR #328](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/328) | Ray walkers reuse a 64 KiB page's lookup (`VICTORY_MEMORY_NEAR`, off by default here), from fxceless 299fba | wike1701 |
| [PR #326](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/326) | Water tile slots a lane on 32-lane cards, scalar loop skips kept, one more water program wave-wide (`VICTORY_WATER_TILE_SLOTS`, `VICTORY_WAVE_LOOP_SKIPS`, `VICTORY_WATER_PORTALS_WIDE`, all off by default here) | wike1701 |
| [PR #358](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/358) | Independent card maintenance jobs batched (`VICTORY_MAINTENANCE_BATCHES`, off by default) | krofty |
| [PR #378](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/378) | Face relocation (decal defrag, 02384a60fda6f2d0): each console wave's two halves meet on their own, no group barrier (`VICTORY_DEFRAG_HALF_MEETS`, off by default here), from DoubleGhosty/cool_fox (Davidjc13) | liuhVIP |
| [PR #77](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/77) | Audio DSP mono convolutions: TVs, phone calls and reverb heard (`VICTORY_DSP_CONVOLVE`, off by default here; `VICTORY_DSP_GAIN`), same as fxceless #1491 | Omnicrash |
| [PR #63](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/63), [PR #393](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/393) | The sky's light in reflections bounded by the light around where the miss program has no depth (glossy eyes, glass and floors indoors; `VICTORY_AMBIENT_SPECULAR_ALPHA=bound`, `VICTORY_AMBIENT_SPECULAR_NO_DEPTH=1`, off by default here) | wasonidas, tippythetaidum |
| [PR #388](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/388) | CS_CollectTriangles(_Meshlets) (6832a9da15c2e584, 6b82bc7d22016cb5) translate on 32-lane cards as local waves with LDS-atomic reservations, instead of being dropped (`VICTORY_COLLECT_LOCAL`, off by default here) | fromrivertothesea |
| [PR #550](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/550) | The code constant table starts after the instructions, so NVIDIA keeps it in constant memory instead of every thread's local memory (`VICTORY_CODE_TABLE_TAIL`, off by default here); fix by desplmfao (Marvisak/VIctory #2143) | kibagami-nc |
| [PR #537](https://github.com/trustno1-coder/VIctory-NVIDIA/pull/537) | An opt-in `fast` build profile: thin LTO, one codegen unit (its release-profile half; not its panic = "abort") | hqjake |

### stellarr-studio/kelvaroq (native patches)

| Source | What | Author |
|---|---|---|
| [0022-gpu-upload-inputs](https://github.com/stellarr-studio/kelvaroq/blob/main/native/patches/0022-gpu-upload-inputs.patch) | Shader inputs only the CPU writes in card memory the CPU maps (`VICTORY_GPU_UPLOAD_READONLY_MB`, off by default), re-done on our mapped card buffers | stellarr-studio |

### xcl-dev/xcl (ideas only, re-written; XCL is GPL-2.0-only)

| Source | What | Author |
|---|---|---|
| [PR #68](https://github.com/xcl-dev/xcl/pull/68) | `v_fract_f32` of an infinity or NaN is NaN | the PR's author; written anew here |

### Others

| Source | What | Author |
|---|---|---|
| [PastiiJay/klbf-lab PR #41](https://github.com/PastiiJay/klbf-lab/pull/41) | DXIL optimiser: folds, loop-invariant hoisting, LDS forwarding, barrier removal | Vladimir (Vladimir-Human) |
| [rorodev6/victory-nvidia-megafork 77e13a97](https://github.com/rorodev6/victory-nvidia-megafork/commit/77e13a97) | `VICTORY_NAN_SCRUB`, hand-ported; `VICTORY_THIN=0` on NVIDIA (79f36b3) | rorodev6 |
| [ZigZagBoi1/ScarlettPort PR #44](https://github.com/ZigZagBoi1/ScarlettPort/pull/44) | Refreshes skip write-watch planning under a kept asking | ifBars |
| [ZigZagBoi1/ScarlettPort PR #72](https://github.com/ZigZagBoi1/ScarlettPort/pull/72) | The write-watch journal finds asked-ahead ranges by binary search | FictionHub |
| [ZigZagBoi1/ScarlettPort PR #86](https://github.com/ZigZagBoi1/ScarlettPort/pull/86) | Loose windows looked up under the shared lock | aswdffdas-sudo |
| [TaynanGT/Vice-Tay PR #191](https://github.com/TaynanGT/Vice-Tay/pull/191) | AVX2/SSE2 micro-tile texture detiler and worker pool (1.6-4.6x on large textures) | Vladimir |
| [DoubleGhosty/cool_fox PR #47](https://github.com/DoubleGhosty/cool_fox/pull/47) | Title memory reads zero only the bytes no run covers | Esteban Viera |
| [DoubleGhosty/cool_fox PR #49](https://github.com/DoubleGhosty/cool_fox/pull/49) | Per-draw maps keyed by address use AddressHash, not SipHash | Esteban Viera |
| [DoubleGhosty/cool_fox PR #18](https://github.com/DoubleGhosty/cool_fox/pull/18) | Idle times for staging and windows, the most a read buffer keeps (`VICTORY_STAGING_IDLE_MS`, `VICTORY_WINDOW_IDLE_MS`, `VICTORY_READ_KEEP_MB`; off by default) | Esteban Viera |
| [DoubleGhosty/cool_fox PR #20](https://github.com/DoubleGhosty/cool_fox/pull/20) | The most spare scratch chunks and send buffers kept (`VICTORY_SCRATCH_SPARE_MB`, `VICTORY_SEND_SPARE_MB`; unset as before) | Esteban Viera |
| [DexNom/VIctory PR #149](https://github.com/DexNom/VIctory/pull/149) | Atomic subtractions on group memory as compare-exchange loops, so NVIDIA makes the pipeline (weatherParticle CS_BeginUpdate, 7f61193d2c0e5f22, refused as out of memory; `VICTORY_ATOMIC_SUB_LOOP`, off by default here; also Marvisak/VIctory #3619) | notquiteog |
| [DoubleGhosty/cool_fox issue #94](https://github.com/DoubleGhosty/cool_fox/issues/94) | Idea: named wave64 programs taken wave-wide on 32-lane cards, which brought the water passes back on an RTX 5080 (`VICTORY_WAVE_WIDE_ONLY`, written anew, unset = none) | cool_fox reporter |
| [HumanlyDev/HUMAN PR #18](https://github.com/HumanlyDev/HUMAN/pull/18) | Dynamic descriptor slots as a bitset, searched 64 at a time (hand-ported) | HumanlyDev |

The NVIDIA fork watch that finds these lives in the project notes. New ports are added here as they land.
