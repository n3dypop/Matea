"""Compile pinned references in temporary copies, keeping sources out of Matea."""
from evidence_checks import require
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
from rustdoc_exports import EXTRACTOR_SHA256, exports

ROOT = Path(__file__).resolve().parents[1]
COLLECTOR_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--reference", choices=["crossterm", "ratatui"], required=True)
    parser.add_argument("--package", required=True)
    parser.add_argument("--profile", choices=["minimal", "default", "all"], default="default")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--toolchain", default="nightly")
    args = parser.parse_args()
    destination = args.destination.resolve()
    if destination.is_relative_to(ROOT):
        parser.error("reference source snapshots must stay outside the Matea repository")
    pin = next(p for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8"))
               if p["name"] == args.reference)
    reference = (args.references / args.reference).resolve()
    require(command("git", "rev-parse", "HEAD", cwd=reference) == pin["sha"])
    require(not command("git", "status", "--porcelain", "--untracked-files=no", cwd=reference))
    snapshot = destination / "source" / args.reference
    marker = snapshot / ".matea-source-pin.json"
    if not marker.exists():
        if snapshot.exists():
            raise ValueError("destination snapshot already exists without provenance; choose an empty destination")
        files = subprocess.check_output(["git", "ls-files", "-z"], cwd=reference).split(b"\0")
        hashes = {}
        symlinks = {}
        for raw in files:
            if not raw:
                continue
            relative = Path(os.fsdecode(raw))
            source = reference / relative
            if source.is_symlink():
                resolved = source.resolve()
                if not resolved.is_relative_to(reference):
                    raise ValueError(f"reference symlink escapes its checkout: {relative}")
                symlinks[relative.as_posix()] = resolved.relative_to(reference).as_posix()
            if not source.is_file():
                raise ValueError(f"review non-regular reference source: {relative}")
            target = snapshot / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            if target.suffix in (".rs", ".toml") or target.name == "Cargo.lock":
                hashes[relative.as_posix()] = hashlib.sha256(target.read_bytes()).hexdigest()
        marker.write_text(json.dumps({"reference": args.reference, "sha": pin["sha"],
                                     "source_hashes": hashes, "dereferenced_internal_symlinks": symlinks}, indent=2) + "\n", encoding="utf-8")
    provenance = json.loads(marker.read_text(encoding="utf-8"))
    require(provenance["sha"] == pin["sha"])
    require(all(hashlib.sha256((snapshot / name).read_bytes()).hexdigest() == value
               for name, value in provenance["source_hashes"].items()), lambda: "snapshot source changed")
    target = destination / "target" / args.reference / args.profile
    reports = destination / "reports"
    reports.mkdir(parents=True, exist_ok=True)
    rustc = command("rustc", f"+{args.toolchain}", "-vV")
    host_target = next(line.removeprefix("host: ") for line in rustc.splitlines() if line.startswith("host: "))
    stem = f"{args.reference}-{args.package}-{args.profile}-{host_target}"
    invocation = ["cargo", f"+{args.toolchain}", "rustdoc", "--locked", "--manifest-path",
                  str(snapshot / "Cargo.toml"), "--package", args.package, "--target", host_target, "--lib"]
    if args.profile == "minimal":
        invocation.append("--no-default-features")
    elif args.profile == "all":
        invocation.append("--all-features")
    if args.offline:
        invocation.append("--offline")
    invocation += ["--", "-Z", "unstable-options", "--output-format", "json", "--document-hidden-items", "--document-private-items"]
    report = {"schema": 1, "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
              "reference": args.reference, "sha": pin["sha"], "package": args.package,
              "profile": args.profile, "host": platform.platform(), "rustc": rustc,
              "target": host_target,
              "scope": "one native platform/profile, not full macro/cfg closure",
              "command": [value.replace(str(destination), "${AUDIT_TEMP}") for value in invocation],
              "cargo": command("cargo", f"+{args.toolchain}", "--version"),
              "source_manifest_sha256": hashlib.sha256(marker.read_bytes()).hexdigest()}
    report["extractor_sha256"] = EXTRACTOR_SHA256
    report["collector_sha256"] = COLLECTOR_SHA256
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), RUSTFLAGS="", RUSTDOCFLAGS="")
    started = time.monotonic()
    result = subprocess.run(invocation, env=env, text=True, capture_output=True)
    report["elapsed_seconds"] = round(time.monotonic() - started, 3)
    log = (result.stdout + result.stderr).replace(str(destination), "${AUDIT_TEMP}").replace(str(Path.home()), "${HOME}")
    (reports / f"{stem}.log").write_text(log, encoding="utf-8")
    report["rustdoc_exit_code"] = result.returncode
    report["exit_code"] = result.returncode
    report["status"] = "compile_failed_profile_unresolved" if result.returncode else "compiled_export_unresolved"
    (reports / f"{stem}-run.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    raw = target / host_target / "doc" / (args.package.replace("-", "_") + ".json")
    if result.returncode == 0:
        data = raw.read_bytes()
        try:
            graph = exports(json.loads(data), snapshot)
        except (ValueError, KeyError, TypeError) as error:
            report["exit_code"] = 1
            report["export_error"] = f"{type(error).__name__}: {error}"
            (reports / f"{stem}-run.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            print(report["export_error"], flush=True)
            return 1
        graph["raw_rustdoc_sha256"] = hashlib.sha256(data).hexdigest()
        graph["extractor_sha256"] = report["extractor_sha256"]
        graph.update({"reference": args.reference, "sha": pin["sha"], "package": args.package,
                      "profile": args.profile, "host": platform.platform(), "target": host_target, "rustc": rustc})
        (reports / f"{stem}-exports.json").write_text(json.dumps(graph, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        report.update({"status": "compiled_profile_only_not_full_scope", "local_items": len(graph["items"]),
                       "external_exports": len(graph["external_exports"]), "rustdoc_format": graph["rustdoc_format"]})
        tree_command = invocation[:invocation.index("--lib")]
        tree_command[2] = "tree"
        tree_command += ["--edges", "normal,build,features", "--charset", "ascii"]
        if args.profile == "minimal":
            tree_command.append("--no-default-features")
        elif args.profile == "all":
            tree_command.append("--all-features")
        if args.offline:
            tree_command.append("--offline")
        tree = subprocess.run(tree_command, env=env, text=True, capture_output=True)
        report["dependency_tree_exit_code"] = tree.returncode
        tree_text = (tree.stdout + tree.stderr).replace(str(destination), "${AUDIT_TEMP}").replace(str(Path.home()), "${HOME}")
        (reports / f"{stem}-dependencies.txt").write_text(tree_text, encoding="utf-8")
        if tree.returncode:
            report["status"] = "compiled_dependency_graph_unresolved"
            report["exit_code"] = tree.returncode
    (reports / f"{stem}-run.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"{stem}: {report['status']}", flush=True)
    return report["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
