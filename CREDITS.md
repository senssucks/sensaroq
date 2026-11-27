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

### Peppacodes/playablegames-peppa

| Source | What | Author |
|---|---|---|
| [6035addd](https://github.com/Peppacodes/playablegames-peppa/commit/6035addd) | Card-copy census; `VICTORY_SKIP_WC_BACK` / `VICTORY_SKIP_WC_COMPARE` | Chipsrain |
| [9aa92ae2](https://github.com/Peppacodes/playablegames-peppa/commit/9aa92ae2) | Placed-copy fills read ahead, gather reads (`VICTORY_PLACED_FORWARD`, `VICTORY_CARD_GATHER`) | Diogo Sousa (SousaTrashBin) |
| [f9f62a4a](https://github.com/Peppacodes/playablegames-peppa/commit/f9f62a4a) | R9G9B9E5 views read the R32_UINT texture itself (`VICTORY_ALIAS_CAST`) | Diogo Sousa (SousaTrashBin) |
| [cb412956](https://github.com/Peppacodes/playablegames-peppa/commit/cb412956) | Landed lists reset on their own thread (`VICTORY_LAND_ASIDE`) | Peppacodes |
| [7f5d8261](https://github.com/Peppacodes/playablegames-peppa/commit/7f5d8261) | Windows never written stop being declared (`VICTORY_QUIET_WINDOWS`) | Peppacodes |

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
| [HumanlyDev/HUMAN PR #18](https://github.com/HumanlyDev/HUMAN/pull/18) | Dynamic descriptor slots as a bitset, searched 64 at a time (hand-ported) | HumanlyDev |

The NVIDIA fork watch that finds these lives in the project notes. New ports are added here as they land.
