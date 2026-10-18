"""Execute pinned source/probe/network paths; shared-runner times are not evidence."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
from check_color_trace import verify
from check_terminal_trace import replay
from evidence_checks import require
from performance_records import read_probe_outputs, read_records
from check_coverage import verify_inventory_summary

ROOT = Path(__file__).resolve().parents[1]


def command(arguments, cwd=ROOT, env=None):
    subprocess.run(list(map(str, arguments)), cwd=cwd, env=env, check=True, timeout=300)


def fetch_all(destination, pins):
    require(not destination.exists(), "choose an empty reference directory")
    destination.mkdir(parents=True)
    fetched = set()
    for pin in pins:
        name = "charm-x" if pin["name"].startswith("charm-x-") else pin["name"]
        checkout = destination / name
        if name not in fetched:
            checkout.mkdir()
            command(["git", "init", "--quiet"], checkout)
            command(["git", "remote", "add", "origin", pin["remote"]], checkout)
        command(["git", "fetch", "--depth", "1", "origin", pin["sha"]], checkout)
        if name not in fetched:
            command(["git", "checkout", "--quiet", "--detach", "FETCH_HEAD"], checkout)
        actual = subprocess.check_output(["git", "rev-parse", pin["sha"]+"^{commit}"], cwd=checkout, text=True).strip()
        require(actual == pin["sha"], "reference pin mismatch")
        fetched.add(name)


def equivalent(actual, expected):
    if type(actual) is float or type(expected) is float:
        return type(actual) in (int, float) and type(expected) in (int, float) and math.isfinite(actual) and math.isfinite(expected) and math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-12)
    if type(actual) is not type(expected):
        return False
    if isinstance(actual, dict):
        return actual.keys() == expected.keys() and all(equivalent(actual[k], expected[k]) for k in actual)
    if isinstance(actual, list):
        return len(actual) == len(expected) and all(equivalent(a,b) for a,b in zip(actual,expected))
    return actual == expected


def check_trajectories(output, fixture, spring):
    actual = [[float(value) for value in line.split(",")] for line in output.splitlines()]
    expected = [[float(value) for value in line.split(",")] for line in fixture.splitlines()]
    require(actual and len(actual) == len(expected), "trajectory row count changed")
    input_count, columns = (2, 4) if spring else (5, 11)
    maximum = 0.0
    for row, (a, b) in enumerate(zip(actual, expected)):
        require(len(a) == len(b) == columns and all(math.isfinite(v) for v in a+b), "invalid trajectory shape/value")
        require(a[:input_count] == b[:input_count], "trajectory inputs changed")
        for column in range(input_count, columns):
            delta = abs(a[column]-b[column])
            maximum = max(maximum, delta)
            # Exactly the established contracts in matea-motion/tests/reference.rs:
            # springs use <1e-6 absolute; projectiles use <=1e-12*max(1,|expected|).
            limit = 1e-6 if spring else 1e-12*max(1.0,abs(b[column]))
            require(delta < limit if spring else delta <= limit,
                    f"trajectory row={row} column={column}: actual={a[column]!r} expected={b[column]!r} abs_error={delta!r} limit={limit!r}")
    return {"rows": len(actual), "maximum_absolute_error": maximum,
            "criterion": "existing matea-motion/tests/reference.rs; no Rust test threshold changed"}


def check_timing_mode(destination, probe_binary, environment, correctness, label="upstream"):
    require(label in ["upstream","matea"],"unknown timing-mode probe label")
    target=20_000_000
    environment = dict(environment, MATEA_PERF_ITERATIONS="1", MATEA_PERF_MIN_SAMPLE_NS=str(target))
    environment.pop("MATEA_PROBE_OUTPUT_ONLY", None)
    rejected=[]
    if label=="matea":
        for key,value in [("MATEA_PERF_ITERATIONS","0"),("MATEA_PERF_ITERATIONS","invalid"),
                          ("MATEA_PERF_ITERATIONS","6710887"),("MATEA_PERF_MIN_SAMPLE_NS","-1"),
                          ("MATEA_PERF_MIN_SAMPLE_NS","1000000001")]:
            failed=subprocess.run([str(probe_binary),"--style"],env={**environment,key:value},capture_output=True,timeout=30)
            require(failed.returncode!=0 and not failed.stdout,"invalid configuration printed successful/partial records")
            rejected.append({"variable":key,"value":value,"exit_code":failed.returncode,"stdout_bytes":len(failed.stdout)})
    checked=[]
    for flag, expected in correctness:
        output=subprocess.check_output([str(probe_binary),*flag],env=environment,timeout=300)
        records=read_records(output.decode("utf-8"))
        require(records.keys()==expected.keys(), "calibrated case set changed")
        for name,item in records.items():
            require(all(item[key]==expected[name][key] for key in ["semantics","output","first_return_value"]),
                    "calibrated output changed: "+name)
            require("calibration" in item and len(item["samples_ns"])==31, "calibrated measurement mode not executed")
            require(item["calibration"]["target_ns"]==target and
                    all(len(b.get("confirmations_ns",[]))==2 for b in item["calibration"]["batches"]),
                    "requested confirmed timing target not executed")
            checked.append({"case":name,"iterations_per_sample":item["iterations_per_sample"],
                            "calibration_batches":len(item["calibration"]["batches"]),"positive_samples":31,
                            "allocation_schema_valid":True})
    require(len(checked)==91 and len({r["case"] for r in checked})==91, "incomplete calibrated probe")
    result={"schema":1,"probe":label,"checked_cases":checked,"target_ns":target,"zero_or_invalid_samples":0,
            "invalid_configurations_rejected":rejected,
            "limits":"Measurement-mode/schema and exact-output portability check only. Shared-runner distributions/allocations are not performance comparisons and are deliberately not published."}
    filename="timing-mode-checks.json" if label=="upstream" else "rust-timing-mode-checks.json"
    (destination/filename).write_text(json.dumps(result,indent=2)+"\n",encoding="utf-8")


def check_probes(destination, go_binary, rust_binary, environment):
    environment = dict(environment, MATEA_PROBE_OUTPUT_ONLY="1")
    records=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary)],cwd=ROOT,env=environment,timeout=120)
        records.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-outputs.jsonl")).write_bytes(output)
        trace=json.loads(subprocess.check_output([str(binary),"--trace"],env=environment,timeout=30))
        require(replay(trace)==[c+"a"*79+"\n"*23 for c in "aba"], "ASCII trace mismatch")
        cursor=json.loads(subprocess.check_output([str(binary),"--cursor-trace"],env=environment,timeout=30))
        require(replay(cursor,include_cursor=True)==[{"screen":"a"*80+"\n"*23,"cursor":[x,3],"visible":visible}
                for x,visible in [(2,True),(4,True),(2,True),(2,False)]], "cursor trace mismatch")
        color=json.loads(subprocess.check_output([str(binary),"--color-trace"],env=environment,timeout=30))
        verify(color)
    require(len(records[0])==84 and records[0].keys()==records[1].keys(), "comparison fixture set changed")
    for name,a in records[0].items():
        b=records[1][name]
        require((a["semantics"],a["output"])==(b["semantics"],b["output"]), "comparison mismatch: "+name)
    cursors=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary),"--virtual-cursor"],env=environment,timeout=30)
        cursors.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-cursor-outputs.jsonl")).write_bytes(output)
    require(len(cursors[0])==7 and cursors[0].keys()==cursors[1].keys(), "virtual cursor fixture set changed")
    for name,a in cursors[0].items():
        b=cursors[1][name]
        require((a["semantics"],a["output"],a["first_return_value"])==
                (b["semantics"],b["output"],b["first_return_value"]), "virtual cursor mismatch: "+name)
    check_timing_mode(destination,go_binary,environment,[([],records[1]),(["--virtual-cursor"],cursors[1])])
    check_timing_mode(destination,rust_binary,environment,[([],records[0]),(["--virtual-cursor"],cursors[0])],label="matea")
    fixtures={}
    numeric={}
    retained={"--key-source-edges":("key-source-edges.json","upstream_observed"),
              "--source-regressions":("motion-pagination-source-edges.json","upstream_observed"),
              "--help-source-edges":("help-source-edges.json","observations"),
              "--clock-source-edges":("clocks-source-edges.json","observations"),
              "--spinner-source-edges":("spinner-source-edges.json","observations"),
              "--cursor-source-edges":("cursor-source-edges.json","observations")}
    for flag in ["--key-source-edges","--source-regressions","--textinput-source-edges","--help-source-edges","--clock-source-edges","--spinner-source-edges","--cursor-source-edges","--duration-fixtures","--spinner-fixtures","--motion-fixtures","--projectile-fixtures","--clocks"]:
        output=subprocess.check_output([str(go_binary),flag],env=environment,timeout=60)
        require(output, "empty source fixture: "+flag)
        if flag in retained:
            filename, key=retained[flag]
            require(equivalent(json.loads(output),json.loads((ROOT/"docs/evidence"/filename).read_text(encoding="utf-8"))[key]), "source edge changed: "+flag)
        if flag in ["--duration-fixtures","--spinner-fixtures"]:
            filename="durations.tsv" if flag=="--duration-fixtures" else "spinners.tsv"
            require(output.decode("utf-8").splitlines()==(ROOT/"crates/matea-widgets/tests/fixtures"/filename).read_text(encoding="utf-8").splitlines(), "widget fixture changed: "+flag)
        if flag in ["--motion-fixtures","--projectile-fixtures"]:
            filename="harmonica-motion.csv" if flag=="--motion-fixtures" else "harmonica-projectiles.csv"
            numeric[flag]=check_trajectories(output.decode(),(ROOT/"crates/matea-motion/tests"/filename).read_text(encoding="utf-8"),flag=="--motion-fixtures")
        if flag=="--clocks":
            require(len(read_probe_outputs(output.decode("utf-8")))==16, "incomplete clock probe")
        (destination/(flag.removeprefix("--")+".txt")).write_bytes(output)
        fixtures[flag]=hashlib.sha256(output).hexdigest()
    (destination/"trajectory-errors.json").write_text(json.dumps(numeric,indent=2)+"\n",encoding="utf-8")
    return fixtures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--local-references", type=Path, help="reuse clean fixed local inputs without fetching/resetting")
    parser.add_argument("--offline", action="store_true", help="require cached Go dependencies and skip real issue API requests")
    args = parser.parse_args()
    destination = args.destination.resolve()
    require(not destination.is_relative_to(ROOT), "reference workspace must stay outside Matea")
    require(not destination.exists(), "destination must be new")
    destination.mkdir(parents=True)
    pins = json.loads((ROOT/"docs/upstream-lock.json").read_text(encoding="utf-8"))
    if args.local_references:
        references = args.local_references.resolve()
    else:
        require(not args.offline, "offline mode requires local references")
        references = destination/"references"
        fetch_all(references,pins)
    environment = dict(os.environ, PYTHONUTF8="1", CARGO_TARGET_DIR=str(ROOT/"target"))
    if args.offline:
        environment["GOPROXY"] = "off"
        environment["GOSUMDB"] = "off"  # Retained go.sum still checks cached module bytes; no sumdb network.
    suffix = ".exe" if os.name=="nt" else ""
    build=["cargo","build","--locked","--release","-p","matea-api-audit","-p","matea-perf-probe"]
    if args.offline:build.append("--offline")
    command(build,env=environment)
    go_auditor=destination/("go-auditor"+suffix)
    command(["go","build","-o",go_auditor,ROOT/"tools/audit-go/main.go"],env=environment)
    facts=destination/"source-facts"
    command([sys.executable,"scripts/audit_references.py","--references",references,"--go-tool",go_auditor,
             "--rust-tool",ROOT/("target/release/matea-api-audit"+suffix),"--destination",facts],env=environment)
    summary=json.loads((facts/"summary.json").read_text(encoding="utf-8"))
    expected=json.loads((ROOT/"docs/coverage/summary.json").read_text(encoding="utf-8"))
    verify_inventory_summary(facts, [json.loads((facts/(pin["name"]+".json")).read_text(encoding="utf-8")) for pin in pins])
    require([(r["name"],r["sha"],r["declarations"]) for r in summary["repositories"]] ==
            [(r["name"],r["sha"],r["declarations"]) for r in expected["repositories"]], "source inventory counts changed")
    for pin in pins:
        name=pin["name"]
        actual=json.loads((facts/(name+".json")).read_text(encoding="utf-8"))
        retained=json.loads((ROOT/"docs/coverage"/(name+".json")).read_text(encoding="utf-8"))
        require(actual["declarations"]==retained["declarations"], "source facts differ: "+name)
    go_workspace=destination/"go-probe"
    command([sys.executable,"scripts/prepare_upstream_bench.py","--references",references,"--destination",go_workspace],env=environment)
    command(["go","mod","tidy"],cwd=go_workspace,env=environment)
    require((go_workspace/"go.sum").read_bytes()==(ROOT/"tools/perf-upstream/go.sum").read_bytes(), "pinned Go checksum cohort changed")
    command(["go","test","."],cwd=go_workspace,env=environment)
    go_binary=go_workspace/("probe"+suffix)
    command(["go","build","-o",go_binary,"."],cwd=go_workspace,env=environment)
    rust_binary=ROOT/("target/release/matea-perf-probe"+suffix)
    fixtures=check_probes(destination,go_binary,rust_binary,environment)
    historical = ["report_view_performance.py", "report_subscription_performance.py",
                  "report_clock_format_performance.py", "report_viewport_performance.py"]
    # Validate retained Mac measurements on each OS without rewriting their
    # evidence or treating this invocation's binaries/times as the original run.
    for name in historical:
        command([sys.executable, ROOT/"scripts"/name, "--check"],env=environment)
    command([sys.executable,ROOT/"scripts/report_performance.py","--artifacts",
             ROOT/"docs/evidence/performance/2026-10-16","--check"],env=environment)
    if not args.offline:
        command([sys.executable,"scripts/fetch_upstream_issues.py","--destination",destination/"issue-metadata.json",
                 "--cache",destination/"private-issue-cache"],env=environment)
    (destination/"path-checks.json").write_text(json.dumps({"schema":1,"source_inventories":len(pins),"matched_cases":84,"virtual_cursor_matched_cases":7,"go_calibrated_timing_cases":91,
        "source_fixture_sha256":fixtures,"historical_reporters_checked":5,"network_issues_executed":not args.offline,
        "limits":["CLI correctness and bounded path portability only; all shared-runner timing samples are excluded from performance conclusions.",
                  "Source declarations are not resolved API graphs. Reference compiler profiles, full public capability mapping and feature closure remain separate."]},indent=2)+"\n",encoding="utf-8")
    print("11 pinned inventories, 84 retained and seven cursor output cases validated; no performance verdict")


if __name__=="__main__":
    main()
