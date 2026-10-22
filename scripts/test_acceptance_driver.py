"""Acceptance launch/timeout/source-drift failures cannot retain old success."""
import contextlib
import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import test_windows_console as console_oracle
import test_terminal_device as device_oracle
import verify_foundation as verifier
from measure_rust_coverage import run_coverage, collect


class AcceptanceDriverTests(unittest.TestCase):
    def test_device_records_and_input_failures_cannot_pass_as_success(self):
        self.assertEqual(device_oracle.parse_line(b"raw\t132\t42\n"), ["raw", "132", "42"])
        for raw in [b"raw\t132\t42", b"raw\t0\t42\n", b"raw\t65536\t42\n", b"raw\t1\ttrue\n",
                    b"result\tok\te88\tnone\tfalse\n", b"result\tok\tXX\tnone\tfalse\n",
                    b"restored\textra\n", b"unknown\n"]:
            with self.subTest(raw=raw), self.assertRaises((AssertionError, ValueError)):
                device_oracle.parse_line(raw)
        fields = ["result", "limit", "7465", "2", "false"]
        device_oracle.check_password(fields, "limit", b"te", 2)
        for index, wrong in [(1, "ok"), (2, "746561"), (3, "none"), (4, "true")]:
            bad = fields.copy(); bad[index] = wrong
            with self.subTest(index=index), self.assertRaises(AssertionError):
                device_oracle.check_password(bad, "limit", b"te", 2)

    def test_device_failed_native_observation_clears_success_and_retains_attempts(self):
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory); probe=folder/"probe"; probe.write_bytes(b"fixture")
            journal=folder/"context.json"; journal.write_text('{"status":"complete"}')
            with patch.object(device_oracle.sys, "platform", "linux"), patch.object(device_oracle, "unix_run", side_effect=ValueError("native timeout")):
                self.assertFalse(device_oracle.run(probe,folder))
            first=json.loads(journal.read_bytes())
            self.assertEqual(first["status"], "failed")
            self.assertIn("native timeout", first["error"])
            self.assertEqual(json.loads((folder/first["attempt"]/"context.json").read_bytes()),first)
            with patch.object(device_oracle.sys,"platform","win32"), patch.object(device_oracle.subprocess,"CREATE_NEW_CONSOLE",16,create=True), patch.object(device_oracle.subprocess,"run",return_value=subprocess.CompletedProcess([],0,b"",b"")):
                self.assertFalse(device_oracle.run(probe,folder))
            second=json.loads(journal.read_bytes())
            self.assertEqual(second["status"],"failed")
            self.assertNotEqual(first["attempt"],second["attempt"])
            self.assertEqual(json.loads((folder/first["attempt"]/"context.json").read_bytes()),first)

    def console_trace(self):
        return {"schema":1,"kind":"native-windows-console-input","read_width":60,"frames":[
            {"case":f"{glyph}/{width}/{fraction}","width":width,"ansi":"\x1b[H","expected":"#"*width}
            for glyph in ["ascii","block","half"] for width in [40,8,60,17] for fraction in [0,625,1000]]}

    def test_console_trace_rejects_missing_duplicate_geometry_and_non_bmp_cells(self):
        trace=self.console_trace();console_oracle.validate_trace(trace)
        for mode in ["empty","duplicate","case","width","supplementary","control","stream"]:
            bad=copy.deepcopy(trace)
            if mode=="empty":bad["frames"]=[]
            elif mode=="duplicate":bad["frames"][1]=bad["frames"][0]
            elif mode=="case":bad["frames"][0]["case"]="bad"
            elif mode=="width":bad["frames"][0]["width"]=True
            elif mode=="supplementary":bad["frames"][0]["expected"]="👩"+"#"*39
            elif mode=="control":bad["frames"][0]["expected"]="\x1b"+"#"*39
            else:bad["frames"][0]["ansi"]="x"*16385
            with self.subTest(mode=mode),self.assertRaises(ValueError):console_oracle.validate_trace(bad)

    def test_console_result_requires_exact_native_rows_and_restoration(self):
        trace=self.console_trace()
        result={"schema":1,"status":"complete","restored":True,"window_width":80,
                "mode_before":1,"mode_after":1,"code_page_before":65001,"code_page_after":65001,
                "cases":[{"case":f["case"],"width":f["width"],"exact_cells":60,"observed":f["expected"].ljust(60)} for f in trace["frames"]]}
        console_oracle.validate_result(result,trace)
        for field,value in [("status","failed"),("restored",False),("mode_after",True),("code_page_after",0),("window_width",59),("cases",[])]:
            with self.subTest(field=field),self.assertRaises(ValueError):console_oracle.validate_result({**result,field:value},trace)
        bad=copy.deepcopy(result);bad['cases'][0]['observed']=bad['cases'][0]['observed'][:-1]+"\x00"
        with self.assertRaises(ValueError):console_oracle.validate_result(bad,trace)

    def test_console_wrong_host_or_missing_new_observation_cannot_reuse_success(self):
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory);probe=folder/"probe";probe.write_bytes(b"fixture binary")
            journal=folder/"context.json";journal.write_bytes(b'{"status":"complete"}')
            with patch.object(console_oracle,"sys",SimpleNamespace(platform="linux")):
                self.assertFalse(console_oracle.run(probe,folder))
            self.assertEqual(json.loads(journal.read_bytes())['status'],'failed')
            previous=folder/"observed.json";previous.write_bytes(b'{"status":"complete"}')
            def process(command,**kwargs):
                output=json.dumps(self.console_trace()).encode() if command[0]==str(probe) else b""
                return subprocess.CompletedProcess(command,0,output,b"")
            with patch.object(console_oracle,"sys",SimpleNamespace(platform="win32")),patch.object(console_oracle.shutil,"which",return_value="pwsh"),patch.object(console_oracle.subprocess,"CREATE_NEW_CONSOLE",16,create=True),patch.object(console_oracle.subprocess,"run",side_effect=process):
                self.assertFalse(console_oracle.run(probe,folder))
            self.assertEqual(json.loads(journal.read_bytes())['status'],'failed')
            self.assertEqual(previous.read_bytes(),b'{"status":"complete"}')

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
