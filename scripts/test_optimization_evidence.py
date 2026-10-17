"""Adversarial records must not earn a measurement verdict."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from optimization_evidence import compare, records


class OptimizationEvidenceTests(unittest.TestCase):
    def test_current_cli_checks_expected_count_and_optimized_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "probe.jsonl"
            path.write_text(json.dumps(self.fixture()), encoding="utf-8")
            tool = str(Path(__file__).with_name("optimization_evidence.py"))
            for flags in [[], ["-O"]]:
                for expected, success in [(1, True), (2, False)]:
                    result = subprocess.run([sys.executable, *flags, tool, str(path),
                                             "--validate-current", "--expected", str(expected)],
                                            capture_output=True)
                    self.assertEqual(result.returncode == 0, success)

    def fixture(self):
        return {"case": "fixture", "semantics": "one operation", "output": "value", "iterations_per_sample": 10,
                "allocations": 0, "allocated_bytes": 0, "first_return_value": 1, "samples_ns": list(range(1, 32))}

    def test_duplicate_cases_are_not_silently_collapsed(self):
        text = json.dumps(self.fixture())
        with self.assertRaises(ValueError): records(text + "\n" + text)

    def test_nonfinite_invalid_counters_and_incomplete_samples_are_rejected(self):
        for value in [float("nan"), float("inf"), -1, 0, True]:
            item = self.fixture(); item["samples_ns"][0] = value
            with self.assertRaises(ValueError): records(json.dumps(item))
        for field, value in [("iterations_per_sample", 0), ("allocations", -1), ("allocated_bytes", True)]:
            item = self.fixture(); item[field] = value
            with self.assertRaises(ValueError): records(json.dumps(item))
        item = self.fixture(); item["samples_ns"].pop()
        with self.assertRaises(ValueError): records(json.dumps(item))

    def test_mismatched_output_semantics_and_counts_never_compare(self):
        before = records(json.dumps(self.fixture()))
        for field, value in [("output", "other"), ("semantics", "less work"), ("iterations_per_sample", 11), ("first_return_value", 2)]:
            after = copy.deepcopy(before); after["fixture"][field] = value
            with self.assertRaises(ValueError): compare(before, after, 1)
        with self.assertRaises(ValueError): compare(before, before, 2)

    def test_adverse_percentiles_and_allocations_remain_in_the_report(self):
        before = records(json.dumps(self.fixture())); after = copy.deepcopy(before)
        after["fixture"]["samples_ns"] = [100] * 31
        after["fixture"]["allocations"] = 7
        row = compare(before, after, 1)[0]
        self.assertEqual(row["after"]["p95_ns"], 100)
        self.assertEqual(row["after"]["allocations"], 7)


if __name__ == "__main__":
    unittest.main()
