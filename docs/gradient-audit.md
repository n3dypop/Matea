# Terminal color gradient audit

The provisional Gradient API in matea-style owns RGB8 stops and caches CIELAB
coordinates once. `colors(count)` and indexed lookup borrow that configuration.
They preserve Lip Gloss's segmented distribution and small-count prefix shortcut
for the tested RGB8 domain. A grid owns one diagonal palette and cached rotation;
its borrowed traversals reuse that palette without materializing an output array.
There is no shared mutable cache or dependency on runtime/terminal internals.

```rust
use matea::prelude::*;
let gradient = Gradient::new(&[[90, 86, 224], [238, 111, 248]])?;
let foregrounds: Vec<Color> = gradient.colors(40).collect();
let field = gradient.grid(40, 8, 45.0)?;
let corner = field.color_at(0, 0);
# Ok::<(), GradientError>(())
```

`cargo run -p matea --example gradients` exercises theme tokens, a half-block
stripe, a rotated field, focus, resize and terminal color fallback. Arrows rotate
the field, Tab toggles focus and q/Esc exits. Authors emit no escape sequences or
restoration commands. Snapshot and Unix PTY checks pass locally; native Windows
console and physical emulator behavior remain unverified.

Lip Gloss 19f011f1 blending.go and checksum-pinned go-colorful v1.4.1 colors.go
are the audited references. The MIT attribution is retained in
THIRD_PARTY_NOTICES.md. [Source hashes and finite output checks](evidence/gradients/reference-source.json)
bind 548 generated fixture cases to the reference. All match natively, and
19 selected cases plus iterator/ownership tests pass Miri at seed 197. Native
tests also cover maximum u16 grid dimensions; those are skipped in Miri.
The CLI's six fresh workloads match complete RGB8 output and FNV64 return values;
six Rust streaming controls are a separate workload, not an upstream speed ratio.
Performance distributions, full acceptance and three-OS verification are pending.

The input slice is copied and never mutated or returned as an alias. Empty stops
are rejected before reservation; dimensions of zero become one, and nonfinite
angles are errors. Exact-size, double-ended and fused iterators delegate range
arithmetic to Rust's standard Range; large indexed skips do not walk the range.
The two-dimensional diagonal palette is reserved fallibly and reused, avoiding
per-cell Lab-to-RGB conversion. No measured speed improvement is claimed yet.

This is **partial** reference coverage: arbitrary color.Color inputs, fractional
or high-depth channel composition, alpha conventions and empty/nil source
semantics remain pending. RGB8 output uses RGBA16 rounding followed by an 8-bit
projection, not RGB255 rounding. The two declarations are linked to this bounded
capability, with no whole-module or all-feature parity claim. The design and
comparison contract are in [ADR 0045](decisions/0045-owned-terminal-color-gradients.md).
