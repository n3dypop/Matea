"""Create a pinned, exhaustive source-declaration/issue-marker inventory.

Source checkouts remain outside the repository. This is not a resolved Rust export
graph: macro expansions and cfg/reexports are explicit review work, never parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--references", type=Path, required=True)
parser.add_argument("--go-tool", type=Path, default=Path(tempfile.gettempdir()) / "matea-audit-go")
parser.add_argument("--rust-tool", type=Path, default=ROOT / "target/debug/matea-api-audit")
parser.add_argument("--destination", type=Path, default=ROOT / "docs/coverage")
args = parser.parse_args()
refs = args.references.resolve()
lock = json.loads((ROOT / "docs/upstream-lock.json").read_text())
out = args.destination.resolve()
out.mkdir(parents=True, exist_ok=True)
repositories = []
for entry in lock:
    name = entry["name"]
    if name.startswith("charm-x-"):
        module = "ansi" if name.endswith("ansi") else "term"
        cache = Path(tempfile.gettempdir()) / "matea-reference-audit" / name
        cache.mkdir(parents=True, exist_ok=True)
        paths = subprocess.check_output(["git", "-C", str(refs / "charm-x"), "ls-tree", "-r", "--name-only", entry["sha"], module], text=True).splitlines()
        for path in paths:
            if not path.endswith((".go", "go.mod", "go.sum")):
                continue
            relative = Path(path).relative_to(module)
            target = cache / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(subprocess.check_output(["git", "-C", str(refs / "charm-x"), "show", f"{entry['sha']}:{path}"]))
        source = cache
        prefix = module + "/"
    else:
        source = refs / name
        actual = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
        if actual != entry["sha"]:
            raise RuntimeError(f"{name}: expected {entry['sha']}, found {actual}")
        dirty = subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"], text=True)
        if dirty:
            raise RuntimeError(f"{name}: source checkout is dirty")
        prefix = ""
    language = "go" if (source / "go.mod").exists() else "rust"
    tool = args.go_tool if language == "go" else args.rust_tool
    result = subprocess.run([str(tool), str(source)], check=True, capture_output=True, text=True)
    declarations = []
    for line in result.stdout.splitlines():
        item = json.loads(line)
        item["file"] = prefix + item["file"]
        identity = f"{name}:{item['file']}:{item['kind']}:{item['symbol']}:{item['line']}"
        item["id"] = hashlib.sha256(identity.encode()).hexdigest()[:20]
        item["source_url"] = f"{entry['remote'].removesuffix('.git')}/blob/{entry['sha']}/{item['file']}#L{item['line']}"
        item["coverage"] = "unmapped"
        declarations.append(item)
    markers = []
    modules = {}
    for path in sorted(source.rglob("*")):
        if not path.is_file() or path.suffix not in {".go", ".rs"}:
            continue
        rel = path.relative_to(source)
        if any(part in {".git", "target", "vendor", "examples", "tutorials", "testdata", ".github", "node_modules", "xtask"} for part in rel.parts):
            continue
        file = prefix + rel.as_posix()
        text = path.read_text(encoding="utf-8")
        key = str(Path(file).parent).replace("\\", "/")
        module = modules.setdefault(key, {"path": key, "production_files": [], "test_files": [], "coverage": "pending", "performance": "unmeasured"})
        is_test = path.name.endswith("_test.go") or path.name.endswith("_test.rs") or "tests" in rel.parts or "benches" in rel.parts
        module["test_files" if is_test else "production_files"].append(file)
        for number, line in enumerate(text.splitlines(), 1):
            if re.search(r"\b(TODO|FIXME|XXX|HACK)\b|\bt\.Skip\(|#\[ignore", line):
                markers.append({"file": file, "line": number, "kind": "skipped_test" if "Skip(" in line or "#[ignore" in line else "issue_marker", "source_url": f"{entry['remote'].removesuffix('.git')}/blob/{entry['sha']}/{file}#L{number}"})
    record = {"schema": 1, "name": name, "sha": entry["sha"], "remote": entry["remote"], "language": language, "inventory_kind": "source_declarations_not_resolved_exports", "macro_cfg_review": "pending" if language == "rust" else "build_tags_recorded", "modules": list(modules.values()), "declarations": declarations, "issue_markers": markers}
    target = out / f"{name}.json"
    if target.is_file():
        previous = json.loads(target.read_text())
        if previous.get("sha") == entry["sha"]:
            assessed = {m["source_url"]: m for m in previous.get("issue_markers", [])}
            for marker in markers:
                old = assessed.get(marker["source_url"], {})
                for field in ("assessment", "analysis", "matea_evidence"):
                    if field in old:
                        marker[field] = old[field]
    target.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n")
    repositories.append({"name": name, "sha": entry["sha"], "modules": len(modules), "declarations": len(declarations), "issue_markers": len(markers), "coverage": "pending", "performance": "unmeasured"})
    print(f"{name}: {len(modules)} modules, {len(declarations)} declarations, {len(markers)} issue markers", flush=True)
(out / "summary.json").write_text(json.dumps({"schema": 1, "scope": "all pinned references including base libraries", "f5_eligible": False, "repositories": repositories}, indent=2) + "\n")
