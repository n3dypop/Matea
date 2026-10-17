"""Ensure quality gates cannot hide missing programs or poor dimensions."""
import copy
from pathlib import Path
import subprocess
import tempfile
import unittest
from check_quality import DIMENSIONS, assess, programs
from measure_rust_coverage import canonical_files


class QualityGateTests(unittest.TestCase):
    def test_alias_coverage_preserves_every_compiler_instance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/terminal/src/modes.rs"
            source.parent.mkdir(parents=True); source.write_text("// fixture")
            alias = root / "crates/terminal/benches/../src/modes.rs"
            (root / "crates/terminal/benches").mkdir()
            value = {"data": [{"files": [{"filename": str(source), "summary": {"lines": {"percent": 0}}},
                                        {"filename": str(alias), "summary": {"lines": {"percent": 100}}},
                                        {"filename": str(root.parent / "external.rs"), "summary": {}}]}]}
            files = canonical_files(value, root)
            self.assertEqual(len(files), 1)
            self.assertEqual(files[0]["file"], "crates/terminal/src/modes.rs")
            self.assertEqual([i["summary"]["lines"]["percent"] for i in files[0]["instances"]], [0, 100])
            self.assertIn("/../", files[0]["instances"][1]["compiler_filename"])

    def test_noninteger_scores_cannot_enter_the_gate(self):
        for score in [False, 75.0, "75", -1, 76]:
            plan = self.fixture(); plan["units"][0]["criteria"]["behavior"]["score"] = score
            with self.subTest(score=score), self.assertRaises(ValueError):
                assess(plan, {"a.rs"})

    def test_source_inventory_covers_new_root_and_benchmark_programs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet", directory], check=True, capture_output=True)
            (root / ".gitignore").write_text("/target/\n/artifacts/\n", encoding="utf-8")
            wanted = {"build.rs", "benches/probe.rs", "crates/example/src/lib.rs", "tools/new/check.py"}
            for name in wanted | {"target/generated.rs", "artifacts/cached.py"}:
                path = root / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text("", encoding="utf-8")
            self.assertEqual(programs(root), wanted)

    def fixture(self):
        return {"units": [{"id": "unit", "files": ["a.rs"], "evidence": ["proof.json"], "remaining": ["finite scope"],
                           "criteria": {d: {"score": 75, "reason": "bounded independent evidence"} for d in DIMENSIONS}}]}

    def test_low_dimension_is_not_hidden_by_other_high_scores(self):
        plan = self.fixture()
        self.assertTrue(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["memory_safety"]["score"] = 25
        result = assess(plan, {"a.rs"})
        self.assertFalse(result["all_high"])
        self.assertEqual(len(result["below_high"]), 1)

    def test_new_files_and_missing_dimensions_are_rejected(self):
        plan = self.fixture()
        with self.assertRaises(ValueError): assess(plan, {"a.rs", "new.rs"})
        with self.assertRaises(ValueError): assess(plan, set())
        broken = copy.deepcopy(plan)
        del broken["units"][0]["criteria"]["performance"]
        with self.assertRaises(ValueError): assess(broken, {"a.rs"})

    def test_not_applicable_requires_an_explanation_and_is_distinct_from_unmeasured(self):
        plan = self.fixture()
        plan["units"][0]["criteria"]["reference"] = {"score": None, "reason": "original audit utility; no reference counterpart"}
        self.assertTrue(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["performance"] = {"score": 0, "reason": "unmeasured"}
        self.assertFalse(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["reference"]["reason"] = ""
        with self.assertRaises(ValueError): assess(plan, {"a.rs"})


if __name__ == "__main__":
    unittest.main()
