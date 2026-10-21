"""Regression-test the replay oracle against the previously missed margin case."""
import hashlib
import json
from pathlib import Path
import unittest
from check_color_trace import replay, verify, verify_fallback
from check_terminal_trace import replay as ascii_replay

ROOT = Path(__file__).resolve().parents[1]


class MarginReplayTests(unittest.TestCase):
    def test_erase_matches_an_independent_flat_cell_model_at_every_position(self):
        # Flattened ranges are deliberately separate from the row implementation.
        for width, height in [(1, 1), (4, 3), (9, 5)]:
            initial = [chr(65 + i % 26) for i in range(width * height)]
            setup = "".join(f"\x1b[{y + 1};1H" + "".join(initial[y * width:(y + 1) * width])
                            for y in range(height))
            for y in range(height):
                for x in range(width + 1):
                    # x == width represents the delayed-wrap cursor state.
                    cursor = f"\x1b[{y + 1};{min(x, width - 1) + 1}H"
                    if x == width:
                        cursor += initial[(y + 1) * width - 1]
                    for command in "JKX":
                        for mode in ([0, 1, 2] if command != "X" else [0, 1, 2, 99]):
                            expected = initial.copy()
                            column = min(x, width - 1)
                            for i in range(len(expected)):
                                row, col = divmod(i, width)
                                if command == "J":
                                    erase = mode == 2 or (i >= y * width + column if mode == 0 else i <= y * width + column)
                                elif command == "K":
                                    erase = row == y and (mode == 2 or (col >= column if mode == 0 else col <= column))
                                else:
                                    erase = row == y and column <= col < column + (mode or 1)
                                if erase:
                                    expected[i] = " "
                            trace = {"width": width, "height": height,
                                     "sequences": [setup + cursor + f"\x1b[{mode}{command}"]}
                            screen = "\n".join("".join(expected[r * width:(r + 1) * width]).rstrip() for r in range(height))
                            with self.subTest(width=width, height=height, x=x, y=y, command=command, mode=mode):
                                self.assertEqual(ascii_replay(trace), [screen])
                                cells, _ = replay(trace)
                                self.assertEqual([c[0] for row in cells for c in row], expected)

    def test_color_erase_uses_current_rendition_including_delayed_wrap(self):
        for command in ["0K", "1K", "2K", "0J", "1J", "2J", "2X"]:
            trace = {"width": 4, "height": 2,
                     "sequences": ["abcd\x1b[1;48;2;9;8;7m\x1b[" + command]}
            cells, _ = replay(trace)
            erased = [c for row in cells for c in row if c[0] == " " and c[2] is not None]
            self.assertTrue(erased, command)
            self.assertTrue(all(c == (" ", None, ("rgb", 9, 8, 7), 2) for c in erased))

    def test_absolute_cursor_coordinates_clamp_to_screen(self):
        for address in ["999;999H", "999;999f", "2;999H", "999G"]:
            trace = {"width": 4, "height": 2,
                     "sequences": ["\x1b[" + address + "X"]}
            with self.subTest(address=address):
                self.assertEqual(ascii_replay(trace)[0], ("\n   X" if address != "999G" else "   X\n"))
                cells, _ = replay(trace)
                self.assertEqual(sum(c[0] == "X" for row in cells for c in row), 1)

    def test_incomplete_out_of_range_and_noncanonical_sgr_are_rejected(self):
        for sgr in ["38", "38;5", "48;2", "48;2;1;2", "38;5;256", "48;2;256;0;0",
                    "38;2;0;0;999", "38;3;1", "?38;2;1;2;3", "38:2:1:2:3"]:
            with self.subTest(sgr=sgr), self.assertRaises(AssertionError):
                replay({"width": 4, "height": 2, "sequences": ["\x1b[" + sgr + "m"]})

    def test_invalid_geometry_and_unbounded_commands_fail_before_allocation_or_replay(self):
        for function in [ascii_replay, replay]:
            for width, height in [(0, 2), (4, 0), (-1, 2), (True, 2), (1.5, 2), (4097, 2), (4096, 4096)]:
                with self.subTest(function=function.__module__, width=width, height=height), self.assertRaises(AssertionError):
                    function({"width": width, "height": height, "sequences": []})
            for sequence in ["\x1b[1;2;3H", "\x1b[>1C", "\x1b[1;2X", "\x1b[99999999999C"]:
                with self.subTest(function=function.__module__, sequence=sequence), self.assertRaises(AssertionError):
                    function({"width": 4, "height": 2, "sequences": [sequence]})
        with self.assertRaises(AssertionError):
            replay({"width": 4, "height": 2, "sequences": ["\x1b[999999999b"]})

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
