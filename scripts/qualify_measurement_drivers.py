"""Execute complete measurement-tool paths on a host; CI times are not speed evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from check_quality import execution_inputs
from check_reference_paths import fetch_all
from fetch_upstream_issues import atomic_json

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--references", type=Path, help="reuse clean pinned checkouts; otherwise fetch only the seven required inputs")
    parser.add_argument("--toolchain", default="nightly-2026-10-11")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    destination = args.destination.resolve()
    if destination.is_relative_to(ROOT):
        parser.error("raw captures and source checkouts must remain outside Matea")
    results = destination / "results"
    results.mkdir(parents=True, exist_ok=True)
    summary = {"schema": 1, "status": "running", "checks": [], "platform": platform.platform(),
               "limits": ["Actual host execution and complete output/schema checks only; all collected timings are excluded from speed conclusions.",
                          "One native all-features compiler capture per reference, not recollection or closure of all twenty public-API profiles."]}
    manifest = results / "qualification.json"
    atomic_json(manifest, summary)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/"target"), PYTHONUTF8="1",
                       GOCACHE=os.environ.get("GOCACHE",str(destination/"go-cache")))
    if args.offline:
        environment.update(CARGO_NET_OFFLINE="true", GOPROXY="off")

    def run(name, arguments, cwd=ROOT):
        summary["current_check"] = name
        atomic_json(manifest, summary)
        result = subprocess.run(list(map(str, arguments)), cwd=cwd, env=environment,
                                capture_output=True, timeout=900)
        log = results / (name+".log")
        log.write_bytes(result.stdout+result.stderr)
        summary["checks"].append({"name": name, "exit_code": result.returncode,
                                  "log": log.name, "log_sha256": digest(log)})
        atomic_json(manifest, summary)
        result.check_returncode()
        print(name+": passed", flush=True)

    try:
        summary["git_head"] = subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip()
        summary["source_sha256"] = {name:digest(ROOT/name) for name in sorted(execution_inputs(ROOT))}
        references = args.references.resolve() if args.references else destination/"references"
        if args.references is None:
            if args.offline:
                raise ValueError("offline qualification requires existing pinned references")
            names = {"crossterm","ratatui","bubbletea","bubbles","ultraviolet","harmonica","lipgloss"}
            pins = [p for p in json.loads((ROOT/"docs/upstream-lock.json").read_bytes()) if p["name"] in names]
            fetch_all(references, pins)
        run("build-release", ["cargo","build","--locked","--release","--workspace","--examples","--bins"])
        suffix = ".exe" if os.name=="nt" else ""
        # Measure each source auditor twice, on real and independent 10k fixtures.
        # This exercises before/after publication, never a same-code speed claim.
        for language, executable in [("rust","matea-api-audit"),("go","matea-go-audit")]:
            for stage in ["before","after"]:
                run("auditor-"+language+"-"+stage, [sys.executable,"scripts/measure_auditor_performance.py",
                    "--references",references,"--binary",ROOT/"target/release"/(executable+suffix),
                    "--directory",results/("auditor-"+language),"--language",language,"--stage",stage])
        # Only the two previously missing native extraction paths are collected.
        capture = destination/"captured"
        for reference, package in [("crossterm","crossterm"),("ratatui","ratatui-core")]:
            arguments = [sys.executable,"scripts/collect_rustdoc.py","--references",references,
                         "--destination",capture,"--reference",reference,"--package",package,
                         "--profile","all","--toolchain",args.toolchain]
            if args.offline:
                arguments.append("--offline")
            run("capture-"+reference, arguments)
        run("remaining-all", [sys.executable,"scripts/measure_remaining_performance.py","--directory",results/"remaining",
                              "--rustdoc-crossterm",capture,"--rustdoc-ratatui",capture])
        run("reader-balanced", [sys.executable,"scripts/measure_reader_performance.py","--directory",results/"reader"])
        run("prepare-go", [sys.executable,"scripts/prepare_upstream_bench.py","--references",references,
                           "--destination",destination/"go-probe"])
        run("go-module", ["go","mod","tidy"], destination/"go-probe")
        run("go-tests", ["go","test","."], destination/"go-probe")
        for script in ["report_view_performance.py","report_subscription_performance.py",
                       "report_clock_format_performance.py","report_viewport_performance.py"]:
            run(script.removesuffix(".py"), [sys.executable,"scripts/"+script,"--check"])
        run("report-comparison", [sys.executable,"scripts/report_performance.py","--artifacts",
                                  ROOT/"docs/evidence/performance/2026-10-16","--check"])
        run("report-attributes", [sys.executable,"scripts/report_attribute_performance.py",ROOT/"docs/evidence/attributes","--check"])
        if summary["source_sha256"] != {name:digest(ROOT/name) for name in sorted(execution_inputs(ROOT))}:
            raise ValueError("source changed during qualification")
        summary["status"] = "complete"
    except Exception as error:
        summary.update(status="failed", error=str(error).replace(str(ROOT),"${REPOSITORY}").replace(str(Path.home()),"${HOME}"))
        print(summary["error"],file=sys.stderr)
    atomic_json(manifest, summary)
    return 0 if summary["status"]=="complete" else 1


if __name__=="__main__":
    raise SystemExit(main())
