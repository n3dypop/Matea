"""Independent ASCII cell/SGR replay for the finite colored-frame fixtures."""
import argparse
import json
import os
import re
import subprocess
from pathlib import Path

CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


def replay(trace):
    width, height = trace["width"], trace["height"]
    cells = [[(" ", None, None, 0) for _ in range(width)] for _ in range(height)]
    x = y = attrs = 0
    fg = bg = None
    visible, last = True, " "
    def put(char):
        nonlocal x, y, last
        if x == width:
            x, y = 0, min(height - 1, y + 1)
        cells[y][x] = (char, fg, bg, attrs)
        x += 1
        last = char
    for stream in trace["sequences"]:
        offset = 0
        while offset < len(stream):
            char = stream[offset]
            if char != "\x1b":
                if char == "\r": x = 0
                elif char == "\n":
                    assert y + 1 < height, "fixture must not scroll below the viewport"
                    y += 1
                elif char == "\b": x = max(0, x - 1)
                else:
                    assert " " <= char <= "~", repr(char)
                    put(char)
                offset += 1
                continue
            match = CSI.match(stream, offset)
            assert match, repr(stream[offset:offset + 40])
            body, intermediate, final = match.groups()
            assert not intermediate, repr(match.group())
            values = [int(n or 0) for n in body.lstrip("?").split(";")]
            n = values[0] or 1
            if body.startswith("?"):
                assert final in "hl" and all(v in (25, 1049, 7, 2026) for v in values)
                if 25 in values: visible = final == "h"
                if 1049 in values and final == "h":
                    cells = [[(" ", None, None, 0) for _ in range(width)] for _ in range(height)]
                    x = y = 0
            elif final in "Hf":
                y = (values[0] or 1) - 1
                x = (values[1] or 1) - 1 if len(values) > 1 else 0
            elif final in "ABCD":
                if final == "A": y = max(0, y - n)
                if final == "B": y = min(height - 1, y + n)
                if final == "C": x = min(width - 1, x + n)
                if final == "D": x = max(0, x - n)
            elif final == "G": x = n - 1
            elif final == "b":
                for _ in range(n): put(last)
            elif final == "m":
                i = 0
                while i < len(values):
                    code = values[i]
                    if code == 0: fg = bg = None; attrs = 0
                    elif code == 39: fg = None
                    elif code == 49: bg = None
                    elif 30 <= code <= 37: fg = code - 30
                    elif 90 <= code <= 97: fg = code - 90 + 8
                    elif 40 <= code <= 47: bg = code - 40
                    elif 100 <= code <= 107: bg = code - 100 + 8
                    elif code in (38, 48):
                        if values[i + 1] == 5:
                            color = values[i + 2]
                            i += 2
                        else:
                            assert values[i + 1] == 2, repr(match.group())
                            color = ("rgb", *values[i + 2:i + 5])
                            i += 4
                        if code == 38: fg = color
                        else: bg = color
                    elif code in (1, 2, 3, 4, 5, 7, 8, 9): attrs |= 1 << code
                    elif code in (22, 23, 24, 25, 27, 28, 29):
                        for flag in ({22: [1, 2]}.get(code, [code - 20])): attrs &= ~(1 << flag)
                    else: raise AssertionError(repr(match.group()))
                    i += 1
            elif final in "JK":
                mode = values[0]
                assert mode in (0, 1, 2)
                for row in range(height):
                    for col in range(width):
                        position = row * width + col if final == "J" else col
                        cursor = y * width + min(x, width - 1) if final == "J" else min(x, width - 1)
                        if (final == "J" or row == y) and (mode == 2 or mode == 0 and position >= cursor or mode == 1 and position <= cursor):
                            cells[row][col] = (" ", fg, bg, attrs)
            elif final == "X":
                for col in range(x, min(width, x + n)): cells[y][col] = (" ", fg, bg, attrs)
            else:
                raise AssertionError(repr(match.group()))
            offset = match.end()
    return cells, visible


def verify(document):
    assert {t["profile"] for t in document["traces"]} == {"ansi16", "ansi256", "truecolor", "none"}
    for trace in document["traces"]:
        assert (trace["width"], trace["height"]) == (80, 24)
        foreground, background = {"ansi16": (15, 0), "ansi256": (67, 132),
                                  "truecolor": (("rgb", 95, 135, 175), ("rgb", 175, 95, 135)),
                                  "none": (None, None)}[trace["profile"]]
        for end in range(1, len(trace["sequences"]) + 1):
            cells, visible = replay({**trace, "sequences": trace["sequences"][:end]})
            assert not visible
            for y, row in enumerate(cells):
                for x, cell in enumerate(row):
                    if y < 20:
                        assert cell == ("a", foreground, background, 0), (trace["profile"], x, y, cell)
                    else:
                        # Foreground on blanks is invisible; background/attrs
                        # must still be at the default erasure state.
                        assert cell[0] == " " and cell[2] is None and cell[3] == 0, (x, y, cell)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("traces", type=Path, nargs="*")
    parser.add_argument("--probe", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    assert args.traces or args.probe, "provide trace paths or a built probe"
    if args.probe:
        # A fresh process observes NO_COLOR before Crossterm initializes its
        # global policy. Explicit capabilities must still encode every profile.
        env = dict(os.environ, NO_COLOR="1")
        output = subprocess.check_output([str(args.probe.resolve()), "--color-trace"], env=env)
        document = json.loads(output)
        verify(document)
        if args.output:
            args.output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
        print("NO_COLOR=1: explicit four-profile capabilities preserved in a fresh process")
    for path in args.traces:
        document = json.loads(path.read_text(encoding="utf-8"))
        verify(document)
        print(f"{path.name}: exact glyph/palette/background/hidden-cursor fixtures")


if __name__ == "__main__":
    main()
