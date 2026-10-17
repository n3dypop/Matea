"""Independent corrupted-evidence fixtures for the public-capability ledger."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from check_coverage import issue_needs_review

ROOT = Path(__file__).resolve().parents[1]


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="matea-coverage-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in ["check_coverage.py", "evidence_checks.py"]:
            path = self.root/"scripts"/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((ROOT/"scripts"/name).read_bytes())
        self.write("docs/upstream-lock.json", [{"name":"fixture", "sha":"a"*40}])
        self.inventory = {"name":"fixture", "sha":"a"*40, "language":"go", "macro_cfg_review":"build_tags_recorded",
                          "declarations":[{"id":"fixture/one", "file":"unit/source.go"}],
                          "modules":[{"path":"unit"}], "issue_markers":[]}
        self.capability = {"id":"one", "status":"implemented", "reference_ids":["fixture/one"],
                           "implementation":["src/lib.rs"], "tests":["tests/check.rs"], "performance":"docs/proof.md"}
        self.verdict = {"reference":"fixture", "source_group":"unit", "status":"implemented_measured",
                        "declarations_reviewed":1, "correctness":["tests/check.rs"], "source_audit":"docs/proof.md",
                        "performance_report":"docs/report.json", "performance_cases":["one"],
                        "performance_verdict":"bounded independent fixture", "notes":"No full parity claim"}
        self.report = {"cases":[{"case":"one", "output_equivalent_for_this_fixture":True}]}
        for name in ["src/lib.rs", "tests/check.rs", "docs/proof.md"]:
            path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text("fixture\n",encoding="utf-8")
        self.write("docs/coverage/upstream-issues.json", {"repositories":[]})
        self.write("docs/coverage/dependency-frontier.json", {"scope_closure_reviewed":False})
        self.save()

    def write(self, name, value):
        path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text(json.dumps(value),encoding="utf-8")

    def save(self):
        self.write("docs/coverage/fixture.json", self.inventory)
        self.write("docs/coverage/capabilities.json", {"capabilities":[self.capability]})
        self.write("docs/coverage/module-verdicts.json", {"module_groups":[self.verdict]})
        self.write("docs/report.json", self.report)

    def run_checker(self, *args):
        return subprocess.run([sys.executable, str(self.root/"scripts/check_coverage.py"), *args],
                              capture_output=True,text=True,encoding="utf-8",timeout=30)

    def assert_rejected(self):
        result=self.run_checker()
        self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)

    def test_valid_partial_ledger_is_read_only_and_cannot_open_f5(self):
        before={p.relative_to(self.root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()
                for p in (self.root/"docs").rglob("*") if p.is_file()}
        self.assertEqual(self.run_checker().returncode,0)
        gate=self.run_checker("--gate")
        self.assertEqual(gate.returncode,1)
        self.assertIn("F5 CLOSED",gate.stdout)
        after={p.relative_to(self.root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()
               for p in (self.root/"docs").rglob("*") if p.is_file()}
        self.assertEqual(before,after)

    def test_duplicate_performance_case_cannot_overwrite_a_failed_result(self):
        self.report["cases"].insert(0,{"case":"one", "output_equivalent_for_this_fixture":False})
        self.save();self.assert_rejected()

    def test_equivalence_must_be_true_boolean(self):
        for value in ["false",1,False,None]:
            with self.subTest(value=value):
                self.report["cases"][0]["output_equivalent_for_this_fixture"]=value
                self.save();self.assert_rejected()

    def test_implemented_capability_requires_implementation_test_and_performance_evidence(self):
        for name in ["implementation","tests","performance"]:
            with self.subTest(name=name):
                original=self.capability.pop(name)
                try:self.save();self.assert_rejected()
                finally:self.capability[name]=original

    def test_unknown_performance_evidence_is_not_treated_as_measured(self):
        self.capability["performance"]="missing-report.md"
        self.save();self.assert_rejected()

    def test_paths_cannot_escape_the_repository_even_when_the_file_exists(self):
        outside=self.root.parent/(self.root.name+"-outside.rs")
        outside.write_text("not project evidence",encoding="utf-8")
        self.addCleanup(outside.unlink)
        self.capability["implementation"]=["../"+outside.name]
        self.save();self.assert_rejected()

    def test_duplicate_json_fields_are_rejected(self):
        (self.root/"docs/report.json").write_text('{"cases":[],"cases":[{"case":"one","output_equivalent_for_this_fixture":true}]}',encoding="utf-8")
        self.assert_rejected()

    def test_stale_inventory_pin_is_rejected(self):
        self.inventory["sha"]="b"*40
        self.save();self.assert_rejected()

    def test_closure_requires_a_boolean(self):
        self.write("docs/coverage/dependency-frontier.json", {"scope_closure_reviewed":"false"})
        self.assert_rejected()

    def test_compiler_identity_exit_codes_and_keys_cannot_be_forged(self):
        identity={"reference":"fixture", "sha":"a"*40, "package":"fixture", "profile":"default", "target":"fixture-target"}
        for defect in ["none", "wrong-package", "boolean-exit", "duplicate-key", "duplicate-profile"]:
            with self.subTest(defect=defect):
                run={**identity,"extractor_sha256":"1"*64,"exit_code":0,"rustdoc_exit_code":0,"dependency_tree_exit_code":0}
                graph={**identity,"extractor_sha256":"1"*64,"items":[{"profile_item_key":"one"}],"trait_implementations":[]}
                if defect=="wrong-package":run["package"]="other"
                if defect=="boolean-exit":run["exit_code"]=False
                if defect=="duplicate-key":graph["items"]*=2
                self.write("docs/run.json",run);self.write("docs/graph.json",graph)
                snapshot={**identity,"run":"docs/run.json","graph":"docs/graph.json",
                          "graph_sha256":hashlib.sha256((self.root/"docs/graph.json").read_bytes()).hexdigest()}
                self.write("docs/coverage/rustdoc/index.json",{"target_feature_closure_reviewed":False,
                    "signature_dependency_closure_reviewed":False,"snapshots":[snapshot]*(2 if defect=="duplicate-profile" else 1)})
                if defect=="none":self.assertEqual(self.run_checker().returncode,0)
                else:self.assert_rejected()

    def test_issue_review_stales_on_upstream_source_or_evidence_change(self):
        pin={"name":"fixture","sha":"a"*40,"remote":"https://github.com/owner/fixture.git"}
        proof="docs/proof.md"
        review={"status":"complete","notes":"bounded review", "reference":"fixture", "reference_sha":pin["sha"],
                "reviewed_updated_at":"snapshot", "evidence_sha256":{proof:hashlib.sha256((self.root/proof).read_bytes()).hexdigest()}}
        issue={"disposition":"assessed", "updated_at":"snapshot", "review":review}
        self.assertFalse(issue_needs_review(self.root,issue,"owner/fixture",{"fixture":pin}))
        for key,value in [("status","partial"),("reference_sha","b"*40),("reviewed_updated_at","old")]:
            original=review[key];review[key]=value
            self.assertTrue(issue_needs_review(self.root,issue,"owner/fixture",{"fixture":pin}))
            review[key]=original
        (self.root/proof).write_text("changed evidence",encoding="utf-8")
        self.assertTrue(issue_needs_review(self.root,issue,"owner/fixture",{"fixture":pin}))
        with self.assertRaises(AssertionError):issue_needs_review(self.root,issue,"other/repo",{"fixture":pin})

    def test_unsubstantiated_triage_cannot_hide_an_issue(self):
        issue={"disposition":"untriaged_report_not_confirmed_defect"}
        self.assertTrue(issue_needs_review(self.root,issue,"owner/fixture",{}))
        issue["disposition"]="fixed"
        with self.assertRaises(KeyError):issue_needs_review(self.root,issue,"owner/fixture",{})


if __name__=="__main__":
    unittest.main()
