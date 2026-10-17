"""Check complete program inventory and report evidence-based ordinal assessments."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIMENSIONS = {"rust_design", "memory_safety", "behavior", "reference", "performance", "portability"}


def programs(root):
    return {p.relative_to(root).as_posix() for folder in ("crates", "tools", "scripts", "examples", ".github")
            for p in (root / folder).rglob("*") if p.is_file() and p.suffix in {".rs", ".go", ".py", ".yml", ".yaml"}}


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
            if value["score"] not in (None, 0, 25, 50, 75, 100) or not value["reason"]:
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
