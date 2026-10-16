"""Regression-test the replay oracle against the previously missed margin case."""
import hashlib
import json
from pathlib import Path
import unittest
from check_color_trace import replay, verify

ROOT = Path(__file__).resolve().parents[1]


class MarginReplayTests(unittest.TestCase):
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
