"""Validate evidence links; --gate deliberately fails while any scope is pending."""
from evidence_checks import require
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_json(path):
    def unique_fields(pairs):
        value = {}
        for key, item in pairs:
            require(key not in value, "duplicate JSON field: " + key)
            value[key] = item
        return value
    def invalid_constant(value):
        raise ValueError("nonfinite JSON constant: " + value)
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_fields,
                      parse_constant=invalid_constant)


def evidence_path(root, name):
    require(isinstance(name, str) and name and "\\" not in name, "invalid evidence path")
    relative = Path(name)
    require(not relative.is_absolute() and not relative.drive and ".." not in relative.parts
            and relative.as_posix() == name, "escaping/noncanonical evidence path")
    path = (root / relative).resolve()
    require(path.is_relative_to(root.resolve()) and path.is_file(), "missing/external evidence: " + name)
    return path


def boolean(value):
    require(type(value) is bool, "closure flag must be boolean")
    return value


def issue_needs_review(root, issue, repository, pins):
    if issue["disposition"].startswith("untriaged"):
        return True
    review = issue["review"]
    require(review["status"] in ("complete", "partial"), "invalid issue review status")
    require(isinstance(review["notes"], str) and review["notes"].strip(), "missing issue assessment")
    pin = pins[review["reference"]]
    require(pin["remote"].removeprefix("https://github.com/").removesuffix(".git") == repository,
            "issue review repository mismatch")
    require(review["evidence_sha256"], "issue assessment needs evidence")
    stale = False
    for name, expected in review["evidence_sha256"].items():
        path = evidence_path(root, name)
        stale |= hashlib.sha256(path.read_bytes()).hexdigest() != expected
    return (review["status"] != "complete" or stale or review["reference_sha"] != pin["sha"]
            or review["reviewed_updated_at"] != issue["updated_at"])


def check(root=ROOT):
    ROOT = root
    inventories = [load_json(p) for p in (ROOT / "docs/coverage").glob("*.json")]
    inventories = [r for r in inventories if "declarations" in r]
    pins = {p["name"]: p for p in load_json(ROOT / "docs/upstream-lock.json")}
    current_pins = {name: p["sha"] for name, p in pins.items()}
    require(inventories and len({r["name"] for r in inventories}) == len(inventories), "missing/duplicate inventories")
    for inventory in inventories:
        require(current_pins[inventory["name"]] == inventory["sha"], "stale inventory pin")
    declarations = {d["id"]: d for r in inventories for d in r["declarations"]}
    require(len(declarations) == sum(len(r["declarations"]) for r in inventories))
    capabilities = load_json(ROOT / "docs/coverage/capabilities.json")["capabilities"]
    require(len({c["id"] for c in capabilities}) == len(capabilities), lambda: "duplicate capability IDs")
    linked = set()
    for capability in capabilities:
        require(capability["status"] in ("partial", "implemented", "owner_deferred"))
        require(capability["reference_ids"] and all(d in declarations for d in capability["reference_ids"]))
        linked.update(capability["reference_ids"])
        performance = capability["performance"]
        if performance not in ("pending", "owner_deferred"):
            evidence_path(ROOT, performance)
        if performance == "owner_deferred":
            require(capability["status"] == "owner_deferred", "unapproved capability deferral")
        if capability["status"] == "implemented":
            require(capability["implementation"] and capability["tests"], "implemented capability needs source and tests")
        for path in capability.get("implementation", []) + capability.get("tests", []):
            evidence_path(ROOT, path)
    unmapped = len(set(declarations) - linked)
    pending = sum(c["status"] == "partial" or c.get("performance") == "pending" for c in capabilities)
    unresolved_exports = sum(r["macro_cfg_review"] == "pending" for r in inventories)
    module_verdicts=load_json(ROOT/"docs/coverage/module-verdicts.json")["module_groups"]
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
        require(verdict["correctness"] and all(evidence_path(ROOT, p) for p in verdict["correctness"]))
        evidence_path(ROOT, verdict["source_audit"])
        report = load_json(evidence_path(ROOT, verdict["performance_report"]))
        cases = {c["case"]: c for c in report["cases"]}
        require(len(cases) == len(report["cases"]), "duplicate performance cases")
        require(all(type(c["output_equivalent_for_this_fixture"]) is bool for c in report["cases"]), "equivalence must be boolean")
        require(verdict["performance_cases"] and all(c in cases and cases[c]["output_equivalent_for_this_fixture"] for c in verdict["performance_cases"]), lambda: verdict)
        require(verdict["performance_verdict"] and verdict["notes"], lambda: verdict)
    issues=load_json(ROOT/"docs/coverage/upstream-issues.json")
    untriaged=sum(issue_needs_review(ROOT, i, r["repository"], pins) for r in issues["repositories"] for i in r["issues"])
    pending_markers=sum(m.get("assessment","pending")=="pending" for r in inventories for m in r["issue_markers"])
    scope_closed=boolean(load_json(ROOT/"docs/coverage/dependency-frontier.json")["scope_closure_reviewed"])
    compiled_index = ROOT / "docs/coverage/rustdoc/index.json"
    compiled_keys, compiled_linked = set(), set()
    compiled_groups = {}
    compiled_scope_closed = False
    if compiled_index.exists():
        compiler = load_json(compiled_index)
        current_pins = {p["name"]: p["sha"] for p in load_json(ROOT / "docs/upstream-lock.json")}
        compiled_scope_closed = boolean(compiler["target_feature_closure_reviewed"]) & boolean(compiler["signature_dependency_closure_reviewed"])
        identities = [(s["reference"], s["package"], s["profile"], s["target"]) for s in compiler["snapshots"]]
        require(len(set(identities)) == len(identities), "duplicate compiler profile")
        for snapshot in compiler["snapshots"]:
            require(current_pins[snapshot["reference"]] == snapshot["sha"])
            run = load_json(evidence_path(ROOT, snapshot["run"]))
            require(run["sha"] == snapshot["sha"] and run["target"] == snapshot["target"])
            if "graph" not in snapshot:
                compiled_scope_closed = False
                continue
            graph_path = evidence_path(ROOT, snapshot["graph"])
            require(hashlib.sha256(graph_path.read_bytes()).hexdigest() == snapshot["graph_sha256"])
            graph = load_json(graph_path)
            require(graph["sha"] == snapshot["sha"] and graph["extractor_sha256"] == run["extractor_sha256"])
            require(all(type(run[k]) is int and run[k] == 0 for k in ("exit_code", "rustdoc_exit_code", "dependency_tree_exit_code")))
            for key in ("reference", "sha", "package", "profile", "target"):
                require(snapshot[key] == run[key] == graph[key], "compiler profile identity mismatch")
            for collection, key in ((graph["items"], "profile_item_key"), (graph["trait_implementations"], "profile_impl_key")):
                require(len({node[key] for node in collection}) == len(collection), "duplicate compiler export key")
            prefix = f"{snapshot['reference']}:{snapshot['sha']}:{snapshot['package']}:"
            compiled_keys.update(prefix + i["profile_item_key"] for i in graph["items"])
            compiled_keys.update(prefix + i["profile_impl_key"] for i in graph["trait_implementations"])
            for node, key_name in [(i, "profile_item_key") for i in graph["items"]] + [(i, "profile_impl_key") for i in graph["trait_implementations"]]:
                source = node.get("source")
                if source and not source.get("outside_reference"):
                    group = (snapshot["reference"], str(Path(source["file"]).parent))
                    compiled_groups.setdefault(group, set()).add(prefix + node[key_name])
        for batch_entry in compiler.get("batches", []):
            batch = load_json(evidence_path(ROOT, batch_entry["path"]))
            require(current_pins[batch["reference"]] == batch["sha"])
            if batch["exclusions"] or not boolean(batch["all_requested_profiles_passed"]):
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
    return bool(blocked)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gate", action="store_true")
    args = parser.parse_args()
    blocked = check()
    return 1 if args.gate and blocked else 0


if __name__ == "__main__":
    raise SystemExit(main())
