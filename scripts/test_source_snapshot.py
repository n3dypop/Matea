"""Fault and provenance fixtures for compiler input snapshots; no compiler invoked."""
import contextlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import collect_rustdoc as collector


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="matea-snapshot-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.reference = self.root/"references/crossterm"
        self.reference.mkdir(parents=True)
        (self.reference/"src").mkdir(); (self.reference/"assets").mkdir()
        (self.reference/"src/lib.rs").write_text("pub struct Tea;\n",encoding="utf-8")
        (self.reference/"Cargo.toml").write_text('[package]\nname="crossterm"\nversion="0.1.0"\nedition="2021"\n',encoding="utf-8")
        self.asset = Path("assets/抹茶.json")
        (self.reference/self.asset).write_text('{"api":1}\n',encoding="utf-8")
        self.git("init","--quiet")
        self.git("add","--all")
        self.pin={"name":"crossterm","sha":self.commit_fixture()}
        self.snapshot=self.root/"audit/source/crossterm"
        self.fake_root=self.root/"Matea"
        (self.fake_root/"docs").mkdir(parents=True)
        (self.fake_root/"docs/upstream-lock.json").write_text(json.dumps([self.pin]),encoding="utf-8")

    def git(self,*args):
        return subprocess.check_output(["git",*args],cwd=self.reference,text=True,stderr=subprocess.PIPE)

    def commit_fixture(self):
        self.git("-c","user.name=Snapshot Fixture","-c","user.email=fixture@example.invalid",
                 "-c","commit.gpgsign=false","-c","core.hooksPath="+str(self.reference/".git/unused-hooks"),
                 "commit","--quiet","-m","fixture")
        return self.git("rev-parse","HEAD").strip()

    @contextlib.contextmanager
    def mocked_compiler_cli(self, cargo_run=None):
        original_command,original_run=collector.command,subprocess.run
        calls=[]
        def command(*args,**kwargs):
            if args[0]=="rustc":return "rustc fixture\nhost: fixture-target"
            if args[0]=="cargo":return "cargo fixture"
            return original_command(*args,**kwargs)
        def run(arguments,*args,**kwargs):
            if arguments[0]=="cargo":
                calls.append(list(arguments))
                if cargo_run is not None:return cargo_run(arguments)
                return subprocess.CompletedProcess(arguments,1,"","fixture: compiler intentionally not executed\n")
            return original_run(arguments,*args,**kwargs)
        arguments=["collect_rustdoc.py","--references",str(self.reference.parent),"--destination",str(self.root/"audit"),
                   "--reference","crossterm","--package","crossterm","--toolchain","fixture"]
        with patch.object(collector,"ROOT",self.fake_root),patch.object(collector,"command",side_effect=command),\
             patch("sys.argv",arguments),patch.object(collector.subprocess,"run",side_effect=run):
            yield calls

    def raw_document(self):
        return {"format_version":61,"root":0,"paths":{},"external_crates":{},"index":{
            "0":{"crate_id":0,"name":"fixture","visibility":"public","span":None,"attrs":[],
                 "inner":{"module":{"items":[],"is_crate":True,"is_stripped":False}}}}}

    def raw_path(self):
        return self.root/"audit/target/crossterm/default/fixture-target/doc/crossterm.json"

    def test_missing_malformed_or_unreadable_exports_never_keep_success_exit_code(self):
        for mode in ["missing","malformed","unknown-format","recursive"]:
            with self.subTest(mode=mode):
                def cargo(arguments):
                    path=self.raw_path();path.parent.mkdir(parents=True,exist_ok=True)
                    path.unlink(missing_ok=True)
                    if mode!="missing":
                        document=self.raw_document()
                        if mode=="unknown-format":document["format_version"]=999
                        path.write_text("{" if mode=="malformed" else json.dumps(document),encoding="utf-8")
                    return subprocess.CompletedProcess(arguments,0,"","")
                exporter=patch.object(collector,"exports",side_effect=RecursionError("fixture nesting")) if mode=="recursive" else contextlib.nullcontext()
                with self.mocked_compiler_cli(cargo) as calls,exporter:
                    self.assertEqual(collector.main(),1)
                self.assertEqual(len(calls),1)
                report=json.loads(next((self.root/"audit/reports").glob("*-run.json")).read_text())
                self.assertEqual(report["rustdoc_exit_code"],0)
                self.assertEqual(report["exit_code"],1)
                self.assertTrue(report["export_error"])
                self.assertNotEqual(report["status"],"compiled_profile_only_not_full_scope")

    def test_dependency_tree_error_or_timeout_keeps_graph_scope_unresolved(self):
        for timeout in [False,True]:
            with self.subTest(timeout=timeout):
                def cargo(arguments):
                    if arguments[2]=="tree":
                        if timeout:raise subprocess.TimeoutExpired(arguments,300,output=b"tree partial")
                        return subprocess.CompletedProcess(arguments,7,"","fixture tree failure")
                    path=self.raw_path();path.parent.mkdir(parents=True,exist_ok=True)
                    path.write_text(json.dumps(self.raw_document()),encoding="utf-8")
                    return subprocess.CompletedProcess(arguments,0,"","")
                with self.mocked_compiler_cli(cargo):
                    self.assertEqual(collector.main(),124 if timeout else 7)
                report=json.loads(next((self.root/"audit/reports").glob("*-run.json")).read_text())
                self.assertEqual(report["status"],"compiled_dependency_graph_unresolved")
                self.assertEqual(report["dependency_tree_exit_code"],124 if timeout else 7)

    def test_rustdoc_timeout_preserves_partial_log_and_nonzero_status(self):
        def cargo(arguments):
            raise subprocess.TimeoutExpired(arguments,300,output=b"partial stdout\xff",stderr=b"partial stderr")
        with self.mocked_compiler_cli(cargo):
            self.assertEqual(collector.main(),124)
        report=json.loads(next((self.root/"audit/reports").glob("*-run.json")).read_text())
        self.assertEqual(report["exit_code"],124)
        self.assertTrue(report["rustdoc_timed_out"])
        self.assertIn("partial stdout",next((self.root/"audit/reports").glob("*.log")).read_text())

    def test_compiler_launch_error_is_recorded_without_stale_success(self):
        def cargo(arguments):
            raise FileNotFoundError("fixture cargo disappeared")
        with self.mocked_compiler_cli(cargo):
            self.assertEqual(collector.main(),127)
        report=json.loads(next((self.root/"audit/reports").glob("*-run.json")).read_text())
        self.assertEqual(report["rustdoc_exit_code"],127)
        self.assertEqual(report["status"],"compile_failed_profile_unresolved")
        self.assertIn("FileNotFoundError",next((self.root/"audit/reports").glob("*.log")).read_text())

    def test_modified_non_rust_asset_is_rejected_before_compiler_invocation(self):
        with self.mocked_compiler_cli() as calls:
            self.assertEqual(collector.main(),1)
            self.assertEqual(len(calls),1)
            (self.snapshot/self.asset).write_text('{"api":2}\n',encoding="utf-8")
            with self.assertRaisesRegex((AssertionError,ValueError),"snapshot source changed"):
                collector.main()
            self.assertEqual(len(calls),1)
        self.assertEqual((self.reference/self.asset).read_text(encoding="utf-8"),'{"api":1}\n')

    def test_every_tracked_input_is_copied_hashed_and_reused_without_mutation(self):
        marker=collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        provenance=json.loads(marker.read_text(encoding="utf-8"))
        self.assertEqual(provenance["schema"],2)
        self.assertIn(self.asset.as_posix(),provenance["source_hashes"])
        self.assertEqual((self.snapshot/self.asset).read_bytes(),(self.reference/self.asset).read_bytes())
        before={p.relative_to(self.snapshot).as_posix():p.read_bytes() for p in self.snapshot.rglob("*") if p.is_file()}
        self.assertEqual(marker,collector.prepare_snapshot(self.reference,self.snapshot,self.pin))
        self.assertEqual(before,{p.relative_to(self.snapshot).as_posix():p.read_bytes() for p in self.snapshot.rglob("*") if p.is_file()})
        self.assertEqual(self.git("status","--porcelain"),"")

    def test_copy_failure_leaves_no_partial_snapshot_and_retry_succeeds(self):
        copy=collector.shutil.copy2
        count=0
        def fail_later(*args,**kwargs):
            nonlocal count
            count+=1
            if count==2:raise OSError("copy fixture")
            return copy(*args,**kwargs)
        with patch.object(collector.shutil,"copy2",side_effect=fail_later),self.assertRaises(OSError):
            collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        self.assertFalse(self.snapshot.exists())
        self.assertFalse(list(self.snapshot.parent.glob(".matea-source-*")))
        self.assertTrue(collector.prepare_snapshot(self.reference,self.snapshot,self.pin).is_file())

    def test_legacy_manifest_is_upgraded_only_after_all_assets_match_the_pin(self):
        marker=collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        legacy=json.loads(marker.read_text(encoding="utf-8"))
        legacy.pop("schema");legacy.pop("source_modes")
        legacy["source_hashes"]={k:v for k,v in legacy["source_hashes"].items() if k.endswith((".rs",".toml"))}
        marker.write_text(json.dumps(legacy),encoding="utf-8")
        before=marker.read_bytes()
        (self.snapshot/self.asset).write_bytes(b"unreviewed")
        with self.assertRaisesRegex(AssertionError,"snapshot source changed"):
            collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        self.assertEqual(marker.read_bytes(),before)
        (self.snapshot/self.asset).write_bytes((self.reference/self.asset).read_bytes())
        collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        upgraded=json.loads(marker.read_text(encoding="utf-8"))
        self.assertEqual(upgraded["schema"],2)
        self.assertIn(self.asset.as_posix(),upgraded["source_hashes"])

    def test_unknown_existing_destination_and_reference_nested_output_are_preserved(self):
        self.snapshot.mkdir(parents=True)
        owned=self.snapshot/"owner.txt";owned.write_bytes(b"keep")
        with self.assertRaises((AssertionError,ValueError)):
            collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        self.assertEqual(owned.read_bytes(),b"keep")
        with self.assertRaises((AssertionError,ValueError)):
            collector.prepare_snapshot(self.reference,self.reference/"generated",self.pin)
        self.assertFalse((self.reference/"generated").exists())

    def test_untracked_additions_deletions_and_escaping_manifest_paths_fail_closed(self):
        marker=collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        original=marker.read_bytes()
        for kind in ["extra-file","extra-directory","missing-file","escape-key"]:
            with self.subTest(kind=kind):
                if kind=="extra-file":
                    path=self.snapshot/"unreviewed.json";path.write_bytes(b"{}");cleanup=lambda:path.unlink()
                elif kind=="extra-directory":
                    path=self.snapshot/"unreviewed";path.mkdir();cleanup=lambda:path.rmdir()
                elif kind=="missing-file":
                    path=self.snapshot/self.asset;data=path.read_bytes();path.unlink();cleanup=lambda:path.write_bytes(data)
                else:
                    value=json.loads(original);value["source_hashes"]["../escape"]="0"*64;marker.write_text(json.dumps(value));cleanup=lambda:marker.write_bytes(original)
                try:
                    with self.assertRaises((AssertionError,ValueError)):
                        collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
                finally:cleanup()

    def test_internal_links_are_dereferenced_and_external_links_rejected(self):
        try:
            (self.reference/"internal-link").symlink_to(self.asset)
        except OSError as error:
            if os.name=="nt" and getattr(error,"winerror",None)==1314:
                self.skipTest("OS denies physical symlink privilege; portable manifest guards still execute")
            raise
        self.git("config","core.symlinks","true")
        self.git("add","internal-link")
        self.pin["sha"]=self.commit_fixture()
        marker=collector.prepare_snapshot(self.reference,self.snapshot,self.pin)
        self.assertFalse((self.snapshot/"internal-link").is_symlink())
        self.assertEqual((self.snapshot/"internal-link").read_bytes(),(self.reference/self.asset).read_bytes())
        self.assertIn("internal-link",json.loads(marker.read_text())["dereferenced_internal_symlinks"])
        (self.reference/"outside-link").symlink_to(self.root/"outside.json")
        (self.root/"outside.json").write_bytes(b"{}")
        self.git("add","outside-link")
        self.pin["sha"]=self.commit_fixture()
        with self.assertRaises((AssertionError,ValueError)):
            collector.prepare_snapshot(self.reference,self.root/"other-snapshot",self.pin)
        self.assertFalse((self.root/"other-snapshot").exists())


if __name__=="__main__":
    unittest.main()
