"""Validate evidence links; --gate deliberately fails while any scope is pending."""
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
assert len(declarations) == sum(len(r["declarations"]) for r in inventories)
capabilities = json.loads((ROOT / "docs/coverage/capabilities.json").read_text())["capabilities"]
linked = set()
for capability in capabilities:
    assert capability["status"] in ("partial", "implemented", "owner_deferred")
    assert capability["reference_ids"] and all(d in declarations for d in capability["reference_ids"])
    linked.update(capability["reference_ids"])
    for path in capability.get("implementation", []) + capability.get("tests", []):
        assert (ROOT / path).is_file(), path
unmapped = len(set(declarations) - linked)
pending = sum(c["status"] == "partial" or c.get("performance") == "pending" for c in capabilities)
unresolved_exports = sum(r["macro_cfg_review"] == "pending" for r in inventories)
module_verdicts=json.loads((ROOT/"docs/coverage/module-verdicts.json").read_text())["module_groups"]
expected_groups={(r["name"],m["path"]) for r in inventories for m in r["modules"]}
assert len(module_verdicts)==len(expected_groups)
assert {(v["reference"],v["source_group"]) for v in module_verdicts}==expected_groups
pending_modules=sum(v["status"]=="pending" or not v["correctness"] or not v["performance_cases"] for v in module_verdicts)
issues=json.loads((ROOT/"docs/coverage/upstream-issues.json").read_text())
untriaged=sum(i["disposition"].startswith("untriaged") for r in issues["repositories"] for i in r["issues"])
pending_markers=sum(m.get("assessment","pending")=="pending" for r in inventories for m in r["issue_markers"])
scope_closed=json.loads((ROOT/"docs/coverage/dependency-frontier.json").read_text())["scope_closure_reviewed"]
print(f"{len(inventories)} reference inventories; {len(declarations)} declaration candidates; "
      f"{unmapped} unlinked; {pending} partial/unmeasured capabilities; {unresolved_exports} unresolved Rust export graphs")
print(f"{pending_modules} module verdicts pending; {untriaged} issue reports and {pending_markers} source markers need ledger disposition")
print(f"Dependency/public-capability closure reviewed: {scope_closed}")
blocked=unmapped or pending or unresolved_exports or pending_modules or untriaged or pending_markers or not scope_closed
print("F5 CLOSED" if blocked else "Coverage entry checks satisfied; full F5 validation still required")
if args.gate and blocked:
    raise SystemExit(1)
