"""Independent iteration, immutable-output and changing-scope driver fixtures."""
import json
from pathlib import Path
import subprocess
import tempfile
import tracemalloc
import unittest
from unittest.mock import patch
from check_quality import DIMENSIONS, execution_inputs
from measure_remaining_performance import measure, quality_snapshot
from measure_reader_performance import compile_reader


class MeasurementDriverTests(unittest.TestCase):
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
                    "tools/probe/go.sum","Cargo.lock",".gitattributes","docs/quality-plan.json","docs/upstream-lock.json"}
            for name in wanted|{"README.md"}:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text("fixture",encoding="utf-8")
            self.assertEqual(execution_inputs(root),wanted)


if __name__=="__main__":unittest.main()
