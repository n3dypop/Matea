"""Independent ASCII cursor/erase replay for the two performance probes.

Reject unknown control sequences. This limited oracle does not validate Unicode,
styles, terminal timing, or actual emulator behavior.
"""
from evidence_checks import require
import argparse
import json
import re
from pathlib import Path

CSI = re.compile(r"\x1b\[(\??[0-9]{0,10}(?:;[0-9]{0,10}){0,31})([ -/]*)([@-~])")


def replay(trace, include_cursor=False):
    width, height = trace["width"], trace["height"]
    require(type(width) is int and type(height) is int
            and 1 <= width <= 4096 and 1 <= height <= 4096
            and width * height <= 1_048_576, "unsupported replay geometry")
    streams = trace["sequences"]
    require(isinstance(streams, list) and len(streams) <= 4096
            and all(isinstance(s, str) for s in streams)
            and sum(map(len, streams)) <= 16_777_216, "unsupported replay streams")
    cells = [[" "] * width for _ in range(height)]
    x = y = 0
    saved = (0, 0)
    snapshots = []
    visible = False
    for stream in streams:
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
                require(match, lambda: repr(stream[offset:]))
                body, intermediate, final = match.groups()
                require(not intermediate, lambda: repr(match.group()))
                private = body.startswith("?")
                values = [int(p or 0) for p in (body[1:] if private else body).split(";")]
                n = values[0] or 1
                if not private and final in "HfABCDGJKX":
                    require(len(values) <= (2 if final in "Hf" else 1), "unexpected CSI parameters")
                if private:
                    require(final in "hl" and all(v in (25, 1049, 7, 2026) for v in values), lambda: repr(match.group()))
                    if 1049 in values and final == "h":
                        cells = [[" "] * width for _ in range(height)]
                        x = y = 0
                    if 25 in values:
                        visible = final == "h"
                elif final in "Hf":
                    y = min(height - 1, (values[0] or 1) - 1)
                    x = min(width - 1, (values[1] or 1) - 1) if len(values) > 1 else 0
                elif final in "ABCD":
                    if final == "A": y = max(0, y - n)
                    if final == "B": y = min(height - 1, y + n)
                    if final == "C": x = min(width - 1, x + n)
                    if final == "D": x = max(0, x - n)
                elif final == "G":
                    x = min(width - 1, n - 1)
                elif final == "m":
                    pass  # This workload uses default styles only.
                elif final in "JK":
                    mode = values[0]
                    require(mode in (0, 1, 2))
                    column = min(x, width - 1)
                    first = y if final == "K" or mode == 0 else 0
                    last = y + 1 if final == "K" or mode == 1 else height
                    for row in range(first, last):
                        left = column if mode == 0 and row == y else 0
                        right = column + 1 if mode == 1 and row == y else width
                        cells[row][left:right] = [" "] * (right - left)
                elif final == "X":
                    column = min(x, width - 1)
                    right = min(width, column + n)
                    cells[y][column:right] = [" "] * (right - column)
                else:
                    raise AssertionError(repr(match.group()))
                offset = match.end()
                continue
            if char == "\r": x = 0
            elif char == "\n": y = min(height - 1, y + 1)
            elif char == "\b": x = max(0, x - 1)
            else:
                require(" " <= char <= "~", lambda: repr(char))
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
            require(replay(trace, include_cursor=True) == cursor_expected, lambda: path)
        else:
            require(replay(trace) == expected, lambda: path)
        print(f"{path.name}: exact states; frame bytes {[len(s.encode()) for s in trace['sequences']]}")


if __name__ == "__main__":
    main()
