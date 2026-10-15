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
print(f"{len(inventories)} reference inventories; {len(declarations)} declaration candidates; "
      f"{unmapped} unlinked; {pending} partial/unmeasured capabilities; {unresolved_exports} unresolved Rust export graphs")
print("F5 CLOSED" if unmapped or pending or unresolved_exports else "Coverage entry checks satisfied; full F5 validation still required")
if args.gate and (unmapped or pending or unresolved_exports):
    raise SystemExit(1)
