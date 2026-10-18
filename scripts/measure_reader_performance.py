"""Compare trusted Git reader code on the same finite retained valid corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess
import sys
from types import SimpleNamespace
import optimization_evidence
from check_quality import execution_inputs
from fetch_upstream_issues import atomic_json
from measure_remaining_performance import measure

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def compile_reader(source, filename):
    # Repository-owned historical sources run with their CLI guard off.
    # Functions keep their own globals instead of using the current parser.
    namespace = {"__name__":"_matea_baseline_reader", "__file__":str(filename)}
    exec(compile(source, str(filename), "exec"), namespace)
    if not all(callable(namespace.get(name)) for name in ["records", "compare"]):
        raise ValueError("baseline requires records and compare functions")
    return SimpleNamespace(records=namespace["records"], compare=namespace["compare"])


def source_snapshot(revision):
    result={}
    for name in sorted(execution_inputs(ROOT)):
        raw=(ROOT/name).read_bytes()
        committed=subprocess.check_output(["git","show",revision+":"+name],cwd=ROOT)
        if raw!=committed:
            raise ValueError("commit measured source inputs first: "+name)
        result[name]=digest(raw)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory",type=Path,required=True)
    parser.add_argument("--before",default="29ffeea6dfb3c24430db239ea4b33b35a910de59",help="full trusted repository commit hash")
    args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    context={"schema":1,"status":"running","scope":"individual reader component; valid 46-case corpus only","runs":[]}
    destination=args.directory/"context.json"
    atomic_json(destination,context)
    try:
        if not re.fullmatch(r"(?:[a-f0-9]{40}|[a-f0-9]{64})",args.before):
            raise ValueError("full baseline commit hash required")
        revision=subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip()
        sources=source_snapshot(revision)
        path="scripts/optimization_evidence.py"
        old_source=subprocess.check_output(["git","show",args.before+":"+path],cwd=ROOT)
        baseline=compile_reader(old_source,ROOT/path)
        folder=ROOT/"docs/evidence/optimization/style"
        inputs={name:(folder/name).read_bytes() for name in ["before.jsonl","after.jsonl"]}
        before,after=(inputs[name].decode("utf-8") for name in ["before.jsonl","after.jsonl"])
        def operation(reader):
            def execute():
                rows=reader.compare(reader.records(before),reader.records(after),46)
                return json.dumps(rows,sort_keys=True,ensure_ascii=False,separators=(",",":")).encode("utf-8")
            return execute
        operations={"before":operation(baseline),"after":operation(optimization_evidence)}
        if operations["before"]()!=operations["after"]():
            raise ValueError("valid-corpus complete report output changed")
        context.update(git_head=revision,source_sha256=sources,
                       baseline={"git_head":args.before,"source":path,"source_sha256":digest(old_source)},
                       input_sha256={name:digest(raw) for name,raw in inputs.items()},
                       platform=platform.platform(),python=sys.version,
                       limits=["31 samples and a separate Python heap pass per run; no build/test/Miri/coverage/download overlap during timing.",
                               "Before/after/after/before, same valid 92-record input and complete 46-case serialized output; new validation checks are intentional work.",
                               "Python tracemalloc is not native allocation/RSS. No affinity/frequency control or universal optimum claim."])
        atomic_json(destination,context)
        for index,stage in enumerate(["before","after","after","before"],1):
            record=measure("readers/strict-46-"+stage,"scripts/performance-reporters",operations[stage],
                           digest(b"".join(inputs.values())),
                           "decode/validate 92 modern records, compare all 46 matching cases and serialize the deterministic report; no file/Git/process I/O inside timing",True)
            raw=(json.dumps(record)+"\n").encode("utf-8")
            name=f"run-{index}-{stage}.jsonl";(args.directory/name).write_bytes(raw)
            context["runs"].append({"stage":stage,"raw":name,"raw_sha256":digest(raw),"p50_ns":record["p50_ns"],"p95_ns":record["p95_ns"],"python_heap":record["python_heap"]})
            atomic_json(destination,context)
        if source_snapshot(revision)!=sources:
            raise ValueError("measured source changed during collection")
        context["status"]="complete"
        atomic_json(destination,context)
        return 0
    except Exception as error:
        context.update(status="failed",error=str(error).replace(str(ROOT),"${REPOSITORY}").replace(str(Path.home()),"${HOME}"))
        atomic_json(destination,context)
        return 1


if __name__=="__main__":
    raise SystemExit(main())
