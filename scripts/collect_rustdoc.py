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
import stat
import subprocess
import sys
import tempfile
import time
from rustdoc_exports import EXTRACTOR_SHA256, exports

ROOT = Path(__file__).resolve().parents[1]
COLLECTOR_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


MARKER = ".matea-source-pin.json"


def file_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024*1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_snapshot(snapshot, provenance):
    """All tracked inputs, including assets and executable modes, are inputs."""
    hashes = provenance["source_hashes"]
    require(hashes, "empty source input manifest")
    directories = {"."}
    for name, expected in hashes.items():
        relative = Path(name)
        require(not relative.is_absolute() and not relative.drive and ".." not in relative.parts,
                "escaping snapshot manifest path")
        require(relative.as_posix() == name and name != MARKER, "noncanonical snapshot path")
        directories.update(parent.as_posix() for parent in relative.parents)
        path = snapshot / relative
        require(path.is_file() and not path.is_symlink(), "snapshot source changed: " + name)
        require(file_digest(path) == expected, "snapshot source changed: " + name)
        require(stat.S_IMODE(path.stat().st_mode) == provenance["source_modes"][name], "snapshot source mode changed: " + name)
    actual_files, actual_directories = set(), {"."}
    for folder, folders, files in os.walk(snapshot, followlinks=False):
        for name in folders + files:
            path = Path(folder) / name
            require(not path.is_symlink(), "unexpected snapshot symlink")
        actual_directories.update((Path(folder)/name).relative_to(snapshot).as_posix() for name in folders)
        actual_files.update((Path(folder)/name).relative_to(snapshot).as_posix() for name in files)
    require(actual_files == set(hashes) | {MARKER} and actual_directories == directories,
            "unexpected or missing snapshot files/directories")


def write_marker(marker, provenance):
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", prefix=".matea-pin-", dir=marker.parent.parent, delete=False) as output:
            temporary = Path(output.name)
            json.dump(provenance, output, indent=2, ensure_ascii=False)
            output.write("\n")
        os.replace(temporary, marker)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def prepare_snapshot(reference, snapshot, pin):
    reference, snapshot = reference.resolve(), snapshot.absolute()
    require(not snapshot.is_symlink(), "snapshot directory must not be a symlink")
    require(not snapshot.resolve().is_relative_to(reference), "snapshot must stay outside its reference checkout")
    require(command("git", "rev-parse", "HEAD", cwd=reference) == pin["sha"])
    require(not command("git", "status", "--porcelain", "--untracked-files=no", cwd=reference))
    files = subprocess.check_output(["git", "ls-files", "-z"], cwd=reference).split(b"\0")
    hashes, modes, symlinks, sources = {}, {}, {}, []
    for raw in files:
        if not raw:
            continue
        relative = Path(os.fsdecode(raw))
        require(not relative.is_absolute() and ".." not in relative.parts and relative.as_posix() != MARKER,
                "invalid/reserved tracked reference path")
        source = reference / relative
        if source.is_symlink():
            resolved = source.resolve()
            require(resolved.is_relative_to(reference), "reference symlink escapes its checkout: " + str(relative))
            symlinks[relative.as_posix()] = resolved.relative_to(reference).as_posix()
        require(source.is_file(), "review non-regular reference source: " + str(relative))
        name = relative.as_posix()
        hashes[name] = file_digest(source)
        modes[name] = stat.S_IMODE(source.stat().st_mode)
        sources.append((relative, source))
    provenance = {"schema": 2, "reference": pin["name"], "sha": pin["sha"], "source_hashes": hashes,
                  "source_modes": modes, "dereferenced_internal_symlinks": symlinks}
    marker = snapshot / MARKER
    if marker.exists():
        previous = json.loads(marker.read_text(encoding="utf-8"))
        require(previous["reference"] == pin["name"] and previous["sha"] == pin["sha"], "snapshot identity changed")
        schema = previous.get("schema")
        require(schema is None or type(schema) is int and schema in (1, 2), "unsupported snapshot manifest schema")
        # Validate against every input in the clean pinned checkout even for old
        # manifests. Never trust an old Rust-only subset to repair itself.
        validate_snapshot(snapshot, provenance)
        if previous.get("schema") != 2:
            write_marker(marker, provenance)
        else:
            require(previous == provenance, "snapshot provenance changed")
        return marker
    require(not snapshot.exists(), "destination snapshot already exists without provenance; choose an empty destination")
    snapshot.parent.mkdir(parents=True, exist_ok=True)
    # One writer per destination. Build in a sibling stage and publish only a
    # complete, checked directory; normal copy failures leave no partial cache.
    with tempfile.TemporaryDirectory(prefix=".matea-source-", dir=snapshot.parent) as temporary:
        stage = Path(temporary) / "snapshot"
        stage.mkdir()
        for relative, source in sources:
            target = stage / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        (stage / MARKER).write_text(json.dumps(provenance, indent=2, ensure_ascii=False)+"\n", encoding="utf-8")
        validate_snapshot(stage, provenance)
        require(not snapshot.exists(), "snapshot destination was created by another writer")
        os.rename(stage, snapshot)
    return marker


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
    snapshot = destination / "source" / args.reference
    marker = prepare_snapshot(reference, snapshot, pin)
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
