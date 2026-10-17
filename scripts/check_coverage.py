"""Validate evidence links; --gate deliberately fails while any scope is pending."""
from evidence_checks import require
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--gate", action="store_true")
args = parser.parse_args()
inventories = [json.loads(p.read_text()) for p in (ROOT / "docs/coverage").glob("*.json")]
inventories = [r for r in inventories if "declarations" in r]
declarations = {d["id"]: d for r in inventories for d in r["declarations"]}
require(len(declarations) == sum(len(r["declarations"]) for r in inventories))
capabilities = json.loads((ROOT / "docs/coverage/capabilities.json").read_text())["capabilities"]
require(len({c["id"] for c in capabilities}) == len(capabilities), lambda: "duplicate capability IDs")
linked = set()
for capability in capabilities:
    require(capability["status"] in ("partial", "implemented", "owner_deferred"))
    require(capability["reference_ids"] and all(d in declarations for d in capability["reference_ids"]))
    linked.update(capability["reference_ids"])
    for path in capability.get("implementation", []) + capability.get("tests", []):
        require((ROOT / path).is_file(), lambda: path)
unmapped = len(set(declarations) - linked)
pending = sum(c["status"] == "partial" or c.get("performance") == "pending" for c in capabilities)
unresolved_exports = sum(r["macro_cfg_review"] == "pending" for r in inventories)
module_verdicts=json.loads((ROOT/"docs/coverage/module-verdicts.json").read_text())["module_groups"]
expected_groups={(r["name"],m["path"]) for r in inventories for m in r["modules"]}
require(len(module_verdicts)==len(expected_groups))
require({(v["reference"],v["source_group"]) for v in module_verdicts}==expected_groups)
pending_modules=sum(v["status"]=="pending" or not v["correctness"] or not v["performance_cases"] for v in module_verdicts)
for verdict in module_verdicts:
    require(verdict["status"] in ("pending", "implemented_measured"), lambda: verdict)
    if verdict["status"] == "pending":
        continue
    inventory = next(r for r in inventories if r["name"] == verdict["reference"])
    ids = {d["id"] for d in inventory["declarations"] if str(Path(d["file"]).parent) == verdict["source_group"]}
    require(ids and ids <= linked, lambda: verdict)
    owning = [c for c in capabilities if ids.intersection(c["reference_ids"])]
    require(all(c["status"] in ("implemented", "owner_deferred") and c.get("performance") != "pending" for c in owning), lambda: verdict)
    require(verdict["declarations_reviewed"] == len(ids), lambda: verdict)
    require(verdict["correctness"] and all((ROOT / p).is_file() for p in verdict["correctness"]))
    require((ROOT / verdict["source_audit"]).is_file())
    report = json.loads((ROOT / verdict["performance_report"]).read_text())
    cases = {c["case"]: c for c in report["cases"]}
    require(verdict["performance_cases"] and all(c in cases and cases[c]["output_equivalent_for_this_fixture"] for c in verdict["performance_cases"]), lambda: verdict)
    require(verdict["performance_verdict"] and verdict["notes"], lambda: verdict)
issues=json.loads((ROOT/"docs/coverage/upstream-issues.json").read_text())
untriaged=sum(i["disposition"].startswith("untriaged") for r in issues["repositories"] for i in r["issues"])
pending_markers=sum(m.get("assessment","pending")=="pending" for r in inventories for m in r["issue_markers"])
scope_closed=json.loads((ROOT/"docs/coverage/dependency-frontier.json").read_text())["scope_closure_reviewed"]
compiled_index = ROOT / "docs/coverage/rustdoc/index.json"
compiled_keys, compiled_linked = set(), set()
compiled_groups = {}
compiled_scope_closed = False
if compiled_index.exists():
    import hashlib
    compiler = json.loads(compiled_index.read_text(encoding="utf-8"))
    current_pins = {p["name"]: p["sha"] for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8"))}
    compiled_scope_closed = compiler["target_feature_closure_reviewed"] and compiler["signature_dependency_closure_reviewed"]
    for snapshot in compiler["snapshots"]:
        require(current_pins[snapshot["reference"]] == snapshot["sha"])
        run = json.loads((ROOT / snapshot["run"]).read_text(encoding="utf-8"))
        require(run["sha"] == snapshot["sha"] and run["target"] == snapshot["target"])
        if "graph" not in snapshot:
            compiled_scope_closed = False
            continue
        graph_path = ROOT / snapshot["graph"]
        require(hashlib.sha256(graph_path.read_bytes()).hexdigest() == snapshot["graph_sha256"])
        graph = json.loads(graph_path.read_text(encoding="utf-8"))
        require(graph["sha"] == snapshot["sha"] and graph["extractor_sha256"] == run["extractor_sha256"])
        require(run["exit_code"] == run["rustdoc_exit_code"] == run["dependency_tree_exit_code"] == 0)
        prefix = f"{snapshot['reference']}:{snapshot['sha']}:{snapshot['package']}:"
        compiled_keys.update(prefix + i["profile_item_key"] for i in graph["items"])
        compiled_keys.update(prefix + i["profile_impl_key"] for i in graph["trait_implementations"])
        for node, key_name in [(i, "profile_item_key") for i in graph["items"]] + [(i, "profile_impl_key") for i in graph["trait_implementations"]]:
            source = node.get("source")
            if source and not source.get("outside_reference"):
                group = (snapshot["reference"], str(Path(source["file"]).parent))
                compiled_groups.setdefault(group, set()).add(prefix + node[key_name])
    for batch_entry in compiler.get("batches", []):
        batch = json.loads((ROOT / batch_entry["path"]).read_text(encoding="utf-8"))
        require(current_pins[batch["reference"]] == batch["sha"])
        if batch["exclusions"] or not batch["all_requested_profiles_passed"]:
            compiled_scope_closed = False
    for capability in capabilities:
        keys = set(capability.get("compiler_export_keys", []))
        require(keys <= compiled_keys, lambda: capability["id"])
        compiled_linked.update(keys)
    for verdict in module_verdicts:
        if verdict["status"] != "implemented_measured":
            continue
        inventory = next(r for r in inventories if r["name"] == verdict["reference"])
        if inventory["language"] == "rust":
            require(verdict.get("cfg_export_reviewed"), lambda: verdict)
            require(compiled_groups.get((verdict["reference"], verdict["source_group"]), set()) <= compiled_linked, lambda: verdict)
compiled_unmapped = len(compiled_keys - compiled_linked)
print(f"{len(inventories)} reference inventories; {len(declarations)} declaration candidates; "
      f"{unmapped} unlinked; {pending} partial/unmeasured capabilities; {unresolved_exports} unresolved Rust export graphs")
print(f"{pending_modules} module verdicts pending; {untriaged} issue reports and {pending_markers} source markers need ledger disposition")
print(f"Dependency/public-capability closure reviewed: {scope_closed}")
print(f"{compiled_unmapped} compiler profile export candidates unlinked (overlap source inventory); target/signature closure reviewed: {compiled_scope_closed}")
blocked=unmapped or pending or unresolved_exports or pending_modules or untriaged or pending_markers or not scope_closed or compiled_unmapped or not compiled_scope_closed
print("F5 CLOSED" if blocked else "Coverage entry checks satisfied; full F5 validation still required")
if args.gate and blocked:
    raise SystemExit(1)
