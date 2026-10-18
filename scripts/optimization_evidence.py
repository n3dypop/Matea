"""Validate module measurement records without relaxing the quality gate."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
from performance_records import parse_json, validate

ROOT = Path(__file__).resolve().parents[1]


def records(text):
    result = {}
    for line in text.splitlines():
        item = validate(parse_json(line))
        name = item["case"]
        if name in result:
            raise ValueError("empty/duplicate case identifier")
        for field in ["allocations", "allocated_bytes", "first_return_value"]:
            value = item.get(field)
            if type(value) is not int or value < 0:
                raise ValueError("invalid counter: " + field)
        samples = item["samples_ns"]
        if len(samples) != 31:
            raise ValueError("31 positive finite timing samples required")
        result[name] = item
    if not result:
        raise ValueError("empty measurement file")
    return result


def summary(item):
    values = sorted(item["samples_ns"])
    return {"p50_ns": statistics.median(values), "p95_ns": values[29],
            "allocations": item["allocations"], "allocated_bytes": item["allocated_bytes"]}


def compare(before, after, expected):
    if len(before) != expected or before.keys() != after.keys():
        raise ValueError("measurement set is incomplete or changed")
    rows = []
    for name, item in before.items():
        changed = after[name]
        for key in ["semantics", "output", "iterations_per_sample", "first_return_value"]:
            if item[key] != changed[key]:
                raise ValueError("incomparable " + key + ": " + name)
        rows.append({"case": name, "semantics": item["semantics"], "output": item["output"],
                     "before": summary(item), "after": summary(changed)})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--expected", type=int, required=True)
    parser.add_argument("--validate-current", action="store_true",
                        help="validate one probe JSONL file without making a historical performance comparison")
    parser.add_argument("--target-ns", type=int,
                        help="require confirmed calibration at this target for every current record")
    args = parser.parse_args()
    if args.target_ns is not None and (not args.validate_current or args.target_ns <= 0):
        parser.error("--target-ns requires --validate-current and a positive target")
    if args.validate_current:
        data = records(args.directory.read_text(encoding="utf-8"))
        if len(data) != args.expected:
            raise ValueError("current probe case count changed")
        if args.target_ns is not None:
            for item in data.values():
                calibration=item.get("calibration",{})
                if calibration.get("target_ns")!=args.target_ns or not all(
                        len(batch.get("confirmations_ns",[]))==2 for batch in calibration.get("batches",[])):
                    raise ValueError("required confirmed calibration missing/changed: "+item["case"])
        print(f"{len(data)} valid current cases; timing is not cross-platform performance evidence")
        return
    contexts, data = [], []
    for stage in ["before", "after"]:
        context = parse_json((args.directory / (stage + "-context.json")).read_text(encoding="utf-8"))
        raw = (args.directory / (stage + ".jsonl")).read_bytes()
        if hashlib.sha256(raw).hexdigest() != context["raw_sha256"]:
            raise ValueError("raw hash mismatch: " + stage)
        revision = context["git_head"]
        for path, expected in context["source_sha256"].items():
            blob = subprocess.check_output(["git", "show", revision + ":" + path], cwd=ROOT)
            if hashlib.sha256(blob).hexdigest() != expected:
                raise ValueError("source hash mismatch: " + path)
        contexts.append(context)
        data.append(records(raw.decode("utf-8")))
    rows = compare(*data, args.expected)
    result = {"schema": 1, "stage": "individual_modules", "case_count": len(rows),
              "contexts": dict(zip(["before", "after"], contexts)), "cases": rows,
              "limits": ["Rust-only matched before/after fixtures; no upstream speed ratio or full-framework-stage claim.",
                         "One macOS arm64 host, sequential release probes, 31 samples. No affinity/frequency control; retain p95 and unchanged-path regressions.",
                         "Requested System allocation counters are separate from timing and are not resident-memory measurements.",
                         "Successful measurements do not automatically raise a quality score or establish universal optimality."],
              "reporter_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (args.directory / "report.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{len(rows)} matching cases; both raw/source snapshots verified")


if __name__ == "__main__":
    main()
