"""Check complete program inventory and report evidence-based ordinal assessments."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from check_coverage import load_json

ROOT = Path(__file__).resolve().parents[1]
DIMENSIONS = {"rust_design", "memory_safety", "behavior", "reference", "performance", "portability"}


def programs(root):
    # Git supplies the whole checkout, including untracked source and future
    # root/build/benchmark programs, while excluding generated ignored files.
    try:
        names = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                                        cwd=root, stderr=subprocess.PIPE).decode("utf-8").split("\0")
    except subprocess.CalledProcessError as error:
        raise ValueError("quality inventory requires a Git checkout") from error
    suffixes = {".rs", ".go", ".py", ".yml", ".yaml", ".sh", ".ps1", ".js", ".mjs", ".ts"}
    return {name for name in names if Path(name).suffix in suffixes and (root / name).is_file()}


def execution_inputs(root):
    """Programs and build/check manifests; exclude generated evidence."""
    names = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                                    cwd=root, stderr=subprocess.PIPE).decode("utf-8").split("\0")
    additional = {name for name in names if name and (root/name).is_file() and
                  (Path(name).suffix in {".toml", ".lock", ".sum", ".mod"} or
                   name in {".gitattributes", "docs/quality-plan.json", "docs/upstream-lock.json"})}
    return programs(root) | additional


def assess(plan, actual):
    """Fail closed on missing files, dimensions, explanations and evidence."""
    units = plan["units"]
    if len({u["id"] for u in units}) != len(units):
        raise ValueError("duplicate review unit")
    covered = set()
    for unit in units:
        if not unit["files"] or not unit["evidence"] or not unit["remaining"]:
            raise ValueError("missing scope/evidence/limits: " + unit["id"])
        covered.update(unit["files"])
        if set(unit["criteria"]) != DIMENSIONS:
            raise ValueError("missing assessment dimension: " + unit["id"])
        for dimension, value in unit["criteria"].items():
            score = value["score"]
            if (score is not None and (type(score) is not int or score not in (0, 25, 50, 75, 100))) or not value["reason"]:
                raise ValueError("invalid score or missing explanation: " + dimension)
    if covered != actual:
        raise ValueError(f"program inventory mismatch: missing={sorted(actual-covered)} stale={sorted(covered-actual)}")
    low = [{"unit": u["id"], "dimension": d, **v} for u in units for d, v in u["criteria"].items()
           if v["score"] is not None and v["score"] < 75]
    return {"all_high": not low, "programs": len(actual), "review_units": len(units), "below_high": low}


def validate_native(report, root):
    """Reject old, incomplete or changed native evidence before publication."""
    if report.get("status") != "complete" or report.get("all_local_checks_passed") is not True:
        raise ValueError("native verification incomplete/failed")
    checks = report.get("checks")
    if not isinstance(checks,list) or not checks or len({c["name"] for c in checks}) != len(checks):
        raise ValueError("missing/duplicate native checks")
    if any(type(c["exit_code"]) is not int or c["exit_code"] != 0 for c in checks):
        raise ValueError("failed native check")
    if any(type(report[t]["exit_code"]) is not int or report[t]["exit_code"] != 0 for t in ["rustc","cargo"]):
        raise ValueError("failed native tool probe")
    if report.get("source_changed_during_run") != []:
        raise ValueError("source changed during native verification")
    expected = {name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in execution_inputs(root)}
    if not expected or report.get("source_sha256") != expected:
        raise ValueError("stale/incomplete native source binding; rerun verification")
    for check in checks:
        log = check["log"]
        if not isinstance(log,str) or log != check["name"]+".log" or Path(log).name != log or "\\" in log:
            raise ValueError("invalid native log path")
        if hashlib.sha256((root/"docs/evidence"/log).read_bytes()).hexdigest() != check.get("log_sha256"):
            raise ValueError("stale native log: "+log)


def validate_coverage(coverage, root):
    if coverage.get("status") != "complete":
        raise ValueError("native execution coverage incomplete/failed")
    expected = {name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in execution_inputs(root)}
    if not expected or coverage.get("input_sha256") != expected:
        raise ValueError("stale/incomplete execution-coverage input binding")
    files=coverage["files"]
    by_file = {f["file"]: f for f in files}
    if not files or len(by_file) != len(files) or set(by_file) != set(coverage["source_sha256"]):
        raise ValueError("incomplete/duplicate execution-coverage paths; retain compiler instances explicitly")
    for path, digest in coverage["source_sha256"].items():
        if path not in expected or expected[path] != digest or not by_file[path].get("instances"):
            raise ValueError("stale/incomplete execution coverage: "+path)
    return by_file


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", action="store_true", help="fail if any applicable dimension is below 75")
    parser.add_argument("--report", action="store_true", help="save source hashes, scores and execution-coverage links")
    args = parser.parse_args()
    plan = json.loads((ROOT / "docs/quality-plan.json").read_text(encoding="utf-8"))
    actual = programs(ROOT)
    result = assess(plan, actual)
    for unit in plan["units"]:
        for path in unit["evidence"]:
            if not (ROOT / path).is_file():
                raise ValueError("missing evidence: " + path)
    if args.report:
        local = load_json(ROOT / "docs/evidence/local-validation.json")
        validate_native(local, ROOT)
        coverage = load_json(ROOT / "docs/evidence/quality/rust-execution-coverage.json")
        by_file = validate_coverage(coverage, ROOT)
        result.update({"schema": 1, "method": "docs/quality-evaluation.md", "units": plan["units"],
                       "plan_sha256": hashlib.sha256((ROOT / "docs/quality-plan.json").read_bytes()).hexdigest(),
                       "validation": "docs/evidence/local-validation.json",
                       "files": [{"file": p, "sha256": hashlib.sha256((ROOT / p).read_bytes()).hexdigest(),
                                  "units": [u["id"] for u in plan["units"] if p in u["files"]],
                                  "native_execution_coverage": by_file.get(p)} for p in sorted(actual)],
                       "limits": ["Scores are bounded review assessments, not benchmark results, percentages of upstream APIs, or mathematical safety proofs.",
                                  "Native coverage includes embedded unit tests. Unlisted files have no LLVM evidence; Python/Go/config coverage is separate.",
                                  "A passing inventory check is not a passing quality gate. No averaging and no unmeasured maximum-performance claim."]})
        (ROOT / "docs/evidence/quality/report.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{result['programs']} program files; {result['review_units']} review units; {len(result['below_high'])} assessments below 75; all_high={result['all_high']}")
    if args.gate and not result["all_high"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
