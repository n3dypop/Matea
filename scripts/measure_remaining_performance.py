"""Measure remaining native example/tool paths without implying whole-unit parity.

Build before timing. Keep 31 sequential samples, check complete deterministic
outputs, and separate Python heap tracing from all timing passes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time
import tracemalloc
from check_color_trace import verify, verify_fallback
from check_terminal_trace import replay
from check_quality import assess, programs
from fetch_upstream_issues import refresh
from import_rustdoc import plan_import
from optimization_evidence import records, compare
from performance_records import read_records, summary
from report_performance import generate

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def measure(name, unit, operation, inputs, semantics, python_heap=False):
    # Validation/serialization performed by an operation is part of its stated
    # workload. The equality/hash checks below are outside every timed interval.
    baseline = operation()
    samples = []
    for _ in range(31):
        start = time.perf_counter_ns()
        value = operation()
        elapsed = time.perf_counter_ns() - start
        if value != baseline:
            raise ValueError("nondeterministic workload output: " + name)
        samples.append(elapsed)
    heap = None
    if python_heap:
        tracemalloc.start()
        try:
            value = operation()
            retained, peak = tracemalloc.get_traced_memory()
            if value != baseline:
                raise ValueError("heap pass changed output: " + name)
            heap = {"retained_bytes": retained, "peak_bytes": peak,
                    "scope": "Python tracemalloc allocations only, separate execution; not subprocess or native RSS"}
        finally:
            tracemalloc.stop()
    return {"case": name, "unit": unit, "semantics": semantics, "input_sha256": inputs,
            "output_sha256": digest(baseline), "output_bytes": len(baseline),
            "samples_ns": samples, "p50_ns": statistics.median(samples), "p95_ns": sorted(samples)[29],
            "python_heap": heap, "allocations": None, "resident_memory": None}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    sources = [p for base in ["crates", "tools", "scripts"] for p in (ROOT/base).rglob("*")
               if p.suffix in {".rs", ".go", ".py", ".toml"} and "__pycache__" not in p.parts]
    sources += [ROOT/"Cargo.toml", ROOT/"Cargo.lock", ROOT/"docs/quality-plan.json"]
    source_hashes = {}
    for path in sorted(sources):
        name = path.relative_to(ROOT).as_posix()
        committed = subprocess.check_output(["git", "show", revision+":"+name], cwd=ROOT)
        if committed != path.read_bytes():
            raise ValueError("commit source inputs before measurement: " + name)
        source_hashes[name] = digest(committed)
    build = ["cargo", "build", "--offline", "--locked", "--release", "--workspace", "--examples", "--bins"]
    subprocess.run(build, cwd=ROOT, check=True, env=dict(os.environ, CARGO_TARGET_DIR=str(ROOT/"target")))
    cases = []
    binary_hashes = {}
    environment = dict(os.environ, NO_COLOR="1", MATEA_REDUCED_MOTION="1")
    suffix = ".exe" if os.name == "nt" else ""
    for name, expected in [("counter", "Count: 0"), ("list", "Matcha"), ("form", "Favorite tea"),
                           ("dashboard", "Tea studio"), ("effects", "Loading tea"), ("motion", "Matea motion"),
                           ("input", "Matea input"), ("pagination", "Page 1 of 4"), ("help", "Matea key help"),
                           ("clocks", "Matea clocks"), ("spinners", "Matea spinners"), ("external-widget", "Third-party badge")]:
        binary = ROOT/"target/release"/("matea-example-external-widget"+suffix) if name=="external-widget" else ROOT/"target/release/examples"/(name+suffix)
        binary_hashes[name] = digest(binary.read_bytes())
        def example(binary=binary, name=name, expected=expected):
            output = subprocess.check_output([str(binary), "--snapshot"], cwd=ROOT, env=environment, timeout=30)
            if expected not in output.decode("utf-8") or b"\x1b" in output:
                raise ValueError("example content/control mismatch: "+name)
            if name=="dashboard" and output != (ROOT/"tests/golden/dashboard.txt").read_bytes():
                raise ValueError("dashboard golden mismatch")
            return output
        cases.append(measure("examples/"+name, "examples", example, binary_hashes[name],
                             "complete release example snapshot process; includes spawn, runtime/frame construction, serialization, pipe and exit; expected content/no ESC and dashboard golden checked; no Cargo or interactive I/O"))
    base = ROOT/"docs/evidence/performance/2026-10-16"
    for name, source, function in [
        ("ascii", base/"matea-trace.json", lambda d: replay(d)),
        ("cursor", base/"matea-cursor-trace.json", lambda d: replay(d, include_cursor=True)),
        ("color", base/"matea-color-trace.json", verify),
        ("color-margins", base/"matea-color-trace.json", lambda d: verify(d, nondefault_margins=True)),
        ("unicode-fallback", ROOT/"docs/evidence/optimization/fallback/after-trace.json", verify_fallback),
    ]:
        data = source.read_bytes(); document=json.loads(data)
        def oracle(function=function, document=document):
            value = function(document)
            return canonical(value)
        cases.append(measure("oracles/"+name, "scripts/independent-terminal-oracles", oracle, digest(data),
                             "complete finite independent replay/verification with existing parsed trace input; includes oracle result serialization; no subprocess or input file read", True))
    pins = {p["name"]:p["sha"] for p in json.loads((ROOT/"docs/upstream-lock.json").read_bytes())}
    with tempfile.TemporaryDirectory(prefix="matea-tool-load-") as temporary:
        destination=Path(temporary)/"never-published"
        folder=ROOT/"docs/coverage/rustdoc"
        index=json.loads((folder/"index.json").read_bytes())
        def compiler():
            files, result=plan_import([folder], destination, pins)
            if len(result["snapshots"])!=len(index["snapshots"]):
                raise ValueError("incomplete profile import")
            return canonical({name:digest(data) for name,data in files.items()})
        inputs=canonical({p.name:digest(p.read_bytes()) for p in sorted(folder.iterdir()) if p.is_file()})
        cases.append(measure("compiler/validate-all-profile-imports", "scripts/compiler-export-audit", compiler, digest(inputs),
                             "complete validation/canonicalization/import planning of all retained compiler profiles; reads actual normalized facts, hashes planned files; no external compiler/network or publication", True))
    source=ROOT/"docs/coverage/upstream-issues.json";data=source.read_bytes(); previous=json.loads(data)
    pages={r["repository"]:[[{"number":i["number"],"title":i["title"],"html_url":i["url"],"updated_at":i["updated_at"],"labels":[{"name":n} for n in i["labels"]]} for i in r["issues"]]] for r in previous["repositories"]}
    expected=sum(len(r["issues"]) for r in previous["repositories"])
    def issues():
        result=refresh(previous, pages.__getitem__);result.pop("as_of_utc")
        if sum(r["open_issue_count"] for r in result["repositories"])!=expected:
            raise ValueError("issue counts changed")
        return canonical(result)
    cases.append(measure("issues/preserve-all-retained-reviews", "scripts/coverage-and-issues", issues, digest(data),
                         "merge/validate/serialize all retained issue records with simulated unchanged API pages; preserves reviews; excludes real network/pagination transport and file publication", True))
    def coverage():
        result=subprocess.check_output([sys.executable,"scripts/check_coverage.py"],cwd=ROOT,timeout=30)
        if b"F5 CLOSED" not in result:raise ValueError("unexpected coverage gate state")
        return result
    cases.append(measure("coverage/complete-current-ledger-cli", "scripts/coverage-and-issues", coverage,
                         digest(canonical({p.relative_to(ROOT).as_posix():digest(p.read_bytes()) for p in sorted((ROOT/"docs/coverage").rglob("*.json"))})),
                         "complete ledger validation CLI including spawn, all reference/profile JSON parsing, hash/link checks and exit; pending scope must remain closed"))
    def reporter():
        files, document=generate(base)
        result=json.loads(files["report.json"]);result.pop("recorded_at_utc")
        if len(result["cases"])!=84:raise ValueError("incomplete retained matched comparison")
        return canonical({"report":result,"document_sha256":digest(document)})
    cases.append(measure("reporters/generate-complete-retained-comparison", "scripts/performance-reporters", reporter,
                         digest(canonical({p.name:digest(p.read_bytes()) for p in sorted(base.iterdir()) if p.is_file()})),
                         "validate retained raw/history, replay independent traces and generate full 84-case comparison/document in memory; includes read/hash/compiler-version metadata subprocesses; excludes publication; timestamp omitted only from output identity", True))
    current=json.loads((ROOT/"docs/quality-plan.json").read_bytes())
    def quality():
        result=assess(current,programs(ROOT))
        if result["review_units"]!=40 or result["all_high"]:raise ValueError("unexpected assessment scope/gate")
        return canonical(result)
    cases.append(measure("acceptance/inventory-and-assess-all-programs", "scripts/test-and-acceptance-drivers", quality,
                         digest((ROOT/"docs/quality-plan.json").read_bytes()),
                         "complete Git source discovery and all six-dimension unit assessments; includes Git process and JSON result serialization; excludes LLVM profiling, PTY and full acceptance suite", True))
    optimization=ROOT/"docs/evidence/optimization/style"
    before_bytes=(optimization/"before.jsonl").read_bytes();after_bytes=(optimization/"after.jsonl").read_bytes()
    def measurement_report():
        before=records(before_bytes.decode());after=records(after_bytes.decode())
        return canonical(compare(before,after,46))
    cases.append(measure("reporters/strict-46-case-comparison", "scripts/performance-reporters", measurement_report,
                         digest(before_bytes+after_bytes),
                         "decode/validate 92 modern probe records and compare all 46 matched cases, preserving p50/p95/allocation regressions; serialized result; no Git provenance check or publication", True))
    # Exercise the shared historical reader separately, including Go averages.
    data=(base/"upstream-current.jsonl").read_bytes()
    def historical():
        return canonical({name:summary(item) for name,item in read_records(data.decode()).items()})
    cases.append(measure("reporters/historical-84-case-reader", "scripts/performance-reporters", historical, digest(data),
                         "decode/validate/summarize every retained Go reference case; preserve fractional average counters; no Rust/Go execution", True))
    args.directory.mkdir(parents=True,exist_ok=True)
    raw=args.directory/"current.jsonl";raw.write_text("".join(json.dumps(c)+"\n" for c in cases),encoding="utf-8")
    context={"schema":1,"stage":"individual_modules","git_head":revision,"platform":platform.platform(),
             "python":sys.version,"rustc":subprocess.check_output(["rustc","-Vv"],text=True),"build_command":build,
             "source_sha256":source_hashes,"binary_sha256":binary_hashes,"raw_sha256":digest(raw.read_bytes()),
             "limits":["One native host; 31 sequential warm samples per path, no affinity/frequency control or removed outliers.",
                       "Partial paths within review units; this does not establish full scope, high scores, upstream parity or Stage B optimization.",
                       "Example process timings include spawn/pipe/exit. Python heap is separately traced and excludes subprocess/native heaps and OS residency.",
                       "External compiler/network and physical terminal paths remain outside these tool measurements."]}
    (args.directory/"context.json").write_text(json.dumps(context,indent=2)+"\n",encoding="utf-8")
    for c in cases: print(c["case"],"p50_ms",round(c["p50_ns"]/1e6,3),"p95_ms",round(c["p95_ns"]/1e6,3))


if __name__ == "__main__":
    main()
