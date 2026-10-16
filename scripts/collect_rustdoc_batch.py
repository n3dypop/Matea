"""A bounded initial OS/profile matrix, deliberately not full feature closure."""
import argparse
import json
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", choices=["crossterm", "ratatui"], required=True)
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--toolchain", default="nightly")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    rustc = subprocess.check_output(["rustc", f"+{args.toolchain}", "-vV"], text=True)
    host_target = next(line.removeprefix("host: ") for line in rustc.splitlines() if line.startswith("host: "))
    pin = next(p for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8")) if p["name"] == args.reference)
    matrix = [("crossterm", p) for p in ["minimal", "default", "all"]] if args.reference == "crossterm" else [
        ("ratatui-core", "all"), ("ratatui-widgets", "all"), ("ratatui", "default")]
    exclusions = []
    if args.reference == "ratatui":
        if sys.platform != "win32":
            matrix.append(("ratatui", "all"))
        else:
            exclusions.append({"package": "ratatui", "profile": "all", "status": "platform_review_pending",
                               "reason": "all enables the Unix Termion backend; individual Windows-supported backend/feature profiles still need review, not blanket exclusion"})
    results = []
    for package, profile in matrix:
        invocation = [sys.executable, str(ROOT / "scripts/collect_rustdoc.py"),
                      "--references", str(args.references), "--destination", str(args.destination),
                      "--reference", args.reference, "--package", package, "--profile", profile,
                      "--toolchain", args.toolchain]
        if args.offline:
            invocation.append("--offline")
        result = subprocess.run(invocation)
        results.append({"package": package, "profile": profile, "exit_code": result.returncode})
    reports = args.destination / "reports"
    reports.mkdir(parents=True, exist_ok=True)
    summary = {"schema": 1, "reference": args.reference, "host": platform.platform(),
               "sha": pin["sha"], "target": host_target,
               "scope": "initial native profiles only; alternative backends, no_std and other feature combinations still require review",
               "results": results, "exclusions": exclusions,
               "all_requested_profiles_passed": all(r["exit_code"] == 0 for r in results),
               "target_feature_closure_reviewed": False}
    (reports / f"{args.reference}-{host_target}-batch.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return 0 if summary["all_requested_profiles_passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
