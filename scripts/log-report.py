#!/usr/bin/env python3
"""Summarises victory-nvidia-*.log files (or .zip files holding them) and
compares them side by side, so that a session's result reads at a glance.

    python3 scripts/log-report.py session33.zip [session34.zip ...]

Per log:
- fps by draws a frame, in the same buckets for every log, so that light and
  heavy scenes compare like with like.
- The card's wait, how busy the card was, and the submitting thread's time.
- With `VICTORY_GPU_TIMING=1` (a diag build): the card's time by command,
  averaged over the run, heaviest first, and by kind of command.
- The barrier experiment (`VICTORY_BARRIER_TEST`) the run had, and its waits.
- What holds the windows of title memory (the memory lines).

With several logs, the fps buckets and the heaviest commands are also given
side by side (the first log is the baseline). Standard library only.
"""

import io
import re
import statistics
import sys
import zipfile
from collections import defaultdict

LINE = re.compile(r"^(\S+)\s+(INFO|WARN|ERROR|DEBUG)\s+(\S+): (.*)$")
FPS = re.compile(r"^([\d.]+) fps, ([\d.]+) ms/frame \(([\d.]+) preparing.*?; ([\d.]+) on the GPU;.*?\), (\d+) draws/frame")
CLOCK = re.compile(r"the card about (\d+)% busy.*?stood waiting for the card ([\d.]+) ms")
SUBMIT = re.compile(r"^preparing per frame, the submitting thread: ([\d.]+) ms in its submissions.*?after the sending ([\d.]+)")
COMMAND = re.compile(r"^\s*the card's time by command \(timing run\): ([\d.]+) ms a frame in ([\d.]+) commands a frame, the longest ([\d.]+) ms: (.*)$")
KINDS = re.compile(r"^\s*the card's time by command \(timing run\), all (\d+) kinds together: ([\d.]+) ms a frame; by first word: (.*)$")
KIND = re.compile(r"(\S+) ([\d.]+) ms in ([\d.]+)")
BARRIERS = re.compile(r"waits for every earlier shader write, per frame: ([\d.]+) for the title's barrier groups, ([\d.]+) for its barriers on buffers;.*barrier experiment (\d+)")
BY_WORK = re.compile(r"^\s*the card's time by what ran: ([\d.]+) ms a frame in ([\d.]+) submissions a frame: (.*)$")
PRIVATE = re.compile(r"process memory: (\d+) MB in RAM \(working set\), (\d+) MB private")
WINDOWS = re.compile(r"windows of title memory (\d+) with (\d+) MB.*?; (\d+) of them in the card's memory.*?card room for windows (\d+) MB")
WINDOWS_OLD = re.compile(r"windows of title memory (\d+) with (\d+) MB")
CARD_USE = re.compile(r"the card's own: budget (\d+) MB, in use (\d+) MB.*?the PC's shared with the card: budget \d+ MB, in use (\d+) MB")
TURN = re.compile(r"the card is kept awake, by turns.*\bon=(true|false)")

# Programs named in the fork notes, so that the heaviest commands read by name.
NAMES = {
    "02384a60fda6f2d0": "decal heap defrag (faces)",
    "e07a678d3f79d4cd": "irradiance cache allocation requests",
    "982f3cccd5c04294": "fog",
    "6d7fe7745ab9c6d8": "ray binning",
    "5cf00dc39042edaf": "shadow masks",
    "14264103296aa43a": "probes: register allocation requests",
    "4a1a6cda9de2974b": "probes: shade copy",
    "3592426e3e9142d8": "probes: gather V-buffer",
    "ce33eeaaf36bce05": "probes: gather copy",
    "cfd83392ec9ae408": "probes: gather copy",
    "7f61193d2c0e5f22": "weather particles begin-update",
    "6b82bc7d22016cb5": "collect triangles (meshlets)",
    "6832a9da15c2e584": "collect triangles",
    "a10179ab7eda0dc3": "water volume portals",
    "407eca669212d400": "water",
}
HEX = re.compile(r"\b[0-9a-f]{16}\b")

BUCKETS = [(0, 2000), (2000, 4000), (4000, 6000), (6000, 8000), (8000, 10000), (10000, 10**9)]


def group_name(text):
    """A short name for a "by what ran" group: its list count, the known
    programs in it by name, else its first pipeline."""
    lists = re.match(r"\s*(\d+) lists", text)
    pipes = re.findall(r"\[([^\]]*)\]", text)
    named = sorted({NAMES[h] for p in pipes for h in HEX.findall(p) if h in NAMES})
    head = ", ".join(named) if named else (pipes[0] if pipes else "no title pipelines (ours)")
    return f"{lists[1] if lists else '?'} lists, {len(pipes)} pipelines: {head}"


def bucket_name(b):
    lo, hi = b
    return f"{lo // 1000}k+" if hi >= 10**9 else f"{lo // 1000}-{hi // 1000}k"


def read_logs(path):
    """The text of the log(s) at `path`: a log, or the largest
    victory-nvidia log in a zip (the session's own; smaller ones are
    earlier starts)."""
    if path.endswith(".zip"):
        with zipfile.ZipFile(path) as z:
            logs = [i for i in z.infolist() if i.filename.endswith(".log") and "victory" in i.filename.rsplit("/", 1)[-1]]
            if not logs:
                logs = [i for i in z.infolist() if i.filename.endswith(".log")]
            if not logs:
                raise SystemExit(f"{path}: no .log inside")
            best = max(logs, key=lambda i: i.file_size)
            return best.filename, z.read(best).decode("utf-8", "replace")
    with open(path, encoding="utf-8", errors="replace") as f:
        return path, f.read()


class Run:
    def __init__(self, name, text):
        self.name = name
        self.frames = []  # (fps, ms, preparing, gpu wait, draws)
        self.busy = []
        self.stood = []
        self.submit = []
        self.after = []
        self.commands = defaultdict(lambda: [0.0, 0.0, 0.0, 0])  # ms sum, n sum, longest, lines
        self.kinds = defaultdict(lambda: [0.0, 0])
        self.kind_lines = 0
        self.barriers = []  # (groups, buffers, experiment)
        self.by_work = defaultdict(lambda: [0.0, 0])
        self.reports = 0  # copy-statistics reports (diag builds), each with its own "by what ran" lines
        self.private = []
        self.windows = []
        self.card_use = []
        self.switches = ""
        self.turn = None  # VICTORY_CARD_AWAKE=turns: whether the kernel runs now
        self.turned = []  # (fps, draws, on) of frame lines after the first turn
        for raw in text.splitlines():
            if raw.startswith("# console=") and not self.switches:
                self.switches = raw
            m = LINE.match(raw)
            if not m:
                continue
            msg = m.group(4)
            self.parse(msg)

    def parse(self, msg):
        if (m := FPS.match(msg)):
            fps, ms, prep, gpu, draws = float(m[1]), float(m[2]), float(m[3]), float(m[4]), int(m[5])
            self.frames.append((fps, ms, prep, gpu, draws))
            if self.turn is not None and draws >= 1500:
                self.turned.append((fps, draws, self.turn))
        elif (m := TURN.search(msg)):
            self.turn = m[1] == "true"
        elif (m := CLOCK.search(msg)):
            self.busy.append(int(m[1]))
            self.stood.append(float(m[2]))
        elif (m := SUBMIT.match(msg)):
            self.submit.append(float(m[1]))
            self.after.append(float(m[2]))
        elif (m := COMMAND.match(msg)):
            c = self.commands[m[4].strip()]
            c[0] += float(m[1])
            c[1] += float(m[2])
            c[2] = max(c[2], float(m[3]))
            c[3] += 1
        elif (m := KINDS.match(msg)):
            self.kind_lines += 1
            for k in KIND.finditer(m[3]):
                self.kinds[k[1]][0] += float(k[2])
                self.kinds[k[1]][1] += float(k[3])
        elif (m := BARRIERS.search(msg)):
            self.barriers.append((float(m[1]), float(m[2]), int(m[3])))
        elif msg.startswith("copies per frame:"):
            self.reports += 1
        elif (m := BY_WORK.match(msg)):
            w = self.by_work[group_name(m[3])]
            w[0] += float(m[1])
            w[1] += 1
        elif (m := PRIVATE.search(msg)):
            self.private.append(int(m[2]))
        elif (m := WINDOWS.search(msg)):
            self.windows.append((int(m[1]), int(m[2]), int(m[3]), int(m[4])))
        elif (m := WINDOWS_OLD.search(msg)):
            self.windows.append((int(m[1]), int(m[2]), None, None))
        elif (m := CARD_USE.search(msg)):
            self.card_use.append((int(m[1]), int(m[2]), int(m[3])))

    def gameplay(self):
        """Frame lines once the world is up: from the first over 1,500 draws."""
        for i, f in enumerate(self.frames):
            if f[4] >= 1500:
                return self.frames[i:]
        return []

    def bucket_fps(self):
        out = {}
        for b in BUCKETS:
            xs = [f[0] for f in self.gameplay() if b[0] <= f[4] < b[1]]
            if xs:
                out[b] = (statistics.median(xs), len(xs))
        return out

    def command_avg(self):
        """Per command, ms a frame averaged over the timing lines (a command
        absent from a line counts as 0 there)."""
        lines = max(self.kind_lines, max((c[3] for c in self.commands.values()), default=0), 1)
        return {k: (v[0] / lines, v[1] / lines, v[2]) for k, v in self.commands.items()}


def med(xs):
    return statistics.median(xs) if xs else float("nan")


def short(what, n=110):
    names = [NAMES[h] for h in HEX.findall(what) if h in NAMES]
    if names:
        what = f"({', '.join(names)}) {what}"
    return what if len(what) <= n else what[: n - 1] + "…"


def report(run):
    print(f"== {run.name}")
    g = run.gameplay()
    if run.switches:
        exp = re.findall(r"VICTORY_(?:GPU_TIMING|BARRIER_TEST|FRAMEGEN|FRAMEGEN_X|FSR_FRAMEGEN|CARD_AWAKE|FRAMES_IN_AIR)=\S+", run.switches)
        if exp:
            print("   switches of note:", " ".join(exp))
    if not g:
        print("   no gameplay frame lines (none over 1,500 draws)")
    else:
        print(
            f"   gameplay: {len(g)} lines, fps median {med([f[0] for f in g]):.1f} "
            f"(p10 {statistics.quantiles([f[0] for f in g], n=10)[0] if len(g) > 9 else float('nan'):.1f}), "
            f"ms/frame {med([f[1] for f in g]):.1f}, preparing {med([f[2] for f in g]):.1f}, "
            f"on the GPU {med([f[3] for f in g]):.1f}, draws {med([f[4] for f in g]):.0f}"
        )
        print("   fps by draws: " + ", ".join(f"{bucket_name(b)} {v[0]:.1f} ({v[1]})" for b, v in run.bucket_fps().items()))
    if run.turned:
        cells = []
        for b in BUCKETS:
            on = [f[0] for f in run.turned if b[0] <= f[1] < b[1] and f[2]]
            off = [f[0] for f in run.turned if b[0] <= f[1] < b[1] and not f[2]]
            if on and off:
                cells.append(f"{bucket_name(b)} on {med(on):.1f} ({len(on)}) / off {med(off):.1f} ({len(off)})")
        print("   CARD_AWAKE turns, fps by draws: " + (", ".join(cells) if cells else "too few lines in both turns"))
    if run.busy:
        print(f"   card busy median {med(run.busy):.0f}%, submitting thread stood waiting for the card {med(run.stood):.1f} ms a frame")
    if run.submit:
        print(f"   submitting thread {med(run.submit):.1f} ms a frame in its submissions, {med(run.after):.1f} after the sending")
    if run.private:
        w = ""
        if run.windows:
            n, mb, carded, card_mb = run.windows[-1]
            w = f", windows {n} with {mb} MB"
            if carded is not None:
                w += f" ({carded} of them, {card_mb} MB, in the card's memory; about {max(mb - card_mb, 0)} MB left in the PC's memory, read over the bus)"
        print(f"   private memory {run.private[0]} -> {run.private[-1]} MB (max {max(run.private)}){w}")
    if run.card_use:
        budget, used, shared = run.card_use[-1]
        print(f"   card memory at the last look: {used} of {budget} MB in use ({100 * used // max(budget, 1)}%), {shared} MB of the PC's shared with it")
    if run.barriers:
        groups = med([b[0] for b in run.barriers])
        buffers = med([b[1] for b in run.barriers])
        exps = sorted({b[2] for b in run.barriers})
        print(f"   barrier waits a frame: {groups:.0f} for barrier groups, {buffers:.0f} on buffers; experiment {exps} (0 none, 1 groups, 2 indirect, 3 both)")
    if run.kinds:
        lines = max(run.kind_lines, 1)
        total = sum(v[0] for v in run.kinds.values()) / lines
        kinds = sorted(run.kinds.items(), key=lambda kv: -kv[1][0])
        print(f"   card time by command, all kinds: {total:.1f} ms a frame; " + ", ".join(f"{k} {v[0] / lines:.1f} ms ({v[1] / lines:.0f})" for k, v in kinds[:8]))
    if run.commands:
        avg = sorted(run.command_avg().items(), key=lambda kv: -kv[1][0])
        print("   heaviest commands (ms a frame, commands a frame, longest ms):")
        for what, (ms, n, longest) in avg[:25]:
            print(f"     {ms:6.2f}  {n:6.1f}  {longest:6.2f}  {short(what)}")
    if run.by_work:
        # Each report lists its heaviest submissions only: averaged over all
        # reports, so a group that is heavy now and then is not overstated.
        reports = max(run.reports, max(n for _, n in run.by_work.values()))
        work = sorted(run.by_work.items(), key=lambda kv: -kv[1][0])
        total = sum(ms for ms, _ in run.by_work.values()) / reports
        print(f"   the card's time by what ran, over {reports} reports: {total:.1f} ms a frame in the listed submissions; heaviest:")
        for what, (ms, n) in work[:8]:
            print(f"     {ms / reports:6.1f}  (in {n} reports)  {what}")
    print()


def compare(runs):
    base = runs[0]
    print("== side by side (first is the baseline)")
    header = "   draws    " + "".join(f"{r.name[-28:]:>30}" for r in runs)
    print(header)
    fps = [r.bucket_fps() for r in runs]
    for b in BUCKETS:
        if not any(b in f for f in fps):
            continue
        cells = []
        for f in fps:
            if b in f:
                v = f[b][0]
                d = "" if f is fps[0] or b not in fps[0] else f" ({(v / fps[0][b][0] - 1) * 100:+.0f}%)"
                cells.append(f"{v:.1f} fps{d}")
            else:
                cells.append("-")
        print(f"   {bucket_name(b):<9}" + "".join(f"{c:>30}" for c in cells))
    timed = [r for r in runs if r.commands]
    if len(timed) >= 2:
        a, b = timed[0], timed[1]
        aa, bb = a.command_avg(), b.command_avg()
        keys = sorted(set(aa) | set(bb), key=lambda k: -(aa.get(k, (0,))[0] + bb.get(k, (0,))[0]))
        print(f"   heaviest commands, {a.name[-24:]} vs {b.name[-24:]} (ms a frame):")
        for k in keys[:20]:
            x, y = aa.get(k, (0, 0, 0))[0], bb.get(k, (0, 0, 0))[0]
            print(f"     {x:6.2f} {y:6.2f} {y - x:+6.2f}  {short(k, 90)}")
    print()


def main(paths):
    if not paths:
        print(__doc__)
        return 2
    runs = []
    for p in paths:
        name, text = read_logs(p)
        runs.append(Run(f"{p}:{name.rsplit('/', 1)[-1]}" if p.endswith(".zip") else name, text))
    for r in runs:
        report(r)
    if len(runs) > 1:
        compare(runs)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
