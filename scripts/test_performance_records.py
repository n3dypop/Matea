"""Bad measurement input must not collapse into a positive verdict."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from performance_records import read_records, summary
from report_performance import generate


class RecordTests(unittest.TestCase):
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
