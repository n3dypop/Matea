"""Regression-test the replay oracle against the previously missed margin case."""
import hashlib
import json
from pathlib import Path
import unittest
from check_color_trace import replay, verify, verify_fallback

ROOT = Path(__file__).resolve().parents[1]


class MarginReplayTests(unittest.TestCase):
    def test_retained_unicode_fallback_exposes_unstyled_continuation(self):
        document = json.loads((ROOT / "docs/evidence/optimization/fallback/before-trace.json").read_text(encoding="utf-8"))
        with self.assertRaises(AssertionError):
            verify_fallback(document)

    def test_explicit_padding_keeps_attributes_and_background_at_the_right_edge(self):
        traces = []
        for profile in ["none", "truecolor"]:
            style = "\x1b[1;7m" if profile == "none" else "\x1b[1;7;38;2;95;135;175;48;2;175;95;135m"
            reset = "\x1b[0m\x1b[2J\x1b[H\x1b[?25l"
            sequences = [reset + style + "? x?", reset + "q", reset + "\x1b[2;7H" + style + "? ",
                         reset + style + "a\x1b[2H? \x1b[2H\x1b[?25h"]
            traces.append({"profile": profile, "width": 8, "height": 2, "sequences": sequences})
        verify_fallback({"kind": "unicode_fallback", "traces": traces})

    def test_retained_crlf_trace_only_passes_with_default_margins(self):
        proof = json.loads((ROOT / "docs/evidence/row-margin-before.json").read_text(encoding="utf-8"))
        path = ROOT / proof["before_trace"]
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), proof["before_trace_sha256"])
        trace = json.loads(path.read_text(encoding="utf-8"))
        verify(trace)
        with self.assertRaises(AssertionError):
            verify(trace, nondefault_margins=True)

    def test_linefeed_scrolls_only_the_active_region(self):
        cells, _ = replay({"width": 4, "height": 4, "scroll_region": [1, 2],
            "sequences": ["\x1b[HAAAA\x1b[2HBBBB\x1b[3HCCCC\x1b[4HDDDD\x1b[3H\r\n"]})
        self.assertEqual(["".join(c[0] for c in row) for row in cells],
                         ["AAAA", "CCCC", "    ", "DDDD"])

    def test_relative_vertical_moves_clamp_but_absolute_addresses_cross_margins(self):
        cells, _ = replay({"width": 4, "height": 4, "scroll_region": [1, 2],
            "sequences": ["\x1b[3;2H\x1b[2AX\x1b[1;3HY\x1b[2;4H\x1b[2BZ"]})
        self.assertEqual(["".join(c[0] for c in row) for row in cells],
                         ["  Y ", " X  ", "   Z", "    "])


if __name__ == "__main__":
    unittest.main()
