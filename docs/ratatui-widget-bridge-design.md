# Ratatui widget bridge: next Foundation capability increment

Initial adapter and owned scenes are implemented under ADRs 0060/0061. The
[bounded audit and measurements](ratatui-bridge-audit.md) supersede the candidate
status below. Complete raw/module/profile parity and the F5 gate remain pending.
ADR 0004 requires reevaluation before a broad widget catalog; ADR 0016 explicitly
rejects bare reexports or nested terminal sessions as integration evidence.

The current 49 below-high axes remain because complete reference breadth and
module evidence are missing. Further nanosecond tuning of an already bounded
primitive cannot resolve that breadth. After qualifying viewport highlights,
prioritize a managed, executable Rust widget/buffer bridge and evaluate reuse of
the pinned widget implementations before independently recreating each catalog.
Keep every raw/compiler declaration linked to an actual callable capability,
configuration, correctness test and performance disposition. Dependency reuse
is valid evidence only within the tested actual graph/profile; it is not automatic
closure of the reference inventory.

The pinned Ratatui facade is 0.30.2 at
d1ef782b48ddf0cb825c111d4d8b9adef572c0e1. Its default features include all-widgets
(calendar), Crossterm, layout cache, macros and underline color. The isolated
reference-text caller currently exercises only ratatui-core/std, with its own
frozen 43-package lock context. A production integration must use the exact Git
revision, record the actual resolved graph, and compare transitive-version changes
with that context. Do not replace it with a registry package sharing a version.
Alternative backends, serde/palette, no_std and unstable traits remain separate
profile obligations. New dependencies/features require Linux/Windows/macOS and
MSRV 1.88 qualification; the prior text-only proof does not cover them.

An initial candidate interface renders a borrowed/owned upstream Widget into an
owned Buffer, validates that buffer, then returns an immutable Matea scene
snapshot. StatefulWidget conversion needs a separate operation borrowing &mut
state only during rendering. No upstream widget/state borrow may escape into the
returned View. The runtime must retain its existing terminal lease, panic cleanup,
cancellation and output batching. The adapter must not create a second Terminal.
Evaluate how a stateful render cache fits the existing read-only Model::view
contract before claiming effortless model integration; document or improve that
contract rather than concealing required RefCell/lock/cached-update patterns.

A compact Arc-backed surface/snapshot View node is a candidate for avoiding a
scene node per cell while preserving View's existing 48-byte budget. The surface
must compose under clipping/layers/panels, own its cells, preserve styles, survive
mutation/drop of the upstream Buffer, and produce deterministic old-frame diffs.
This node is also useful for independent non-Ratatui extensions. Benchmark full
widget-to-frame and frame-to-terminal paths, with complete output checks, against
direct pinned rendering. Include per-frame allocations, snapshots, resizing,
scroll/selection state, wide-to-narrow damage and invalid geometry. Do not label
existing narrow row-diff measurements as evidence for this new boundary.

The import policy must be explicit and tested. Ratatui's public Cell accepts raw
arbitrary symbol strings and CellDiffOption::{None,Skip,AlwaysUpdate,ForcedWidth},
plus deprecated skip. A blind cell copy can inject controls, lose forced-width
or skip semantics, or let row-clearing erase externally reserved resources.
Every imported cell must be translated faithfully or return a typed error before
publishing a partial scene. Silent sanitization is not exact Buffer equivalence.
Rejecting unsupported cells is a bounded bridge contract, not implementation of
all such source capabilities. Track remaining contracts separately. Ordinary
terminal-default/named/indexed/RGB colors, all modifiers and conditional underline
colors also need explicit mappings and frame-byte/capability fallback tests.

Next decisions require source-backed experiments for: buffer origin/size handling;
continuation ownership; zero-width/multi-EGC/control symbols; forced/skip/always
updates; stateful render lifetime/error semantics; actual dependency graph;
optional API paths; public facade ergonomics; and retained/native/cross-platform
performance. Record the selected contract in an ADR only after these experiments.
No image engine, decoding, conversion or CUDA work is authorized by this bridge.
