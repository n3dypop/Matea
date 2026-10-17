"""Ensure quality gates cannot hide missing programs or poor dimensions."""
import copy
import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest
from check_quality import DIMENSIONS, assess, programs, validate_native, validate_coverage
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

    def test_native_success_requires_current_sources_logs_and_completed_integer_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);subprocess.run(["git","init","--quiet"],cwd=root,check=True,capture_output=True)
            (root/"unit.rs").write_bytes(b"source")
            evidence=root/"docs/evidence";evidence.mkdir(parents=True)
            (evidence/"check.log").write_bytes(b"output")
            report={"status":"complete","all_local_checks_passed":True,"rustc":{"exit_code":0},"cargo":{"exit_code":0},
                "source_changed_during_run":[],"source_sha256":{"unit.rs":hashlib.sha256(b"source").hexdigest()},
                "checks":[{"name":"check","exit_code":0,"log":"check.log","log_sha256":hashlib.sha256(b"output").hexdigest()}]}
            validate_native(report,root)
            for defect in ["running","truthy","empty","duplicate","boolean-exit","tool","changed","source","missing-source","log","escape"]:
                bad=copy.deepcopy(report)
                if defect=="running":bad["status"]="running"
                if defect=="truthy":bad["all_local_checks_passed"]=1
                if defect=="empty":bad["checks"]=[]
                if defect=="duplicate":bad["checks"]*=2
                if defect=="boolean-exit":bad["checks"][0]["exit_code"]=False
                if defect=="tool":bad["rustc"]["exit_code"]=127
                if defect=="changed":bad["source_changed_during_run"]=["unit.rs"]
                if defect=="source":bad["source_sha256"]["unit.rs"]="a"*64
                if defect=="missing-source":bad["source_sha256"]={}
                if defect=="log":bad["checks"][0]["log_sha256"]="a"*64
                if defect=="escape":bad["checks"][0]["log"]="../check.log"
                with self.subTest(defect=defect),self.assertRaises(ValueError):validate_native(bad,root)

    def test_source_inventory_covers_new_root_and_benchmark_programs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "--quiet", directory], check=True, capture_output=True)
            (root / ".gitignore").write_text("/target/\n/artifacts/\n", encoding="utf-8")
            wanted = {"build.rs", "benches/probe.rs", "crates/example/src/lib.rs", "tools/new/check.py"}
            for name in wanted | {"target/generated.rs", "artifacts/cached.py"}:
                path = root / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text("", encoding="utf-8")
            self.assertEqual(programs(root), wanted)

    def test_coverage_publication_rejects_stale_failed_or_missing_source_instances(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);subprocess.run(["git","init","--quiet"],cwd=root,check=True,capture_output=True)
            (root/"unit.rs").write_bytes(b"source")
            hashes={"unit.rs":hashlib.sha256(b"source").hexdigest()}
            report={"status":"complete","input_sha256":hashes,"source_sha256":dict(hashes),
                    "files":[{"file":"unit.rs","instances":[{"summary":{}}]}]}
            self.assertEqual(set(validate_coverage(report,root)),{"unit.rs"})
            for defect in ["running","source","input","missing","duplicate","instances"]:
                bad=copy.deepcopy(report)
                if defect=="running":bad["status"]="running"
                if defect=="source":bad["source_sha256"]["unit.rs"]="a"*64
                if defect=="input":bad["input_sha256"]={}
                if defect=="missing":bad["source_sha256"]={}
                if defect=="duplicate":bad["files"]*=2
                if defect=="instances":bad["files"][0]["instances"]=[]
                with self.subTest(defect=defect),self.assertRaises(ValueError):validate_coverage(bad,root)

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
