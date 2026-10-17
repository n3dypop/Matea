"""Validate retained cross-language samples without hiding duplicate cases.

Historical probes may omit first_return_value and Go reports averaged allocation
counts/bytes. Those fields are not fabricated or rounded by this reader.
"""
import json
import math
import os
import statistics
import sys


def probe_path(root):
    return root / "target/release" / ("matea-perf-probe.exe" if os.name == "nt" else "matea-perf-probe")


def write_report(path, text):
    """--check executes historical validation without replacing any evidence."""
    if "--check" not in sys.argv:
        path.write_text(text, encoding="utf-8")


def numeric(value, *, positive=False):
    return type(value) in (int, float) and math.isfinite(value) and (value > 0 if positive else value >= 0)


def validate(record):
    if not isinstance(record, dict) or not isinstance(record.get("case"), str) or not record["case"]:
        raise ValueError("missing case identifier")
    if not isinstance(record.get("semantics"), str) or not record["semantics"] or not isinstance(record.get("output"), str):
        raise ValueError("missing semantics/output")
    samples = record.get("samples_ns")
    if not isinstance(samples, list) or len(samples) < 31 or not all(numeric(n, positive=True) for n in samples):
        raise ValueError("at least 31 positive finite timing samples required")
    iterations = record.get("iterations_per_sample")
    if type(iterations) is not int or iterations <= 0:
        raise ValueError("invalid iterations_per_sample")
    for key in ["allocations", "allocated_bytes"]:
        if not numeric(record.get(key)):
            raise ValueError("invalid counter: " + key)
    if "first_return_value" in record and not numeric(record["first_return_value"]):
        raise ValueError("invalid first_return_value")
    return record


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON field: " + key)
        result[key] = value
    return result


def read_records(text):
    records = {}
    for line in text.splitlines():
        item = validate(json.loads(line, object_pairs_hook=unique_object))
        if item["case"] in records:
            raise ValueError("duplicate performance case: " + item["case"])
        records[item["case"]] = item
    if not records:
        raise ValueError("empty performance records")
    return records


def summary(record):
    validate(record)
    samples = sorted(record["samples_ns"])
    return {"p50_ns": statistics.median(samples), "p95_ns": samples[math.ceil(len(samples) * .95) - 1],
            "allocations": record["allocations"], "allocated_bytes": record["allocated_bytes"]}
