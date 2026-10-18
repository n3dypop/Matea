# Terminal color gradient audit

The provisional Gradient API in matea-style owns RGB8 stops and caches CIELAB
coordinates once. `colors(count)` and indexed lookup borrow that configuration.
They preserve Lip Gloss's segmented distribution and small-count prefix shortcut
for the tested RGB8 domain. A grid owns one diagonal palette and cached rotation;
its borrowed traversals reuse that palette without materializing an output array.
There is no shared mutable cache or dependency on runtime/terminal internals.

```rust
use matea::prelude::*;
let gradient = Gradient::new(&[[90, 86, 224], [238, 111, 248]])
    .expect("nonempty RGB8 stops");
let foregrounds: Vec<Color> = gradient.colors(40).collect();
let field = gradient.grid(40, 8, 45.0).expect("finite angle and terminal dimensions");
let corner = field.color_at(0, 0);
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
tests also construct the 65,535-entry diagonal palette at maximum u16 dimensions,
check logical length and sample the last cell. They do not traverse or materialize
the whole logical grid; the maximum case is skipped in Miri.
The CLI's six fresh workloads match complete RGB8 output and FNV64 return values;
six Rust streaming controls are a separate workload, not an upstream speed ratio.
At 1202535 all 33 native checks pass: 167 Rust tests plus two doctests, 115 Python
tests, 15 snapshots and 20 Unix PTY cases. LLVM records 66 source files and 71
compiler instances. [Source/log-bound native cohort](evidence/gradients/native/local-validation.json).
The known Rust CSV/TSV fixtures and dashboard golden are now included in the
138-input snapshot, alongside all 116 programs and build/check manifests.

The input slice is copied and never mutated or returned as an alias. Empty stops
are rejected before reservation; dimensions of zero become one, and nonfinite
angles are errors. Exact-size, double-ended and fused iterators delegate range
arithmetic to Rust's standard Range; large indexed skips do not walk the range.
The two-dimensional diagonal palette is reserved fallibly and reused, avoiding
per-cell Lab-to-RGB conversion.

## Matched finite performance

Fresh workloads configure the same three RGB8 stops, materialize all normalized
RGB8 colors and consume every channel in a fixed checksum. The complete outputs
and first returns match. Both use a 20 ms target confirmed by three observations,
31 positive per-operation intervals and a separate allocation pass. Actual
adaptive counts may differ and remain recorded; the historical fixed-count
comparator is unchanged. Reference/Matea/Matea/reference order retains both pairs.
Percentiles summarize 31 per-operation batch averages, not individual-call tail
latency.

| Workload | Reference p50, two runs | Matea p50, two runs | Reference / Matea allocation counts |
| --- | --- | --- | --- |
| 1D, 8 colors | 3.467 / 3.503 µs | 0.435 / 0.436 µs | 11 / 2 |
| 1D, 80 colors | 33.830 / 33.916 µs | 4.479 / 4.509 µs | 83 / 2 |
| 1D, 1,024 colors | 423.199 / 418.708 µs | 57.643 / 57.468 µs | 1,027 / 2 |
| 16×8, 45° | 8.533 / 8.169 µs | 1.753 / 1.761 µs | 20 / 3 |
| 80×24, 0° | 52.989 / 54.013 µs | 17.828 / 17.967 µs | 84 / 3 |
| 80×24, 90° | 53.088 / 53.703 µs | 17.795 / 17.813 µs | 84 / 3 |

The six separate Rust reuse controls stream without materializing an output
vector; all request zero allocations. Their medians range from about 0.263 µs
for eight 1D colors to 53.2–53.5 µs for 1,024. A reused 80×24 palette traversal
takes about 7.57–7.64 µs. These have different work and receive no reference ratio.
The reference's 80×24/0° p95 reaches 99.604 µs in the reverse run; all tails remain
unfiltered. [All raw/context/distributions](evidence/gradients/performance/report.json).

Rust counters are requested System allocation/reallocation calls and Layout
bytes. Go uses AllocsPerRun(10) and average TotalAlloc deltas, with GC disabled
only for the separate byte-count pass. Allocator rounding/representations differ;
neither is RSS. Timing uses normal Go GC. [Metric review](evidence/gradients/performance-review.json).
No owned compilation, tests, Miri, coverage or downloads overlap timing. This is
one macOS arm64 host without affinity/frequency control. Single-stop/prefix and
error costs, other palettes and full styled View/render/transport performance
remain pending. The [three-OS/MSRV/reference CI](https://github.com/n3dypop/Matea/actions/runs/39109108150)
passes all seven required jobs. [Independent artifact binding](evidence/gradients/ci-source-bindings.json)
verifies all 116 program and 138 known input hashes, 17 uploaded logs per host,
548 exact reference fixtures, six matched fresh outputs and the complete 12/6
Rust/Go calibrated gradient cohorts. The existing 91-case cohorts also pass.
Unix drivers pass 33 checks with LLVM 66 files/71 instances; Windows passes
32 with 65/69. This does not verify every historical data dependency, physical
emulator or native Windows console. No quality score is promoted and Stage B/F5
remain closed.

This is **partial** reference coverage: arbitrary color.Color inputs, fractional
or high-depth channel composition, alpha conventions and empty/nil source
semantics remain pending. RGB8 output uses RGBA16 rounding followed by an 8-bit
projection, not RGB255 rounding. The two declarations are linked to this bounded
capability, with no whole-module or all-feature parity claim. The design and
comparison contract are in [ADR 0045](decisions/0045-owned-terminal-color-gradients.md).

An additive constructor now resolves canonical indexed/RGB stops without hex
parsing; all 256 indexed starting colors have separate eight-sample reference
fixtures. Default terminal colors remain unresolved errors. This new path is
under verification in [the progress slice](progress-audit.md); the timing and CI
results above remain tied to source 1202535, not silently reassigned to it.
