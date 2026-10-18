"""Independent iteration, immutable-output and changing-scope driver fixtures."""
import json
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import tracemalloc
import unittest
from unittest.mock import patch
from check_quality import DIMENSIONS, execution_inputs
from measure_remaining_performance import measure, quality_snapshot
from measure_reader_performance import compile_reader
from measurement_run import MeasurementRun, atomic_bytes, positive_interval
from measure_auditor_performance import synthetic_fixture, validate_baseline
from measure_remaining_performance import captured_extraction
from rustdoc_exports import FORMAT, EXTRACTOR_SHA256, exports
from collect_rustdoc import MARKER


class MeasurementDriverTests(unittest.TestCase):
    def test_journal_keeps_partial_attempts_and_publishes_only_complete_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory);raw=folder/"current.jsonl";raw.write_bytes(b"old success")
            with self.assertRaisesRegex(ValueError,"simulated failure"):
                with MeasurementRun(folder) as run:
                    first=run.context["attempt"]
                    run.record({"case":"first","samples_ns":[1]*31})
                    raise ValueError("simulated failure")
            failed=json.loads((folder/"context.json").read_bytes())
            self.assertEqual((failed["status"],failed["completed_cases"]),("failed",1))
            self.assertEqual(raw.read_bytes(),b"old success")
            self.assertTrue((folder/first/"cases.jsonl").is_file())
            with MeasurementRun(folder) as run:
                self.assertNotEqual(first,run.context["attempt"])
                run.record({"case":"second","samples_ns":[2]*31})
                run.complete(source_sha256={"source":"fixture"})
            complete=json.loads((folder/"context.json").read_bytes())
            self.assertEqual(complete["status"],"complete")
            self.assertEqual(complete["raw_sha256"],hashlib.sha256(raw.read_bytes()).hexdigest())
            self.assertEqual(json.loads((folder/first/"context.json").read_bytes()),failed)

    def test_empty_duplicate_nonfinite_and_interrupted_attempts_cannot_complete(self):
        for mode in ["empty","duplicate","nonfinite","interrupt"]:
            with self.subTest(mode=mode),tempfile.TemporaryDirectory() as directory:
                kind=KeyboardInterrupt if mode=="interrupt" else ValueError
                with self.assertRaises(kind):
                    with MeasurementRun(Path(directory)) as run:
                        if mode=="interrupt":raise KeyboardInterrupt()
                        if mode=="empty":run.complete()
                        run.record({"case":"one","n":1})
                        if mode=="nonfinite":run.complete(value=float("nan"))
                        run.record({"case":"one","n":2})
                context=json.loads((Path(directory)/"context.json").read_bytes())
                self.assertEqual(context["status"],"interrupted" if mode=="interrupt" else "failed")
                self.assertFalse((Path(directory)/"current.jsonl").exists())

    def test_atomic_write_failure_preserves_old_bytes_and_removes_stage_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"data";path.write_bytes(b"old")
            with patch("measurement_run.os.replace",side_effect=OSError("replace failed")),self.assertRaises(OSError):
                atomic_bytes(path,b"new")
            self.assertEqual(path.read_bytes(),b"old")
            self.assertEqual(list(Path(directory).iterdir()),[path])

    def test_synthetic_manifest_hashes_written_utf8_bytes_including_lf(self):
        for language,name,rows in [("rust","lib.rs",10000),("go","fixture.go",10001)]:
            with self.subTest(language=language),tempfile.TemporaryDirectory(prefix="tea space-抹茶-") as directory:
                folder=Path(directory);digest=synthetic_fixture(folder,language);data=(folder/name).read_bytes()
                self.assertEqual(digest,hashlib.sha256(data).hexdigest())
                self.assertNotIn(b"\r",data)
                self.assertEqual(len(data.splitlines()),rows)
        for value in [True,False,0,-1,0.5,float("nan"),float("inf")]:
            with self.assertRaises(ValueError):positive_interval(value)
        self.assertEqual(positive_interval(1),1)

    def test_incomparable_baseline_fails_before_success_can_publish(self):
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory)
            row={"case":"one","semantics":"whole output","input":"pin","output_sha256":"hash","output_bytes":2,"declarations":1}
            with MeasurementRun(folder,"before-context.json","before.jsonl") as run:
                run.record(row);run.complete()
            validate_baseline(folder,[row])
            for field in row:
                with self.subTest(field=field),self.assertRaises(ValueError):
                    validate_baseline(folder,[{**row,field:"different"}])
            context=folder/"before-context.json";original=context.read_bytes()
            context.write_bytes(original.replace(b'"status": "complete"',b'"status": "failed", "status": "complete"'))
            with self.assertRaisesRegex(ValueError,"duplicate"):validate_baseline(folder,[row])
            context.write_bytes(original)
            (folder/"before.jsonl").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError,"hash-checked"):validate_baseline(folder,[row])

    def captured_fixture(self, directory):
        reference,package,host,pin="crossterm","crossterm","fixture-target","a"*40
        source=directory/"source"/reference;source.mkdir(parents=True)
        text=b"pub fn tea() {}\n";(source/"lib.rs").write_bytes(text)
        provenance={"schema":2,"reference":reference,"sha":pin,"source_hashes":{"lib.rs":hashlib.sha256(text).hexdigest()},
                    "source_modes":{"lib.rs":(source/"lib.rs").stat().st_mode & 0o777},"dereferenced_internal_symlinks":{}}
        marker=json.dumps(provenance).encode();(source/MARKER).write_bytes(marker)
        item={"id":0,"crate_id":0,"name":"fixture","span":None,"visibility":"public","inner":{"module":{"items":[]}},"deprecation":None}
        document={"format_version":FORMAT,"root":0,"paths":{},"index":{"0":item}}
        raw=json.dumps(document).encode();path=directory/"target"/reference/"all"/host/"doc"/(package+".json");path.parent.mkdir(parents=True);path.write_bytes(raw)
        identity={"reference":reference,"package":package,"profile":"all","target":host,"sha":pin,"extractor_sha256":EXTRACTOR_SHA256}
        run={**identity,"exit_code":0,"rustdoc_exit_code":0,"dependency_tree_exit_code":0,"source_manifest_sha256":hashlib.sha256(marker).hexdigest(),"rustc":"fixture"}
        graph={**exports(document,source),**identity,"raw_rustdoc_sha256":hashlib.sha256(raw).hexdigest()}
        reports=directory/"reports";reports.mkdir();stem=f"{reference}-{package}-all-{host}"
        (reports/(stem+"-run.json")).write_bytes(json.dumps(run).encode())
        (reports/(stem+"-exports.json")).write_bytes(json.dumps(graph).encode())
        return reference,package,directory,pin,host

    def test_captured_extraction_checks_actual_sources_metadata_raw_and_whole_graph(self):
        for changed in [None,"source","unexpected","marker","raw","run","graph"]:
            with self.subTest(changed=changed),tempfile.TemporaryDirectory() as directory:
                folder=Path(directory);arguments=self.captured_fixture(folder)
                operation,inputs,verify=captured_extraction(*arguments)
                self.assertEqual(json.loads(operation())["root"],"0")
                self.assertEqual(inputs["source_manifest_sha256"],hashlib.sha256((folder/"source/crossterm"/MARKER).read_bytes()).hexdigest())
                verify()
                paths={"source":folder/"source/crossterm/lib.rs","unexpected":folder/"source/crossterm/new.rs","marker":folder/"source/crossterm"/MARKER,
                       "raw":folder/"target/crossterm/all/fixture-target/doc/crossterm.json",
                       "run":folder/"reports/crossterm-crossterm-all-fixture-target-run.json",
                       "graph":folder/"reports/crossterm-crossterm-all-fixture-target-exports.json"}
                if changed is None:continue
                paths[changed].write_bytes(b"changed\n")
                with self.assertRaises((ValueError,AssertionError)):verify()

    def test_incomplete_or_wrong_identity_capture_is_rejected(self):
        for field,value in [("exit_code",False),("rustdoc_exit_code",1),("dependency_tree_exit_code",None),("sha","b"*40),("target","other"),("extractor_sha256","changed"),("source_manifest_sha256","changed")]:
            with self.subTest(field=field),tempfile.TemporaryDirectory() as directory:
                folder=Path(directory);arguments=self.captured_fixture(folder)
                path=folder/"reports/crossterm-crossterm-all-fixture-target-run.json"
                data=json.loads(path.read_bytes());data[field]=value;path.write_bytes(json.dumps(data).encode())
                with self.assertRaises(ValueError):captured_extraction(*arguments)

    def test_go_preparation_rejects_dirty_or_wrong_pins_without_publication(self):
        from prepare_upstream_bench import prepare
        for bad in ["wrong-pin","dirty","valid"]:
            with self.subTest(bad=bad),tempfile.TemporaryDirectory(prefix="tea space-抹茶-") as directory:
                folder=Path(directory);root=folder/"Matea";references=folder/"references";output=folder/"prepared"
                names=["ultraviolet","harmonica","bubbles","bubbletea","lipgloss"]
                (root/"docs").mkdir(parents=True);(root/"tools/perf-upstream").mkdir(parents=True)
                (root/"docs/upstream-lock.json").write_bytes(json.dumps([{"name":n,"sha":"a"*40} for n in names]).encode())
                for name in ["main.go","main_test.go","go.sum"]:(root/"tools/perf-upstream"/name).write_bytes((name+"\n").encode())
                def git(command,cwd,text,timeout):
                    if command[-1]=="HEAD":return "b"*40 if bad=="wrong-pin" else "a"*40
                    return " M file" if bad=="dirty" else ""
                with patch("prepare_upstream_bench.subprocess.check_output",side_effect=git):
                    if bad!="valid":
                        with self.assertRaises(ValueError):prepare(references,output,root)
                        self.assertFalse(output.exists())
                    else:
                        prepare(references,output,root)
                        pin=json.loads((output/"source-pin.json").read_bytes())
                        self.assertEqual(pin["status"],"complete")
                        for name,digest in pin["file_sha256"].items():self.assertEqual(hashlib.sha256((output/name).read_bytes()).hexdigest(),digest)
                        self.assertIn(json.dumps(str(references.resolve()/"bubbles")),(output/"go.mod").read_text())
                        for path in [root/"forbidden",references/"forbidden"]:
                            with self.assertRaises(ValueError):prepare(references,path,root)

    def test_go_preparation_failed_copy_never_leaves_complete_marker(self):
        from prepare_upstream_bench import prepare
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory);root=folder/"Matea";output=folder/"prepared";(root/"docs").mkdir(parents=True);(root/"tools/perf-upstream").mkdir(parents=True)
            names=["ultraviolet","harmonica","bubbles","bubbletea","lipgloss"]
            (root/"docs/upstream-lock.json").write_bytes(json.dumps([{"name":n,"sha":"a"*40} for n in names]).encode())
            for name in ["main.go","main_test.go","go.sum"]:(root/"tools/perf-upstream"/name).write_bytes(b"source\n")
            def git(command,**kwargs):return "a"*40 if command[-1]=="HEAD" else ""
            def write(path,data):
                if path.name=="go.sum":raise OSError("simulated disk failure")
                atomic_bytes(path,data)
            with patch("prepare_upstream_bench.subprocess.check_output",side_effect=git),patch("prepare_upstream_bench.atomic_bytes",side_effect=write),self.assertRaises(OSError):prepare(folder/"references",output,root)
            self.assertEqual(json.loads((output/"source-pin.json").read_bytes())["status"],"failed")

    def test_failed_auditor_invocation_replaces_old_success_manifest(self):
        from measure_auditor_performance import main
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory)
            manifest=folder/"before-context.json"
            manifest.write_text('{"status":"complete","old":true}',encoding="utf-8")
            (folder/"before.jsonl").write_bytes(b"old successful data\n")
            with patch("measure_auditor_performance.source_snapshot",return_value={}),patch.object(sys,"argv",["measure-auditor","--references",directory,"--binary",str(folder/"wrong"),"--directory",directory,"--stage","before"]):
                try: main()
                except ValueError: pass
            context=json.loads(manifest.read_bytes())
            self.assertEqual(context["status"],"failed")
            self.assertIn("binary",context["error"])
            self.assertEqual((folder/"before.jsonl").read_bytes(),b"old successful data\n")

    def test_failed_remaining_source_check_cannot_reuse_previous_success(self):
        from measure_remaining_performance import main
        with tempfile.TemporaryDirectory() as directory:
            folder=Path(directory);(folder/"changed.py").write_bytes(b"dirty source")
            manifest=folder/"context.json";manifest.write_text('{"status":"complete"}',encoding="utf-8")
            with patch.object(sys,"argv",["measure-remaining","--directory",directory]),patch("measure_remaining_performance.ROOT",folder),patch("measure_remaining_performance.execution_inputs",return_value={"changed.py"}),patch("measure_remaining_performance.subprocess.check_output",side_effect=["a"*40,b"committed source"]):
                try: main()
                except ValueError: pass
            context=json.loads(manifest.read_bytes())
            self.assertEqual(context["status"],"failed")
            self.assertIn("commit source",context["error"])

    def test_frozen_reader_keeps_its_globals_and_never_enters_its_cli(self):
        source='''\nVALUE = "frozen"\nif __name__ == "__main__":\n    raise RuntimeError("CLI must not execute")\ndef records(text):\n    return VALUE + text\ndef compare(a,b,count):\n    return (a,b,count,VALUE)\n'''
        reader=compile_reader(source,"fixture.py")
        self.assertEqual(reader.records(" input"),"frozen input")
        self.assertEqual(reader.compare(1,2,46),(1,2,46,"frozen"))
        for bad in ["records=1\ncompare=2", "def records(text): return text"]:
            with self.assertRaises(ValueError):compile_reader(bad,"fixture.py")

    def test_reader_dependencies_are_frozen_without_global_module_replacement(self):
        import performance_records
        original=performance_records.parse_json
        source='from performance_records import parse_json\ndef records(text): return parse_json(text)\ndef compare(a,b,count): return records(a)'
        dependency='VALUE="historical"\ndef parse_json(text): return VALUE + text'
        reader=compile_reader(source,"fixture.py",dependency)
        self.assertEqual(reader.records(" input"),"historical input")
        self.assertIs(performance_records.parse_json,original)
        self.assertEqual(performance_records.parse_json('{"n":1}'),{"n":1})

    def test_all_31_intervals_and_separate_heap_pass_are_retained(self):
        times=[value for n in range(1,32) for value in [1000*n,1000*n+n]]
        traced=[]
        def operation():traced.append(tracemalloc.is_tracing());return b"complete output"
        with patch("measure_remaining_performance.time.perf_counter_ns",side_effect=times):
            result=measure("fixture","unit",operation,"input-hash","finite output",True)
        self.assertEqual(result["samples_ns"],list(range(1,32)))
        self.assertEqual((result["p50_ns"],result["p95_ns"]),(16,30))
        self.assertEqual(traced,[False]*32+[True])
        self.assertIsNone(result["resident_memory"])
        self.assertFalse(tracemalloc.is_tracing())

    def test_changed_output_invalid_time_or_mutable_return_cannot_publish(self):
        for elapsed in [0,-1,0.5]:
            values=[100,100+elapsed]
            with self.subTest(elapsed=elapsed),patch("measure_remaining_performance.time.perf_counter_ns",side_effect=values),self.assertRaises(ValueError):
                measure("bad","unit",lambda:b"same","input","scope")
        with self.assertRaises(TypeError):measure("mutable","unit",lambda:bytearray(b"output"),"input","scope")
        values=iter([b"baseline",b"changed"])
        with self.assertRaises(ValueError):measure("changed","unit",lambda:next(values),"input","scope")

    def test_failed_heap_pass_always_stops_instrumentation(self):
        def operation():return b"changed" if tracemalloc.is_tracing() else b"baseline"
        with self.assertRaises(ValueError):measure("heap","unit",operation,"input","scope",True)
        self.assertFalse(tracemalloc.is_tracing())

    def test_review_scope_can_grow_and_eventually_become_high(self):
        for count in [1,2,41]:
            with self.subTest(count=count),tempfile.TemporaryDirectory() as directory:
                root=Path(directory);subprocess.run(["git","init","--quiet"],cwd=root,check=True,capture_output=True)
                units=[]
                for index in range(count):
                    name=f"unit_{index}.rs";(root/name).write_text("// independent fixture\n",encoding="utf-8")
                    units.append({"id":name,"files":[name],"evidence":["proof"],"remaining":["bounded fixture"],
                        "criteria":{d:{"score":75,"reason":"finite fixture"} for d in DIMENSIONS}})
                result=json.loads(quality_snapshot({"units":units},root))
                self.assertTrue(result["all_high"])
                self.assertEqual((result["programs"],result["review_units"]),(count,count))
                units[-1]["criteria"]["behavior"]["score"]=50
                self.assertEqual(len(json.loads(quality_snapshot({"units":units},root))["below_high"]),1)
                (root/"unreviewed.py").write_text("",encoding="utf-8")
                with self.assertRaises(ValueError):quality_snapshot({"units":units},root)

    def test_source_binding_includes_root_external_example_and_checksum_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);subprocess.run(["git","init","--quiet"],cwd=root,check=True,capture_output=True)
            wanted={"build.rs","scripts/check.py","examples/external/src/main.rs","examples/external/Cargo.toml",
                    "tools/probe/go.sum","Cargo.lock",".gitattributes","docs/quality-plan.json","docs/upstream-lock.json",
                    "crates/example/tests/fixtures/data.tsv","crates/example/tests/reference.csv","tests/golden/dashboard.txt"}
            for name in wanted|{"README.md"}:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text("fixture",encoding="utf-8")
            self.assertEqual(execution_inputs(root),wanted)


if __name__=="__main__":unittest.main()
