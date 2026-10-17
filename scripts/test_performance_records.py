"""Bad measurement input must not collapse into a positive verdict."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from performance_records import read_records, read_probe_outputs, summary, write_report
from report_performance import generate
from report_viewport_performance import apply_harness_patch


class RecordTests(unittest.TestCase):
    def test_historical_patch_reconstruction_preserves_bytes_with_windows_git_defaults(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);source=root/"source.txt";source.write_bytes(b"before\n")
            diff=root/"harness.patch"
            diff.write_bytes(b"diff --git a/source.txt b/source.txt\n--- a/source.txt\n+++ b/source.txt\n@@ -1 +1 @@\n-before\n+after\n")
            environment={"GIT_CONFIG_COUNT":"2","GIT_CONFIG_KEY_0":"core.autocrlf","GIT_CONFIG_VALUE_0":"true",
                         "GIT_CONFIG_KEY_1":"core.eol","GIT_CONFIG_VALUE_1":"crlf"}
            with patch.dict(os.environ,environment):apply_harness_patch(root,diff)
            self.assertEqual(source.read_bytes(),b"after\n")

    def test_correctness_and_measurement_records_cannot_be_confused(self):
        record={"record_kind":"probe-output", "case":"one", "semantics":"exact output", "first_return_value":1,"output":"value"}
        text=json.dumps(record)
        self.assertEqual(read_probe_outputs(text)["one"],record)
        with self.assertRaises(ValueError):read_records(text)
        with self.assertRaises(ValueError):read_records(json.dumps({**self.fixture(),**record}))
        with self.assertRaises(ValueError):read_probe_outputs(json.dumps(self.fixture()))
        for bad in ["",text+"\n"+text,text.replace('"case": "one"','"case": "two", "case": "one"')]:
            with self.assertRaises(ValueError):read_probe_outputs(bad)
        for key,value in [("first_return_value",True),("first_return_value",-1),("first_return_value",1<<64),
                          ("record_kind","measurement"),("output",None),("semantics","")]:
            bad={**record,key:value}
            with self.subTest(key=key,value=value),self.assertRaises(ValueError):read_probe_outputs(json.dumps(bad))

    def test_check_mode_never_rewrites_or_creates_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)/"report.json"
            with patch("performance_records.sys.argv", ["reporter", "--check"]):
                write_report(path, "new")
                self.assertFalse(path.exists())
                path.write_bytes(b"reviewed")
                write_report(path, "new")
                self.assertEqual(path.read_bytes(), b"reviewed")

    def fixture(self):
        return {"case": "one", "semantics": "one operation", "output": "value", "iterations_per_sample": 10,
                "samples_ns": list(range(1, 32)), "allocations": .25, "allocated_bytes": 8.5}

    def test_historical_missing_return_and_averaged_go_counters_stay_explicit(self):
        record = self.fixture()
        parsed = read_records(json.dumps(record))
        self.assertNotIn("first_return_value", parsed["one"])
        self.assertEqual(summary(record), {"p50_ns": 16, "p95_ns": 30, "allocations": .25, "allocated_bytes": 8.5})

    def test_duplicates_empty_and_bad_numbers_are_rejected(self):
        text = json.dumps(self.fixture())
        for bad in ["", text+"\n"+text, text.replace('"case": "one"','"case": "two", "case": "one"')]:
            with self.assertRaises(ValueError): read_records(bad)
        for key in ["samples_ns", "allocations", "allocated_bytes", "iterations_per_sample"]:
            for number in [float("inf"), float("nan"), True, -1]:
                value = self.fixture()
                value[key] = [number] * 31 if key == "samples_ns" else number
                with self.subTest(key=key, number=number), self.assertRaises(ValueError):
                    read_records(json.dumps(value))

    def test_rejected_report_does_not_overwrite_published_raw_samples(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); source=root/"source"; target=root/"destination"
            source.mkdir(); target.mkdir()
            (target/"matea-current.jsonl").write_text("old reviewed evidence",encoding="utf-8")
            (source/"matea-current.jsonl").write_text("invalid",encoding="utf-8")
            before={p.name:p.read_bytes() for p in target.iterdir()}
            with self.assertRaises((ValueError,FileNotFoundError)):
                generate(source,destination=target)
            self.assertEqual(before,{p.name:p.read_bytes() for p in target.iterdir()})


if __name__ == "__main__":
    unittest.main()
