"""Sums up a play log's frame times, so two runs can be compared by numbers.

    python scripts/log_summary.py <log> [<log> ...] [--min-draws 1000] [--skip-s 60]

Reads the layer's 5-second lines (`NN fps, NN ms/frame (NN preparing: copies ...`) and the submitting
thread's breakdown (`preparing per frame, the submitting thread: ...`). Only in-world samples count:
those with at least --min-draws draws a frame (menus and loading screens draw a few dozen), after the
first --skip-s seconds of them (shader compiles and first streaming). For each log it prints the
median and spread of the rendered frame rate and frame time, and the median of each part of the CPU's
preparation, which is where the time goes on most PCs.

The 5-second lines are means over 5 s, so p95 here is the slowest 5-second stretches, not single
frames: a run with fewer slow stretches is the steadier one.
"""

import argparse
import re
import statistics
import sys
from datetime import datetime

ANSI = re.compile(r"\x1b\[[0-9;]*m")
FRAME = re.compile(
    r"^(?P<t>\S+)\s+INFO victory_gpu::host: (?P<fps>[\d.]+) fps, (?P<ms>[\d.]+) ms/frame "
    r"\((?P<prep>[\d.]+) preparing: copies (?P<copy_ms>[\d.]+) ms (?P<copy_mb>[\d.]+) MB, "
    r"textures (?P<tex_ms>[\d.]+) ms (?P<tex_mb>[\d.]+) MB; (?P<gpu>[\d.]+) on the GPU; "
    r"(?P<back_ms>[\d.]+) copying back [\d.]+ MB\), (?P<draws>\d+) draws/frame, "
    r"dropped (?P<dropped>\d+) without a host pipeline \((?P<making>\d+) being made\)"
)
SUBMIT = re.compile(
    r"^(?P<t>\S+)\s+INFO victory_gpu::host: preparing per frame, the submitting thread: "
    r"(?P<total>[\d.]+) ms in its submissions: (?P<watch>[\d.]+) asking the write watch.*?"
    r"(?P<steps>[\d.]+) in the lists' steps \(refreshes (?P<refresh>[\d.]+) in .*?"
    r"(?P<card>[\d.]+) for copies in the card's memory, (?P<send>[\d.]+) sending the lists"
)


def when(stamp):
    return datetime.fromisoformat(stamp.rstrip("Z")[:26])


def pct(values, p):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(round(p / 100 * (len(ordered) - 1))))]


def summarize(path, min_draws, skip_s):
    frames, submits = [], []
    switches = ""
    with open(path, encoding="utf-8", errors="replace") as log:
        for raw in log:
            line = ANSI.sub("", raw)
            if line.startswith("# console="):
                switches = line.strip()
            elif m := FRAME.match(line):
                frames.append({k: (v if k == "t" else float(v)) for k, v in m.groupdict().items()})
            elif m := SUBMIT.match(line):
                submits.append({k: (v if k == "t" else float(v)) for k, v in m.groupdict().items()})
    world = [f for f in frames if f["draws"] >= min_draws]
    if not world:
        return f"{path}: no in-world samples (none with {min_draws}+ draws a frame)\n"
    start = when(world[0]["t"])
    world = [f for f in world if (when(f["t"]) - start).total_seconds() >= skip_s] or world
    first, last = when(world[0]["t"]), when(world[-1]["t"])
    stamps = {f["t"][:19] for f in world}
    subs = [s for s in submits if s["t"][:19] in stamps] or submits

    def med(rows, key):
        return statistics.median(r[key] for r in rows) if rows else float("nan")

    fps = [f["fps"] for f in world]
    ms = [f["ms"] for f in world]
    out = [
        f"{path}",
        f"  in world: {len(world)} samples, {(last - first).total_seconds() / 60:.1f} min",
        f"  fps        median {statistics.median(fps):6.1f}   p10 {pct(fps, 10):6.1f}   p90 {pct(fps, 90):6.1f}",
        f"  ms/frame   median {statistics.median(ms):6.1f}   p90 {pct(ms, 90):6.1f}   p95 {pct(ms, 95):6.1f}   worst {max(ms):7.1f}",
        f"  draws/frame median {med(world, 'draws'):.0f}, dropped without a pipeline {med(world, 'dropped'):.0f}",
        f"  CPU preparing {med(world, 'prep'):.1f} ms: title copies {med(world, 'copy_ms'):.1f} ms "
        f"({med(world, 'copy_mb'):.1f} MB), textures {med(world, 'tex_ms'):.1f} ms ({med(world, 'tex_mb'):.1f} MB); "
        f"GPU {med(world, 'gpu'):.1f} ms; copying back {med(world, 'back_ms'):.1f} ms",
    ]
    if subs:
        out.append(
            f"  submitting thread {med(subs, 'total'):.1f} ms: refreshes {med(subs, 'refresh'):.1f}, "
            f"other list steps {med(subs, 'steps') - med(subs, 'refresh'):.1f}, write watch {med(subs, 'watch'):.1f}, "
            f"card copies {med(subs, 'card'):.1f}, sending {med(subs, 'send'):.1f}"
        )
    if switches:
        out.append("  " + switches[:2000])
    return "\n".join(out) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("logs", nargs="+")
    parser.add_argument("--min-draws", type=int, default=1000)
    parser.add_argument("--skip-s", type=float, default=60)
    parser.add_argument("--no-switches", action="store_true", help="leave out the switches line")
    args = parser.parse_args()
    for path in args.logs:
        text = summarize(path, args.min_draws, args.skip_s)
        if args.no_switches:
            text = "\n".join(l for l in text.splitlines() if not l.startswith("  # console=")) + "\n"
        sys.stdout.write(text + "\n")


if __name__ == "__main__":
    main()
