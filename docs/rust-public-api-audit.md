# Compiler-resolved Rust reference API audit

Status: partial evidence. The source inventory's two unresolved Rust export graphs
and the revised F5 gate remain open. Compiler snapshots overlap the source
declaration candidates; their counts must not be added together.

The collector copies only tracked files from verified clean, fixed-SHA reference
checkouts into an external temporary directory. Cargo uses the reference's tracked
lockfile and explicit native target. The code is unchanged; internal license
symlinks are dereferenced with provenance, and outside-checkout links are rejected.
Sources, raw rustdoc JSON and source documentation stay outside Matea. Imported
graphs contain API facts/headers, not implementation bodies.

The extractor checks rustdoc schema 61 and records its own hash. It follows public
modules, aliases, globs, inherent/trait members and signature-reachable public
types; unrelated private declarations are filtered. Hidden/private raw items are
needed to retain callable methods of inferred public return types. Representative
paths are marked when they are not directly nameable. Synthetic/external blanket
impls are summarized; local blanket impls retain their own review keys. External
exports and types in signatures form a dependency frontier, not proof of closure.
Tests cover nested alias/glob visibility, private fields/methods, default trait
members, inferred return types, compiler-ID renumbering and schema changes. A tiny
actual compiler experiment also reproduced the inferred-type omission.

Seven initial macOS profiles compile successfully, including the facade's all
features. The normalized ledger contains 5,699 distinct item/impl keys before any
capability mapping. This is additional overlapping review evidence, not 5,699
newly implemented functions. Standard generic blankets are identified separately;
local generic implementations remain in review.

```sh
python3 scripts/test_rustdoc_exports.py
python3 scripts/collect_rustdoc_batch.py --references ../references \
  --destination /tmp/matea-rustdoc --reference crossterm
python3 scripts/collect_rustdoc_batch.py --references ../references \
  --destination /tmp/matea-rustdoc --reference ratatui
python3 scripts/import_rustdoc.py --reports /tmp/matea-rustdoc/reports
python3 scripts/check_coverage.py --gate
```

Use the recorded compiler (current local nightly: rustc 1.101.0-nightly,
6f5da406b, 2026-10-10). The optional reference_api workflow input runs an initial
three-OS profile matrix with nightly-2026-10-11 and retains normalized artifacts.
Normal Foundation CI runs the extractor's tests without fetching/building the
references. A failed profile/schema is unresolved evidence; no stale prior success
is retained as the new run's result. Only import files from completed profiles
whose source pins and extractor hashes match their run records.

Next work includes reviewing all platform/feature/backend alternatives, resolving
external and signature dependencies, matching source declarations/macros to
compiled items, and linking those items/impls to implementation/correctness/performance.
No capability is implemented merely because the upstream compiler can see it.

The renderer reevaluation found nontrivial forced-width/skip/always-update cell
contracts. An optional bridge requires style, stateful-render and sanitization
decisions before adoption; see [ADR 0016](decisions/0016-compiler-api-coverage-before-renderer-integration.md).
The published ratatui-core 0.1.2 source SHA differs from the pinned reference;
version labels alone are not interchangeability evidence.
