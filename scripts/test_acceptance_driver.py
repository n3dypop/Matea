"""Acceptance launch/timeout/source-drift failures cannot retain old success."""
import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import verify_foundation as verifier
from measure_rust_coverage import run_coverage, collect


class AcceptanceDriverTests(unittest.TestCase):
    def setUp(self):
        self.temporary=tempfile.TemporaryDirectory(prefix="matea-acceptance-fixture-")
        self.addCleanup(self.temporary.cleanup);self.root=Path(self.temporary.name).resolve()
        subprocess.run(["git","init","--quiet"],cwd=self.root,check=True,capture_output=True)
        (self.root/"unit.rs").write_text("// initial\n",encoding="utf-8")
        self.evidence=self.root/"docs/evidence";self.evidence.mkdir(parents=True)
        (self.evidence/"local-validation.json").write_text('{"all_local_checks_passed":true}',encoding="utf-8")

    def run_checks(self,commands,**options):
        with contextlib.redirect_stdout(io.StringIO()):return verifier.run_checks(self.root,commands,**options)

    def test_real_success_failure_and_missing_launch_all_run_and_clear_old_success(self):
        commands=[("success",[sys.executable,"-c","print('complete output')"]),
                  ("failure",[sys.executable,"-c","raise SystemExit(7)"]),
                  ("missing",[str(self.root/"no-such-executable")])]
        result=self.run_checks(commands)
        self.assertEqual([c["exit_code"] for c in result["checks"]],[0,7,127])
        self.assertFalse(result["all_local_checks_passed"])
        self.assertEqual(result["status"],"failed")
        self.assertIn("complete output",(self.evidence/"success.log").read_text())
        self.assertEqual(json.loads((self.evidence/"local-validation.json").read_text()),result)

    def test_timeout_keeps_partial_output_continues_and_journals_failure(self):
        real_run=verifier.subprocess.run
        def run(command,**kwargs):
            if command[0]=="timeout-fixture":raise subprocess.TimeoutExpired(command,1,output=b"partial output",stderr=b"partial error")
            return real_run(command,**kwargs)
        with patch.object(verifier.subprocess,"run",side_effect=run):
            result=self.run_checks([("timeout",["timeout-fixture"]),("after",[sys.executable,"-c","pass"])])
        self.assertEqual([c["exit_code"] for c in result["checks"]],[124,0])
        log=(self.evidence/"timeout.log").read_text();self.assertIn("partial output",log);self.assertIn("partial error",log)
        self.assertFalse(result["all_local_checks_passed"])

    def test_empty_duplicate_or_non_git_configuration_cannot_keep_old_success(self):
        for commands in [[],[("same",["unused"]),("same",["unused"])],[("../escape",["unused"])],[("empty",[])]]:
            result=self.run_checks(commands)
            self.assertFalse(result["all_local_checks_passed"])
            self.assertEqual(result["checks"],[])
        with patch.object(verifier,"execution_inputs",side_effect=ValueError("missing Git")):
            result=self.run_checks([("unused",["unused"])])
        self.assertEqual(result["status"],"failed")
        self.assertIn("missing Git",result["configuration_error"])

    def test_source_changes_and_tool_probe_failures_prevent_success(self):
        command=[sys.executable,"-c","from pathlib import Path; Path('unit.rs').write_text('changed')"]
        result=self.run_checks([("change",command)])
        self.assertEqual(result["checks"][0]["exit_code"],0)
        self.assertEqual(result["source_changed_during_run"],["unit.rs"])
        self.assertFalse(result["all_local_checks_passed"])
        real_run=verifier.subprocess.run
        def run(command,**kwargs):
            if command==["rustc","--version"]:raise FileNotFoundError("missing rustc")
            return real_run(command,**kwargs)
        with patch.object(verifier.subprocess,"run",side_effect=run):result=self.run_checks([("success",[sys.executable,"-c","pass"])])
        self.assertEqual(result["rustc"]["exit_code"],127)
        self.assertFalse(result["all_local_checks_passed"])

    def test_complete_success_has_bound_sources_and_running_journal_is_never_success(self):
        command=[sys.executable,"-c","import json; from pathlib import Path; r=json.loads(Path('docs/evidence/local-validation.json').read_text()); assert r['status']=='running' and not r['all_local_checks_passed']"]
        result=self.run_checks([("journal",command)])
        self.assertTrue(result["all_local_checks_passed"])
        self.assertEqual(result["status"],"complete")
        self.assertEqual(set(result["source_sha256"]),{"unit.rs"})

    def test_failed_coverage_launch_timeout_export_and_source_drift_clear_old_success(self):
        path=self.evidence/"quality/rust-execution-coverage.json";path.parent.mkdir()
        for failure in [FileNotFoundError("missing LLVM"),subprocess.TimeoutExpired("cargo",1),
                        json.JSONDecodeError("bad LLVM export","!",0),ValueError("empty profiles")]:
            path.write_text('{"status":"complete"}',encoding="utf-8")
            def collector(root):
                self.assertEqual(json.loads(path.read_text())["status"],"running")
                raise failure
            with self.subTest(failure=type(failure).__name__):
                result=run_coverage(self.root,collector)
                self.assertEqual(result["status"],"failed")
                self.assertIn("error",result)
                self.assertEqual(json.loads(path.read_text()),result)
        def changed(root):
            (root/"unit.rs").write_text("changed",encoding="utf-8")
            return {"schema":2,"files":[{"file":"unit.rs"}],"source_sha256":{"unit.rs":"old"}}
        self.assertEqual(run_coverage(self.root,changed)["status"],"failed")
        # Exercise the real collector's process launch boundary before any Rust
        # build, rather than treating wrapper-only exceptions as execution.
        with patch("measure_rust_coverage.subprocess.run",side_effect=FileNotFoundError("missing cargo")):
            self.assertEqual(run_coverage(self.root,collect)["status"],"failed")


if __name__=="__main__":unittest.main()
