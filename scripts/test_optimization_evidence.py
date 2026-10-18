"""Adversarial records must not earn a measurement verdict."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from optimization_evidence import compare, records
from report_attribute_performance import legacy_comparison, normalize_legacy_theme


class OptimizationEvidenceTests(unittest.TestCase):
    def test_theme_schema_change_requires_all_seven_exact_legacy_styles(self):
        old = "Style { fg: Default, bg: Indexed(9), bold: true, underline: true, reverse: true }"
        new = "Style { fg: Default, bg: Indexed(9), underline_color: Default, attributes: TextAttributes { flags: {Bold, Reverse}, underline: Single, blink: None, frame: None } }"
        text = "Theme { text: " + ", ".join([old]*7) + " }"
        expected = "Theme { text: " + ", ".join([new]*7) + " }"
        self.assertEqual(normalize_legacy_theme(text), expected)
        for bad in [text.replace(old, new, 1), text.replace("bold: true", "bold: unknown", 1), text+old]:
            with self.assertRaises(ValueError): normalize_legacy_theme(bad)

    def test_theme_normalization_never_hides_new_attributes_or_other_output_changes(self):
        old = "Style { fg: Default, bg: Default, bold: false, underline: false, reverse: false }"
        before = self.fixture(); before.update(case="style/theme_plain", output="Theme { text: "+", ".join([old]*7)+" }")
        after = copy.deepcopy(before); after["output"] = normalize_legacy_theme(before["output"])
        row = legacy_comparison({before["case"]:before}, {after["case"]:after})[0]
        self.assertEqual(row["output_comparison"], "explicit_legacy_theme_defaults")
        self.assertEqual(before["output"].count("bold: false"), 7)
        for replacement in ["blink: Rapid", "underline_color: Indexed(9)", "frame: Framed"]:
            changed=copy.deepcopy(after)
            field=replacement.split(":")[0]
            changed["output"]=changed["output"].replace(field+": "+("Default" if field=="underline_color" else "None"), replacement, 1)
            with self.assertRaises(ValueError): legacy_comparison({before["case"]:before}, {changed["case"]:changed})
        before["case"]=after["case"]="other"
        with self.assertRaises(ValueError): legacy_comparison({"other":before},{"other":after})

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
