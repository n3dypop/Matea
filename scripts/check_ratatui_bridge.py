"""Fail-closed managed-widget boundary observations, not full public-API parity."""
import argparse
import os
from pathlib import Path
import subprocess
from performance_records import read_probe_outputs, probe_path
from fetch_upstream_issues import atomic_json

ROOT = Path(__file__).resolve().parents[1]
CASES = {f"ratatui-bridge/{widget}_{kind}_{w}x{h}": (widget, kind, w, h)
         for widget in ["paragraph", "list", "table"] for kind in ["ascii", "unicode"]
         for w, h in [(24, 6), (80, 24)]}


def decode(output, width, height):
    if not isinstance(output, str) or len(output) > 1_000_000 or len(output) % 2 or any(c not in "0123456789abcdef" for c in output):
        raise ValueError("invalid bounded canonical hex")
    raw = bytes.fromhex(output)
    offset = 0

    def take(n):
        nonlocal offset
        if n < 0 or offset+n > len(raw):
            raise ValueError("truncated projection")
        value = raw[offset:offset+n]
        offset += n
        return value

    if int.from_bytes(take(2), "little") != width or int.from_bytes(take(2), "little") != height:
        raise ValueError("wrong dimensions")
    rows = []
    for _ in range(height):
        row, covered, x = [], 0, 0
        while x < width:
            span = int.from_bytes(take(2), "little")
            if covered:
                if span != 0:
                    raise ValueError("missing continuation")
                covered -= 1
                x += 1
                continue
            if span == 0 or span > width-x:
                raise ValueError("invalid lead width")
            symbol = take(int.from_bytes(take(4), "little")).decode("utf-8")
            if not symbol or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in symbol):
                raise ValueError("invalid cell symbol")
            colors = [tuple(take(4)) for _ in range(3)]
            for c in colors:
                if c[0] not in (0, 1, 2) or (c[0] == 0 and c != (0, 0, 0, 0)) or (c[0] == 1 and c[2:] != (0, 0)):
                    raise ValueError("invalid color")
            flags = int.from_bytes(take(2), "little")
            if flags & ~511:
                raise ValueError("unknown modifier")
            row.append((x, span, symbol, colors, flags))
            covered = span-1
            x += 1
        rows.append(row)
    if offset != len(raw):
        raise ValueError("trailing projection data")
    digest = 0xcbf29ce484222325
    for byte in raw:
        digest = ((digest ^ byte)*0x100000001b3) & ((1 << 64)-1)
    return rows, digest


def validate(records):
    if set(records) != set(CASES):
        raise ValueError("incomplete or unexpected managed-widget cases")
    for case, (widget, kind, w, h) in CASES.items():
        record = records[case]
        rows, digest = decode(record["output"], w, h)
        if record["first_return_value"] != digest:
            raise ValueError("independent FNV mismatch: "+case)
        plain = ["".join(cell[2] for cell in row) for row in rows]
        if not plain[0].startswith("┌Matea") or not plain[-1].startswith("└"):
            raise ValueError("literal block border/title mismatch: "+case)
        expected = "matcha tea" if kind == "ascii" else "抹茶 e\u0301 👩‍💻 tea"
        if not any(expected in line for line in plain):
            raise ValueError("literal body mismatch: "+case)
        if widget != "paragraph" and not any("#50" in line for line in plain):
            raise ValueError("stateful selected row missing: "+case)
        # Explicitly styled cells must retain the requested RGB, indexed and
        # named underline colors. The modifier bit values are source public API.
        styled = [cell for row in rows for cell in row if cell[4] != 0]
        if not styled or any(cell[3] != [(2,40,180,80),(1,17,0,0),(1,14,0,0)] or cell[4] != 77 for cell in styled):
            raise ValueError("literal complete selected style mismatch: "+case)
    return len(records)


def check(probe, destination):
    destination.mkdir(parents=True, exist_ok=True)
    journal = destination/"outputs-check.json"
    atomic_json(journal,{"schema":1,"status":"running"})
    try:
        observed = []
        for mode in ["source", "matea"]:
            result = subprocess.run([str(probe), "--ratatui-bridge-"+mode], cwd=ROOT,
                                    env=dict(os.environ, MATEA_PROBE_OUTPUT_ONLY="1"),
                                    capture_output=True, check=True, timeout=60)
            (destination/(mode+"-outputs.jsonl")).write_bytes(result.stdout)
            records = read_probe_outputs(result.stdout.decode("utf-8"))
            validate(records)
            observed.append(records)
        if observed[0] != observed[1]:
            raise ValueError("source/managed full projection mismatch")
        report = {"schema":1,"status":"complete","cases":len(CASES),
                  "limits":"Pinned managed widgets only; no full public API/raw diff/width policy/physical terminal parity."}
        atomic_json(destination/"outputs-check.json",report)
        print(f"PASS {len(CASES)} independently decoded, literal-checked managed widget pairs")
    except Exception as error:
        atomic_json(journal,{"schema":1,"status":"failed","error":str(error)})
        raise


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe",type=Path,default=probe_path(ROOT))
    parser.add_argument("--destination",type=Path,default=ROOT/"docs/evidence/ratatui-bridge/outputs")
    args = parser.parse_args()
    check(args.probe.resolve(),args.destination)
