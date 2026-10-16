"""Import normalized API facts/logs, never raw reference source or rustdoc docs."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
DESTINATION = ROOT / "docs/coverage/rustdoc"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reports", type=Path, action="append", required=True)
    args = parser.parse_args()
    DESTINATION.mkdir(parents=True, exist_ok=True)
    index_path = DESTINATION / "index.json"
    index = json.loads(index_path.read_text(encoding="utf-8")) if index_path.exists() else {
        "schema": 1, "target_feature_closure_reviewed": False,
        "signature_dependency_closure_reviewed": False, "snapshots": [],
        "limits": ["Compiler profiles supplement, overlap and expand the source declaration inventory; do not sum the two counts",
                   "Initial matrix is not every feature combination/backend/no_std/architecture",
                   "Aliases and inferred types are graph edges, not recursively flattened import paths",
                   "Reachability, source mapping, capability implementation and performance are separate reviews"]}
    snapshots = {s["stem"]: s for s in index["snapshots"]}
    batches = {s["file"]: s for s in index.get("batches", [])}
    pins = {p["name"]: p["sha"] for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8"))}
    for folder in args.reports:
        for batch_path in sorted(folder.rglob("*-batch.json")):
            batch = json.loads(batch_path.read_text(encoding="utf-8"))
            assert pins[batch["reference"]] == batch["sha"]
            shutil.copyfile(batch_path, DESTINATION / batch_path.name)
            batches[batch_path.name] = {"file": batch_path.name, "path": f"docs/coverage/rustdoc/{batch_path.name}",
                                        "reference": batch["reference"], "sha": batch["sha"], "target": batch["target"]}
        for run_path in sorted(folder.rglob("*-run.json")):
            report = json.loads(run_path.read_text(encoding="utf-8"))
            assert pins[report["reference"]] == report["sha"], run_path
            stem = run_path.name.removesuffix("-run.json")
            entry = {"stem": stem, "reference": report["reference"], "sha": report["sha"],
                     "package": report["package"], "profile": report["profile"], "target": report["target"],
                     "status": report["status"], "run": f"docs/coverage/rustdoc/{run_path.name}"}
            graph_path = run_path.with_name(f"{stem}-exports.json")
            if report["status"] == "compiled_profile_only_not_full_scope":
                graph = json.loads(graph_path.read_text(encoding="utf-8"))
                assert graph["sha"] == report["sha"] and graph["reference"] == report["reference"]
                assert graph["extractor_sha256"] == report["extractor_sha256"]
                assert report["rustdoc_exit_code"] == report["dependency_tree_exit_code"] == 0
                assert all("profile_item_key" in i and "docs" not in i for i in graph["items"])
                shutil.copyfile(graph_path, DESTINATION / graph_path.name)
                entry.update({"graph": f"docs/coverage/rustdoc/{graph_path.name}",
                              "graph_sha256": hashlib.sha256(graph_path.read_bytes()).hexdigest(),
                              "local_items": len(graph["items"]), "external_exports": len(graph["external_exports"])})
            for suffix in ("-run.json", ".log", "-dependencies.txt"):
                source = run_path.with_name(stem + suffix)
                if source.exists():
                    shutil.copyfile(source, DESTINATION / source.name)
            snapshots[stem] = entry
    index["snapshots"] = sorted(snapshots.values(), key=lambda s: s["stem"])
    index["batches"] = sorted(batches.values(), key=lambda b: b["file"])
    index_path.write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")
    print(f"Imported {len(index['snapshots'])} profile snapshots; full target/feature closure remains unreviewed")


if __name__ == "__main__":
    main()
