"""Plan and publish pinned source inventories, not resolved export graphs.

Validate all inputs before publication; replace individual files atomically and
publish a hash-bound summary last. Single writer; readers verify the summary to
detect an interrupted multi-file publication. This is not a crash transaction.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile
from urllib.parse import quote
from check_coverage import load_json, parse_json
from evidence_checks import require

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED = {".git", "target", "vendor", "examples", "tutorials", "testdata", ".github", "node_modules", "xtask"}
MARKER = re.compile(r"\b(TODO|FIXME|XXX|HACK)\b|\bt\.Skip\(|#\[ignore")


def serialized(value):
    return (json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def git(source, *arguments):
    return subprocess.check_output(["git", "-C", str(source), *arguments], timeout=60)


def verify_checkout(source, pin):
    require(git(source, "rev-parse", "HEAD").decode().strip() == pin["sha"], pin["name"] + ": source pin mismatch")
    require(not git(source, "status", "--porcelain"), pin["name"] + ": dirty source checkout")


def relative_file(value):
    require(isinstance(value, str) and value and "\\" not in value and not re.match(r"^[A-Za-z]:", value), "invalid source file")
    path = PurePosixPath(value)
    require(not path.is_absolute() and ".." not in path.parts and path.as_posix() == value, "noncanonical source file")
    return path


def source_url(pin, file, line):
    return f"{pin['remote'].removesuffix('.git')}/blob/{pin['sha']}/{quote(file, safe='/')}#L{line}"


def run_auditor(tool, source):
    return subprocess.run([str(tool), str(source)], check=True, capture_output=True,
                          text=True, encoding="utf-8", timeout=120).stdout


def declarations_from(output, source, pin, prefix):
    result, identities, validated = [], set(), set()
    source = source.resolve()
    for line in output.splitlines():
        item = parse_json(line)
        require(isinstance(item, dict), "invalid declaration record")
        file = relative_file(item["file"]).as_posix()
        if file not in validated:
            path = (source / file).resolve()
            require(path.is_relative_to(source) and path.is_file(), "missing/external source declaration")
            validated.add(file)
        require(type(item["line"]) is int and item["line"] > 0, "invalid declaration line")
        require(all(isinstance(item[field], str) and item[field] for field in ["kind", "symbol"]), "invalid declaration identity")
        item["file"] = prefix + file
        identity = f"{pin['name']}:{item['file']}:{item['kind']}:{item['symbol']}:{item['line']}"
        item["id"] = hashlib.sha256(identity.encode()).hexdigest()[:20]
        require(item["id"] not in identities, "duplicate declaration identity")
        identities.add(item["id"])
        item["source_url"] = source_url(pin, item["file"], item["line"])
        item["coverage"] = "unmapped"
        result.append(item)
    return result


def inventory(source, pin, prefix, go_tool, rust_tool, previous, auditor):
    source = source.resolve()
    language = "go" if (source / "go.mod").is_file() else "rust"
    declarations = declarations_from(auditor(go_tool if language == "go" else rust_tool, source), source, pin, prefix)
    modules, markers = {}, []
    for path in sorted(source.rglob("*")):
        if not path.is_file() or path.suffix not in {".go", ".rs"}:
            continue
        rel = path.relative_to(source)
        if EXCLUDED.intersection(rel.parts):
            continue
        require(path.resolve().is_relative_to(source), "external source marker file")
        file = prefix + rel.as_posix()
        key = PurePosixPath(file).parent.as_posix()
        module = modules.setdefault(key, {"path": key, "production_files": [], "test_files": [], "coverage": "pending", "performance": "unmeasured"})
        is_test = path.name.endswith(("_test.go", "_test.rs")) or bool({"tests", "benches"}.intersection(rel.parts))
        module["test_files" if is_test else "production_files"].append(file)
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if MARKER.search(line):
                markers.append({"file": file, "line": number,
                    "kind": "skipped_test" if "Skip(" in line or "#[ignore" in line else "issue_marker",
                    "source_url": source_url(pin, file, number)})
    if previous.get("sha") == pin["sha"]:
        old_markers = previous.get("issue_markers", [])
        assessed = {m["source_url"]: m for m in old_markers}
        require(len(assessed) == len(old_markers), "duplicate prior marker identity")
        for marker in markers:
            old = assessed.get(marker["source_url"], {})
            for field in ("assessment", "analysis", "matea_evidence"):
                if field in old:
                    marker[field] = old[field]
    return {"schema": 1, "name": pin["name"], "sha": pin["sha"], "remote": pin["remote"], "language": language,
            "inventory_kind": "source_declarations_not_resolved_exports",
            "macro_cfg_review": "pending" if language == "rust" else "build_tags_recorded",
            "modules": list(modules.values()), "declarations": declarations, "issue_markers": markers}


def materialize_module(references, pin, destination):
    module = "ansi" if pin["name"] == "charm-x-ansi" else "term"
    source = references / "charm-x"
    paths = git(source, "ls-tree", "-r", "-z", "--name-only", pin["sha"], module).split(b"\0")
    require(any(paths), "empty pinned x module")
    for raw in filter(None, paths):
        file = relative_file(raw.decode("utf-8"))
        if not file.name.endswith((".go", "go.mod", "go.sum")):
            continue
        target = destination / file.relative_to(module).as_posix()
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(git(source, "show", f"{pin['sha']}:{file}"))
    return module + "/"


def plan(references, pins, destination, go_tool, rust_tool, *, auditor=None):
    auditor = auditor or run_auditor
    references, destination = references.resolve(), destination.resolve()
    require(references.is_dir() and not destination.is_relative_to(references), "inventory destination overlaps source checkouts")
    require(isinstance(pins, list) and pins, "missing source pins")
    require(len({p["name"] for p in pins}) == len(pins), "duplicate source pins")
    for pin in pins:
        require(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]*", pin["name"]) and re.fullmatch(r"[0-9a-f]{40}", pin["sha"]), "invalid source pin")
        require(isinstance(pin["remote"], str) and pin["remote"].startswith("https://"), "invalid source remote")
    files, repositories = {}, []
    with tempfile.TemporaryDirectory(prefix="matea-source-audit-") as temporary:
        for pin in pins:
            name = pin["name"]
            target = destination / (name + ".json")
            previous = load_json(target) if target.exists() else {}
            require(isinstance(previous, dict), "invalid prior inventory")
            if name in {"charm-x-ansi", "charm-x-term"}:
                source = Path(temporary) / name
                source.mkdir()
                prefix = materialize_module(references, pin, source)
            else:
                source = references / name
                verify_checkout(source, pin)
                prefix = ""
            record = inventory(source, pin, prefix, go_tool, rust_tool, previous, auditor)
            if not prefix:
                verify_checkout(source, pin)
            files[name + ".json"] = serialized(record)
            repositories.append({"name": name, "sha": pin["sha"], "modules": len(record["modules"]),
                "declarations": len(record["declarations"]), "issue_markers": len(record["issue_markers"]), "coverage": "pending", "performance": "unmeasured"})
    summary = {"schema": 2, "scope": "all pinned references including base libraries", "f5_eligible": False,
               "repositories": repositories, "inventory_sha256": {name: hashlib.sha256(raw).hexdigest() for name, raw in files.items()}}
    files["summary.json"] = serialized(summary)
    return files


def publish(destination, files):
    require(files and list(files)[-1] == "summary.json", "summary must publish last")
    destination.mkdir(parents=True, exist_ok=True)
    for name, raw in files.items():
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=destination, delete=False) as stream:
                temporary = Path(stream.name)
                stream.write(raw)
            os.replace(temporary, destination / name)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--go-tool", type=Path, default=Path(tempfile.gettempdir()) / "matea-audit-go")
    parser.add_argument("--rust-tool", type=Path, default=ROOT / "target/debug/matea-api-audit")
    parser.add_argument("--destination", type=Path, default=ROOT / "docs/coverage")
    args = parser.parse_args()
    files = plan(args.references, load_json(ROOT / "docs/upstream-lock.json"), args.destination, args.go_tool, args.rust_tool)
    publish(args.destination, files)
    for repo in json.loads(files["summary.json"])["repositories"]:
        print(f"{repo['name']}: {repo['modules']} modules, {repo['declarations']} declarations, {repo['issue_markers']} issue markers")


if __name__ == "__main__":
    main()
