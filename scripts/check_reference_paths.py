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


def check_viewport_wrap_fixtures(output, fixture):
    actual = [line.split("\t") for line in output.decode("utf-8").splitlines()]
    expected = [line.split("\t") for line in fixture.decode("utf-8").splitlines()]
    require(actual and len(actual) == len(expected), "wrap matrix row count changed")
    portable, zero = 0, 0
    for a, b in zip(actual, expected):
        require(len(a) == len(b) == 13 and a[:6] == b[:6], "wrap matrix input identity changed")
        for value in a[6:10]:
            number = int(value)
            require(value == str(number) and -(1 << 63) <= number < (1 << 63), "wrap matrix invalid integer")
        for value in a[10:]:
            require(len(value) % 2 == 0 and all(c in "0123456789abcdef" for c in value), "wrap matrix invalid hex")
        if a[2] == "0":
            # Go's out-of-range float-to-int conversion / subsequent signed
            # overflow is target dependent. Keep all raw totals/positions;
            # never coerce them into matching the Mac observation or Matea.
            require(a[8:10] == ["0", "0"] and a[10:] == ["", "", ""], "zero-width source geometry changed")
            zero += 1
        else:
            require(a == b, "portable wrap matrix observation changed")
            portable += 1
    return {"rows": len(actual), "portable_exact_rows": portable, "platform_zero_width_rows_retained": zero}


def check_timing_mode(destination, probe_binary, environment, correctness, label="upstream", group="foundation"):
    require(label in ["upstream","matea"],"unknown timing-mode probe label")
    counts = {"foundation":(91,91), "gradients":(6,12), "progress":(6,12), "viewport-geometry":(14,18), "viewport-wrap":(10,12)}
    require(group in counts,"unknown timing-mode fixture group")
    target=20_000_000
    environment = dict(environment, MATEA_PERF_ITERATIONS="1", MATEA_PERF_MIN_SAMPLE_NS=str(target))
    environment.pop("MATEA_PROBE_OUTPUT_ONLY", None)
    rejected=[]
    if label=="matea" and group=="foundation":
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
    count=counts[group][label=="matea"]
    require(len(checked)==count and len({r["case"] for r in checked})==count, "incomplete calibrated probe")
    result={"schema":1,"probe":label,"group":group,"checked_cases":checked,"target_ns":target,"zero_or_invalid_samples":0,
            "invalid_configurations_rejected":rejected,
            "limits":"Measurement-mode/schema and exact-output portability check only. Shared-runner distributions/allocations are not performance comparisons and are deliberately not published."}
    filename="timing-mode-checks.json" if label=="upstream" else "rust-timing-mode-checks.json"
    if group!="foundation":filename=label+"-"+("gradient" if group=="gradients" else group)+"-timing-mode-checks.json"
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
    gradients=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary),"--gradients"],env=environment,timeout=30)
        gradients.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-gradient-outputs.jsonl")).write_bytes(output)
    require(len(gradients[0])==12 and len(gradients[1])==6,"gradient fixture set changed")
    for name,b in gradients[1].items():
        require(name.startswith("gradients/fresh_") and name in gradients[0],"unknown matched gradient")
        a=gradients[0][name]
        require(all(a[key]==b[key] for key in ["semantics","output","first_return_value"]),"gradient output mismatch: "+name)
    check_timing_mode(destination,go_binary,environment,[([],records[1]),(["--virtual-cursor"],cursors[1])])
    check_timing_mode(destination,rust_binary,environment,[([],records[0]),(["--virtual-cursor"],cursors[0])],label="matea")
    check_timing_mode(destination,go_binary,environment,[(["--gradients"],gradients[1])],group="gradients")
    check_timing_mode(destination,rust_binary,environment,[(["--gradients"],gradients[0])],label="matea",group="gradients")
    progress=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary),"--progress"],env=environment,timeout=30)
        progress.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-progress-outputs.jsonl")).write_bytes(output)
    require(len(progress[0])==12 and len(progress[1])==6,"progress fixture set changed")
    require(set(progress[0])==set(progress[1])|{name.replace("/frame_","/cells_") for name in progress[1]},"progress control set changed")
    for name,b in progress[1].items():
        require(name.startswith("progress/frame_"),"unknown matched progress")
        a=progress[0][name]
        require(all(a[key]==b[key] for key in ["semantics","output","first_return_value"]),"progress output mismatch: "+name)
    check_timing_mode(destination,go_binary,environment,[(["--progress"],progress[1])],group="progress")
    check_timing_mode(destination,rust_binary,environment,[(["--progress"],progress[0])],label="matea",group="progress")
    geometry=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary),"--viewport-geometry"],env=environment,timeout=30)
        geometry.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-viewport-geometry-outputs.jsonl")).write_bytes(output)
    require(len(geometry[0])==18 and len(geometry[1])==14,"viewport geometry fixture set changed")
    require(set(geometry[0])==set(geometry[1])|{"viewport-geometry/borrowed_"+boundary+"_"+text for boundary in ["aligned","partial"] for text in ["ascii","unicode"]},"viewport geometry control set changed")
    for name,b in geometry[1].items():
        require(all(geometry[0][name][key]==b[key] for key in ["semantics","output","first_return_value"]),"viewport geometry mismatch: "+name)
    check_timing_mode(destination,go_binary,environment,[(["--viewport-geometry"],geometry[1])],group="viewport-geometry")
    check_timing_mode(destination,rust_binary,environment,[(["--viewport-geometry"],geometry[0])],label="matea",group="viewport-geometry")
    varied=[]
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output=subprocess.check_output([str(binary),"--viewport-geometry-varied"],env=environment,timeout=30)
        varied.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-viewport-geometry-varied-outputs.jsonl")).write_bytes(output)
    require(varied[0].keys()==geometry[0].keys() and varied[1].keys()==geometry[1].keys(),"varied viewport fixture set changed")
    for name,b in varied[1].items():
        require(all(varied[0][name][key]==b[key] for key in ["semantics","output","first_return_value"]),"varied viewport geometry mismatch: "+name)
    wrapped = []
    for label,binary in [("matea",rust_binary),("upstream",go_binary)]:
        output = subprocess.check_output([str(binary),"--viewport-wrap"],env=environment,timeout=30)
        wrapped.append(read_probe_outputs(output.decode("utf-8")))
        (destination/(label+"-viewport-wrap-outputs.jsonl")).write_bytes(output)
    require(len(wrapped[0])==12 and len(wrapped[1])==10,"incomplete wrapped viewport outputs")
    require(set(wrapped[0])==set(wrapped[1])|{"viewport-wrap/borrowed_ascii","viewport-wrap/borrowed_unicode"},"wrapped iterator control set changed")
    for name,b in wrapped[1].items():
        require(all(wrapped[0][name][key]==b[key] for key in ["semantics","output","first_return_value"]),"wrapped viewport output mismatch: "+name)
    check_timing_mode(destination,go_binary,environment,[(["--viewport-wrap"],wrapped[1])],group="viewport-wrap")
    check_timing_mode(destination,rust_binary,environment,[(["--viewport-wrap"],wrapped[0])],label="matea",group="viewport-wrap")
    fixtures={}
    numeric={}
    retained={"--key-source-edges":("key-source-edges.json","upstream_observed"),
              "--source-regressions":("motion-pagination-source-edges.json","upstream_observed"),
              "--help-source-edges":("help-source-edges.json","observations"),
              "--clock-source-edges":("clocks-source-edges.json","observations"),
              "--spinner-source-edges":("spinner-source-edges.json","observations"),
              "--cursor-source-edges":("cursor-source-edges.json","observations")}
    for flag in ["--key-source-edges","--source-regressions","--textinput-source-edges","--help-source-edges","--clock-source-edges","--spinner-source-edges","--cursor-source-edges","--duration-fixtures","--spinner-fixtures","--motion-fixtures","--projectile-fixtures","--clocks","--gradient-fixtures","--progress-fixtures","--indexed-gradient-fixtures","--viewport-content-fixtures","--viewport-horizontal-fixtures","--viewport-cut-fixtures","--viewport-input-fixtures","--viewport-geometry-edges","--viewport-wrap-fixtures"]:
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
        if flag=="--gradient-fixtures":
            fixture_text=(ROOT/"crates/matea-style/tests/fixtures/gradients.tsv").read_text(encoding="utf-8")
            require(len(output.decode("utf-8").splitlines())==548 and output.decode("utf-8").splitlines()==fixture_text.splitlines(),"gradient fixtures changed")
        if flag=="--indexed-gradient-fixtures":
            fixture_text=(ROOT/"crates/matea-style/tests/fixtures/indexed-gradients.tsv").read_text(encoding="utf-8")
            require(len(output.decode("utf-8").splitlines())==256 and output.decode("utf-8").splitlines()==fixture_text.splitlines(),"indexed gradient fixtures changed")
        if flag=="--progress-fixtures":
            fixture_text=(ROOT/"crates/matea-widgets/tests/fixtures/progress.tsv").read_text(encoding="utf-8")
            require(len(output.decode("utf-8").splitlines())==120 and output.decode("utf-8").splitlines()==fixture_text.splitlines(),"progress fixtures changed")
        if flag=="--viewport-content-fixtures":
            require(len(output.splitlines())==220 and output==(ROOT/"crates/matea-widgets/tests/fixtures/viewport-controls.tsv").read_bytes(),"viewport content fixtures changed")
        if flag=="--viewport-horizontal-fixtures":
            require(len(output.splitlines())==1404 and output==(ROOT/"crates/matea-widgets/tests/fixtures/viewport-horizontal.tsv").read_bytes(),"viewport horizontal fixtures changed")
        if flag=="--viewport-cut-fixtures":
            require(len(output.splitlines())==200 and output==(ROOT/"crates/matea-widgets/tests/fixtures/viewport-cuts.tsv").read_bytes(),"viewport cut fixtures changed")
        if flag=="--viewport-input-fixtures":
            require(len(output.splitlines())==34 and output==(ROOT/"crates/matea-widgets/tests/fixtures/viewport-input.tsv").read_bytes(),"viewport input fixtures changed")
        if flag=="--viewport-geometry-edges":
            require(len(output.splitlines())==10 and output==(ROOT/"crates/matea-widgets/tests/fixtures/viewport-geometry-edges.tsv").read_bytes(),"viewport geometry edges changed")
        if flag=="--viewport-wrap-fixtures":
            wrap = check_viewport_wrap_fixtures(output, (ROOT/"crates/matea-widgets/tests/fixtures/viewport-wrap-source.tsv").read_bytes())
            require(wrap == {"rows":1008,"portable_exact_rows":840,"platform_zero_width_rows_retained":168}, "incomplete wrap matrix")
            (destination/"viewport-wrap-checks.json").write_text(json.dumps(wrap,indent=2)+"\n",encoding="utf-8")
        (destination/(flag.removeprefix("--")+".txt")).write_bytes(output)
        fixtures[flag]=hashlib.sha256(output).hexdigest()
    output = subprocess.check_output([str(rust_binary),"--viewport-wrap-fixtures"],env=environment,timeout=60)
    require(output == (ROOT/"crates/matea-widgets/tests/fixtures/viewport-wrap-matea.tsv").read_bytes(),"native Matea wrap states changed")
    (destination/"matea-viewport-wrap-fixtures.txt").write_bytes(output)
    fixtures["--matea-viewport-wrap-fixtures"] = hashlib.sha256(output).hexdigest()
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
        "gradient_matched_cases":6,"gradient_fixture_cases":548,"rust_gradient_timing_cases":12,"go_gradient_timing_cases":6,
        "indexed_gradient_fixture_cases":256,"progress_matched_cases":6,"progress_fixture_cases":120,"rust_progress_timing_cases":12,"go_progress_timing_cases":6,
        "viewport_content_fixture_cases":220,"viewport_horizontal_fixture_cases":1404,"viewport_cut_fixture_cases":200,"viewport_input_fixture_cases":34,"viewport_geometry_edge_cases":10,"viewport_geometry_matched_cases":14,"viewport_geometry_varied_matched_cases":14,"rust_viewport_geometry_timing_cases":18,"go_viewport_geometry_timing_cases":14,"source_fixture_sha256":fixtures,"historical_reporters_checked":5,"network_issues_executed":not args.offline,
        "limits":["CLI correctness and bounded path portability only; all shared-runner timing samples are excluded from performance conclusions.",
                  "Source declarations are not resolved API graphs. Reference compiler profiles, full public capability mapping and feature closure remain separate."]},indent=2)+"\n",encoding="utf-8")
    print("11 pinned inventories, 84 retained and seven cursor output cases validated; no performance verdict")


if __name__=="__main__":
    main()
