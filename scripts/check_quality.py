"""Check complete program inventory and report evidence-based ordinal assessments."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

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
        local = json.loads((ROOT / "docs/evidence/local-validation.json").read_text(encoding="utf-8"))
        if not local["all_local_checks_passed"]:
            raise ValueError("native verification failed; assessments cannot be published as validated")
        coverage = json.loads((ROOT / "docs/evidence/quality/rust-execution-coverage.json").read_text(encoding="utf-8"))
        for path, expected in coverage["source_sha256"].items():
            if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != expected:
                raise ValueError("stale execution coverage: " + path)
        by_file = {f["file"]: f for f in coverage["files"]}
        if len(by_file) != len(coverage["files"]):
            raise ValueError("duplicate execution-coverage paths; retain compiler instances explicitly")
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
