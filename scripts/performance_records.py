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
    # Python integers are finite without a lossy float conversion (which can
    # overflow for a malformed/large input before validation can reject it).
    finite = type(value) is int or (type(value) is float and math.isfinite(value))
    return finite and (value > 0 if positive else value >= 0)


def validate(record):
    if not isinstance(record, dict) or not isinstance(record.get("case"), str) or not record["case"]:
        raise ValueError("missing case identifier")
    if "record_kind" in record:
        raise ValueError("typed correctness records are not measurements")
    if not isinstance(record.get("semantics"), str) or not record["semantics"] or not isinstance(record.get("output"), str):
        raise ValueError("missing semantics/output")
    samples = record.get("samples_ns")
    if not isinstance(samples, list) or len(samples) < 31:
        raise ValueError("at least 31 positive finite timing samples required")
    # Validate the hot array in one loop, without a generator and a Python
    # function call for each sample. The float bounds also reject NaN/inf;
    # integers stay exact, including values too large to convert to float.
    for sample in samples:
        if type(sample) is float:
            valid = 0 < sample < math.inf
        else:
            valid = type(sample) is int and sample > 0
        if not valid:
            raise ValueError("at least 31 positive finite timing samples required")
    iterations = record.get("iterations_per_sample")
    if type(iterations) is not int or iterations <= 0:
        raise ValueError("invalid iterations_per_sample")
    if "calibration" in record:
        calibration = record["calibration"]
        if not isinstance(calibration, dict) or set(calibration) != {"target_ns", "start_n", "max_n", "batches"}:
            raise ValueError("invalid calibration schema")
        target, starting, maximum = (calibration[key] for key in ["target_ns", "start_n", "max_n"])
        if any(type(n) is not int or n <= 0 for n in [target, starting, maximum]) or starting > maximum:
            raise ValueError("invalid calibration bounds")
        batches = calibration["batches"]
        if not isinstance(batches, list) or not batches:
            raise ValueError("missing calibration batches")
        expected = starting
        confirmed = isinstance(batches[0],dict) and "confirmations_ns" in batches[0]
        for index, batch in enumerate(batches):
            fields = {"iterations", "elapsed_ns"} | ({"confirmations_ns"} if confirmed else set())
            if not isinstance(batch, dict) or set(batch) != fields:
                raise ValueError("invalid calibration batch")
            count, elapsed = batch["iterations"], batch["elapsed_ns"]
            confirmations = batch.get("confirmations_ns", [])
            if not isinstance(confirmations, list) or (confirmed and len(confirmations) != 2):
                raise ValueError("two calibration confirmations required")
            observations = [elapsed, *confirmations]
            if type(count) is not int or count != expected or any(type(n) is not int or n < 0 for n in observations):
                raise ValueError("invalid calibration observation")
            ready = min(observations) >= target
            if index == len(batches)-1:
                if count != iterations or not ready:
                    raise ValueError("calibration did not reach the recorded sample count/target")
            elif ready or count == maximum:
                raise ValueError("calibration continued after completion/limit")
            expected = min(count*2, maximum)
    for key in ["allocations", "allocated_bytes"]:
        if not numeric(record.get(key)):
            raise ValueError("invalid counter: " + key)
    if "first_return_value" in record and not numeric(record["first_return_value"]):
        raise ValueError("invalid first_return_value")
    if "cold_ns" in record and not numeric(record["cold_ns"]):
        raise ValueError("invalid cold_ns")
    return record


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON field: " + key)
        result[key] = value
    return result


def invalid_constant(value):
    raise ValueError("nonfinite JSON constant: " + value)


def finite_float(text):
    value = float(text)
    if not math.isfinite(value):
        raise ValueError("nonfinite JSON number: " + text)
    return value


_STRICT_DECODER = json.JSONDecoder(object_pairs_hook=unique_object, parse_constant=invalid_constant, parse_float=finite_float)


def parse_json(text):
    # Reuse immutable decoder configuration across records. Keep json.loads'
    # bytes/encoding behavior for callers that supply a raw context file.
    if isinstance(text, (bytes, bytearray)):
        text = text.decode(json.detect_encoding(text), "surrogatepass")
    return _STRICT_DECODER.decode(text)


def read_records(text):
    records = {}
    for line in text.splitlines():
        item = validate(parse_json(line))
        if item["case"] in records:
            raise ValueError("duplicate performance case: " + item["case"])
        records[item["case"]] = item
    if not records:
        raise ValueError("empty performance records")
    return records


def read_probe_outputs(text):
    """Correctness records are intentionally not accepted as measurements."""
    records = {}
    for line in text.splitlines():
        item = parse_json(line)
        if not isinstance(item, dict) or set(item) != {"record_kind", "case", "semantics", "first_return_value", "output"}:
            raise ValueError("invalid output-only record schema")
        if item["record_kind"] != "probe-output" or not isinstance(item["case"], str) or not item["case"]:
            raise ValueError("invalid output-only identifier")
        if not isinstance(item["semantics"], str) or not item["semantics"] or not isinstance(item["output"], str):
            raise ValueError("invalid output-only semantics/output")
        if type(item["first_return_value"]) is not int or not 0 <= item["first_return_value"] <= (1 << 64)-1:
            raise ValueError("invalid output-only return value")
        if item["case"] in records:
            raise ValueError("duplicate output-only case")
        records[item["case"]] = item
    if not records:
        raise ValueError("empty output-only records")
    return records


def summary(record):
    validate(record)
    samples = sorted(record["samples_ns"])
    return {"p50_ns": statistics.median(samples), "p95_ns": samples[math.ceil(len(samples) * .95) - 1],
            "allocations": record["allocations"], "allocated_bytes": record["allocated_bytes"]}
