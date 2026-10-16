"""Independent ASCII cursor/erase replay for the two performance probes.

Reject unknown control sequences. This limited oracle does not validate Unicode,
styles, terminal timing, or actual emulator behavior.
"""
import argparse
import json
import re
from pathlib import Path

CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


def replay(trace, include_cursor=False):
    width, height = trace["width"], trace["height"]
    cells = [[" "] * width for _ in range(height)]
    x = y = 0
    saved = (0, 0)
    snapshots = []
    visible = False
    for stream in trace["sequences"]:
        offset = 0
        while offset < len(stream):
            char = stream[offset]
            if char == "\x1b":
                if stream[offset:offset + 2] in ("\x1b7", "\x1b8"):
                    if stream[offset + 1] == "7":
                        saved = (x, y)
                    else:
                        x, y = saved
                    offset += 2
                    continue
                match = CSI.match(stream, offset)
                assert match, repr(stream[offset:])
                body, intermediate, final = match.groups()
                assert not intermediate, repr(match.group())
                private = body.startswith("?")
                values = [int(n or 0) for n in body.lstrip("?").split(";")]
                n = values[0] or 1
                if private:
                    assert final in "hl" and all(v in (25, 1049, 7, 2026) for v in values), repr(match.group())
                    if 1049 in values and final == "h":
                        cells = [[" "] * width for _ in range(height)]
                        x = y = 0
                    if 25 in values:
                        visible = final == "h"
                elif final in "Hf":
                    y = (values[0] or 1) - 1
                    x = (values[1] or 1) - 1 if len(values) > 1 else 0
                elif final in "ABCD":
                    if final == "A": y = max(0, y - n)
                    if final == "B": y = min(height - 1, y + n)
                    if final == "C": x = min(width - 1, x + n)
                    if final == "D": x = max(0, x - n)
                elif final == "G":
                    x = n - 1
                elif final == "m":
                    pass  # This workload uses default styles only.
                elif final in "JK":
                    mode = values[0]
                    assert mode in (0, 1, 2)
                    for row in range(height):
                        for col in range(width):
                            position = row * width + col if final == "J" else col
                            cursor = y * width + min(x, width - 1) if final == "J" else min(x, width - 1)
                            if (final == "J" or row == y) and (mode == 2 or mode == 0 and position >= cursor or mode == 1 and position <= cursor):
                                cells[row][col] = " "
                elif final == "X":
                    for col in range(x, min(width, x + n)): cells[y][col] = " "
                else:
                    raise AssertionError(repr(match.group()))
                offset = match.end()
                continue
            if char == "\r": x = 0
            elif char == "\n": y = min(height - 1, y + 1)
            elif char == "\b": x = max(0, x - 1)
            else:
                assert " " <= char <= "~", repr(char)
                if x == width:
                    x = 0
                    y = min(height - 1, y + 1)
                cells[y][x] = char
                x += 1
            offset += 1
        screen = "\n".join("".join(row).rstrip() for row in cells)
        snapshots.append({"screen": screen, "cursor": [x, y], "visible": visible} if include_cursor else screen)
    return snapshots


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("traces", type=Path, nargs="+")
    args = parser.parse_args()
    expected = [c + "a" * 79 + "\n" * 23 for c in "aba"]
    for path in args.traces:
        trace = json.loads(path.read_text())
        if "cursor-trace" in path.name:
            cursor_expected = [{"screen": "a" * 80 + "\n" * 23, "cursor": [x, 3], "visible": show}
                               for x, show in [(2, True), (4, True), (2, True), (2, False)]]
            assert replay(trace, include_cursor=True) == cursor_expected, path
        else:
            assert replay(trace) == expected, path
        print(f"{path.name}: exact states; frame bytes {[len(s.encode()) for s in trace['sequences']]}")


if __name__ == "__main__":
    main()
