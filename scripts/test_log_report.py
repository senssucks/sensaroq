"""Tests of scripts/log-report.py on synthetic lines (no game logs):
    python3 scripts/test_log_report.py
"""

import importlib.util
import pathlib
import unittest

SPEC = importlib.util.spec_from_file_location("log_report", pathlib.Path(__file__).with_name("log-report.py"))
log_report = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(log_report)

T = "2026-11-27T21:10:45.108371Z  INFO victory_gpu::host: "


def run(*lines):
    return log_report.Run("synthetic", "\n".join(T + line for line in lines))


class Parse(unittest.TestCase):
    def test_frame_lines(self):
        r = run("8.8 fps, 113.2 ms/frame (55.3 preparing: copies 24.3 ms 51.2 MB, textures 3.1 ms 36.5 MB; 13.3 on the GPU; "
                "15.0 copying back 0.5 MB), 2755 draws/frame, dropped 426 without a host pipeline")
        self.assertEqual(r.frames, [(8.8, 113.2, 55.3, 13.3, 2755)])

    def test_windows_on_the_card(self):
        r = run("of it ours: buffers' copies in the card's memory 1108 MB, with as much of the PC's for their CPU side "
                "(0 MB of twins); windows of title memory 2312 with 3091 MB (1706 MB between the first and the last byte "
                "ever asked for in each; 1140 of them in the card's memory, with as much of the PC's), card room for "
                "windows 1271 MB, mapped 0 MB")
        self.assertEqual(r.windows, [(2312, 3091, 1140, 1271)])

    def test_windows_in_an_older_line(self):
        r = run("of it ours: windows of title memory 861 with 3552 MB")
        self.assertEqual(r.windows, [(861, 3552, None, None)])

    def test_card_memory(self):
        r = run("video memory as the system counts it, right after that change: the card's own: budget 14809 MB, in use "
                "11294 MB, 7770 MB could still be reserved, 0 MB reserved; the PC's shared with the card: budget 15513 MB, "
                "in use 4993 MB, 7884 MB could still be reserved, 0 MB reserved (the layer reserves none)")
        self.assertEqual(r.card_use, [(14809, 11294, 4993)])

    def test_awake_turns_split_frames(self):
        r = run("the card is kept awake, by turns: on=true",
                "9.0 fps, 111.0 ms/frame (50.0 preparing; 13.0 on the GPU; 1.0 copying back), 2000 draws/frame",
                "the card is kept awake, by turns: on=false",
                "8.0 fps, 125.0 ms/frame (50.0 preparing; 13.0 on the GPU; 1.0 copying back), 2100 draws/frame")
        self.assertEqual(r.turned, [(9.0, 2000, True), (8.0, 2100, False)])


if __name__ == "__main__":
    unittest.main()
