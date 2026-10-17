"""Import normalized API facts/logs, never raw reference source or rustdoc docs."""
from evidence_checks import require
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DESTINATION = ROOT / "docs/coverage/rustdoc"


def canonical_fact_bytes(path):
    text = Path(path).read_text(encoding="utf-8")
    if Path(path).suffix == ".json":
        text = json.dumps(json.loads(text), indent=2, ensure_ascii=False) + "\n"
    return text.encode("utf-8")


def plan_import(folders, destination, pins):
    """Validate the entire input set before allowing any destination mutation."""
    index_path = destination / "index.json"
    index = json.loads(index_path.read_text(encoding="utf-8")) if index_path.exists() else {
        "schema": 1, "target_feature_closure_reviewed": False,
        "signature_dependency_closure_reviewed": False, "snapshots": [],
        "limits": ["Compiler profiles supplement, overlap and expand the source declaration inventory; do not sum the two counts",
                   "Initial matrix is not every feature combination/backend/no_std/architecture",
                   "Aliases and inferred types are graph edges, not recursively flattened import paths",
                   "Reachability, source mapping, capability implementation and performance are separate reviews"]}
    require(index["schema"] == 1, "unsupported import index schema")
    snapshots = {s["stem"]: s for s in index["snapshots"]}
    batches = {s["file"]: s for s in index.get("batches", [])}
    require(len(snapshots) == len(index["snapshots"]), "duplicate indexed snapshot")
    require(len(batches) == len(index.get("batches", [])), "duplicate indexed batch")
    pending = {}

    def stage(path):
        value = canonical_fact_bytes(path)
        if path.name in pending:
            require(pending[path.name] == value, "conflicting input file: " + path.name)
        pending[path.name] = value
        return value

    runs = 0
    for folder in folders:
        require(folder.is_dir(), "missing input directory: " + str(folder))
        for batch_path in sorted(folder.rglob("*-batch.json")):
            batch = json.loads(batch_path.read_text(encoding="utf-8"))
            require(pins[batch["reference"]] == batch["sha"])
            stage(batch_path)
            batches[batch_path.name] = {"file": batch_path.name, "path": f"docs/coverage/rustdoc/{batch_path.name}",
                                        "reference": batch["reference"], "sha": batch["sha"], "target": batch["target"]}
        for run_path in sorted(folder.rglob("*-run.json")):
            runs += 1
            report = json.loads(run_path.read_text(encoding="utf-8"))
            require(pins[report["reference"]] == report["sha"], lambda: run_path)
            stem = run_path.name.removesuffix("-run.json")
            entry = {"stem": stem, "reference": report["reference"], "sha": report["sha"],
                     "package": report["package"], "profile": report["profile"], "target": report["target"],
                     "status": report["status"], "run": f"docs/coverage/rustdoc/{run_path.name}"}
            graph_path = run_path.with_name(f"{stem}-exports.json")
            if report["status"] == "compiled_profile_only_not_full_scope":
                graph = json.loads(graph_path.read_text(encoding="utf-8"))
                require(graph["sha"] == report["sha"] and graph["reference"] == report["reference"])
                require(graph["extractor_sha256"] == report["extractor_sha256"])
                require(report["exit_code"] == report["rustdoc_exit_code"] == report["dependency_tree_exit_code"] == 0)
                require(all(graph[k] == report[k] for k in ("package", "profile", "target")), "profile identity mismatch")
                require(all("profile_item_key" in i and "docs" not in i for i in graph["items"]))
                keys = [i["profile_item_key"] for i in graph["items"]]
                require(len(keys) == len(set(keys)), "duplicate compiler item key")
                graph_bytes = stage(graph_path)
                entry.update({"graph": f"docs/coverage/rustdoc/{graph_path.name}",
                              "graph_sha256": hashlib.sha256(graph_bytes).hexdigest(),
                              "local_items": len(graph["items"]), "external_exports": len(graph["external_exports"])})
            for suffix in ("-run.json", ".log", "-dependencies.txt"):
                source = run_path.with_name(stem + suffix)
                if source.exists():
                    stage(source)
            snapshots[stem] = entry
    require(runs > 0, "no profile reports found")
    index["snapshots"] = sorted(snapshots.values(), key=lambda s: s["stem"])
    index["batches"] = sorted(batches.values(), key=lambda b: b["file"])
    pending["index.json"] = (json.dumps(index, indent=2) + "\n").encode("utf-8")
    return pending, index


def import_reports(folders, destination, pins):
    pending, index = plan_import(folders, destination, pins)
    destination.mkdir(parents=True, exist_ok=True)
    # Each replacement is atomic; publish the index last. This is not a
    # multi-file transaction on OS failure or concurrent independent importers.
    with tempfile.TemporaryDirectory(prefix=".matea-import-", dir=destination.parent) as temporary:
        stage = Path(temporary)
        for name, value in pending.items():
            (stage / name).write_bytes(value)
        for name in pending:
            os.replace(stage / name, destination / name)
    return index


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reports", type=Path, action="append", required=True)
    args = parser.parse_args()
    pins = {p["name"]: p["sha"] for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8"))}
    index = import_reports(args.reports, DESTINATION, pins)
    print(f"Imported {len(index['snapshots'])} profile snapshots; full target/feature closure remains unreviewed")


if __name__ == "__main__":
    main()
