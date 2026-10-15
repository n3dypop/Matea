# Full reference coverage and performance gate

Owner revision: 2026-10-16. This replaces the earlier small Foundation candidate as the F5 entry criterion. F5 is not active. Owner clarification permits terminal image protocol encoders in Foundation; the image processing engine, image codecs and CUDA remain gated.

## Scope and interpretation

Audit all public functionality at the pinned revisions of Bubble Tea, Lip Gloss, Bubbles, Huh, Glamour, Harmonica, Ultraviolet, Ratatui and Crossterm, plus the pinned x/ansi and x/term modules. Dependency internals are investigated where they affect these contracts; this is not an unbounded requirement to reimplement the Go/Rust standard libraries. Platform/feature restrictions must be recorded. Rust source reuse/adapters count only where the complete observable contract is exposed and tested; a dependency or reexport alone is not evidence of integration.

The goal is equivalent user-visible capability, not identical Go syntax. Every public declaration must link to a capability entry. Generated Rust APIs, macros and cfg-dependent exports require explicit review; source declarations alone are not a complete resolved rustdoc export graph.

The eleven current inventories are the first pass, not a declaration that the
dependency boundary is complete. `coverage/dependency-frontier.json` records
manifest edges: colorprofile, x/termios, x/windows, cancelreader, terminfo,
Unicode/width/color helpers and other support libraries must be classified and
their public foundational capabilities included where applicable. Resolve test-only
dependencies, version differences and the Rust Cargo/platform/feature closure.
Keep the strict gate closed until this review is complete; do not silently narrow
the owner's instruction to the libraries already inventoried.

## Execution order

1. Mechanically inventory every production module/public declaration, benchmark, test skip and TODO/FIXME at the fixed source revisions. Preserve source paths, lines and SHAs, without vendoring the source into this repository.
2. Collect current upstream issues, separating reported defects, maintainer-confirmed behavior, intended semantics and testable local reproduction. Never assume an open report affects Matea without a regression test.
3. Map declarations to observable capability IDs, implementation paths, tests and performance cases. Initial status is missing/partial; advance only with evidence. A module with missing features cannot be labeled parity.
4. Run comparable Go/Rust release workloads with identical inputs/outputs, color/width policy and state transitions. Record compiler versions, source SHAs, data hashes, warm/cold distributions, allocation/output counts and limits. Incompatible semantics are not comparable and get no speed ratio.
5. Improve allocation/ownership, rendering, runtime scheduling, Unicode editing, reusable components and public extension design. Adopt existing Rust primitives where they make a stronger tested boundary.
6. Enter F5 only after the full ledger is implemented and module-level correctness/performance requirements are satisfied. Repeat three-OS/MSRV tests and request owner approval then.

## Performance decisions

Measure state transitions, rendering/layout, formatting, terminal encoding and real transport separately. A memory sink is not a terminal benchmark. Absolute p50/p95, throughput and output bytes remain useful even when comparison is not yet valid. Cold startup includes initialization; warm runs exclude compilation. Keep raw samples and schema-versioned machine-readable results. Performance results describe the measured host and dataset, not universal guarantees. Unmeasured modules remain pending and keep F5 closed.

## Encoder scope clarification

The owner explicitly included Kitty/Sixel and other referenced terminal image encoders before F5. They accept caller-supplied protocol data/pixels; processing, file decoding and CUDA are not authorized. Capability negotiation and session resource lifetime are Foundation transport contracts. This is a narrow exception to the older encoder gate, not approval for the image engine.
