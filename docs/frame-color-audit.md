# Deterministic frame color and quantization audit

Status: four finite styled-frame fixtures reviewed; complete style/terminal
capabilities and module-level performance remain pending.

## Correctness finding

With `NO_COLOR=1`, FrameEncoder's explicit ANSI16/ANSI256 capabilities previously
lost their colors: the reused Crossterm formatter emitted empty SGR commands.
[The failing observation](evidence/color-policy-before.json) records source hashes,
the released dependency SHA and observed sequences. This is a Matea integration
defect: Crossterm's global output policy is intentional. Capability selection now
owns environment policy; the pure writer encodes closed Color values without
consulting or mutating global state. Default environment detection still respects
NO_COLOR. CI starts a fresh process with NO_COLOR set and replays all four explicit
profiles. [The independent checker](../scripts/check_color_trace.py) validates
glyphs, foreground/background, default blank rows and hidden cursor after both
initial output and recovery.

## Source review and Rust design

The runtime uses released Crossterm 0.29, VCS SHA
`36d95b26a26e64b0f8c12edfe11f410a6d56a812`, distinct from the fixed upstream audit
reference. Its `src/style/types/colored.rs:75` reads cached global color policy;
`src/style/types/attribute.rs:177` constructs a String for attribute SGR. The
Matea serializer now uses direct checked value formatting and fixed byte literals.
Differential tests compare all eight supported attribute combinations with the
dependency formatter. No public arbitrary escape-string API was introduced.

Pinned Ultraviolet `6b8d4baf91b4c933e6613b0eefc8f180f6eda002`,
`terminal_renderer.go:597–620` and `cell.go:434`, adapts pen colors through
colorprofile. Its resolved `colorprofile v0.4.3/profile.go` uses a process-global
map keyed by profile and comparable color interface values, protected by a mutex,
without eviction. Matea retains at most eight source/adapted Style pairs per
exclusive FrameEncoder. Closed Eq colors, inline storage and exclusive mutable
ownership avoid dynamic comparability, global locks and growing cache storage.
Eviction preserves correctness; no cache capacity performance guarantee is made.

The logical cache survives external-output recovery because nominal palette
mapping is pure. Physical pen/cursor/visibility never survive invalidation. Level
changes invalidate cached mappings; a future configurable palette needs a policy
generation in the key. Unchanged ASCII cells are skipped before adaptation.
None/TrueColor paths bypass palette lookup entirely.

## Matched measurement

Both public writers repaint an existing 80×24 surface containing twenty uniform
80-cell ASCII rows, recover unknown cursor/style/visibility, preserve warmed
logical color caches and use reused memory sinks. The Go probe resizes its renderer
and explicitly resets pen/visibility before Redraw. Selected RGB colors produce
identical profile outputs; arbitrary nearest-color distance metrics are excluded.

| Profile | Matea before p50 / p95 ns | Matea after p50 / p95 ns | allocations before / after | frame bytes M / U after |
| --- | ---: | ---: | ---: | ---: |
| ANSI16 | 90,303.32 / 93,142.5 | 28,615 / 30,347.5 | 2 / 0 | 1,669 / 1,875 |
| ANSI256 | 1,575,277.5 / 1,581,835 | 28,472.5 / 31,150 | 2 / 0 | 1,680 / 2,095 |
| TrueColor | 29,291.68 / 30,719.16 | 24,580.82 / 25,429.18 | 2 / 0 | 1,695 / 2,395 |
| None | 27,266.66 / 28,645.84 | 23,352.5 / 25,080.84 | 1 / 0 | 1,655 / 1,655 |

The baseline already has the corrected color serializer. The change combines
bounded memoization, inline direct-level adaptation, fixed attributes and shorter
row/home commands; it does not attribute every gain to caching. All 31 samples,
source hashes, intermediate regressions and byte counts for this increment are
retained under `color_frame_optimization` in
[color-final-report.json](evidence/performance/2026-10-16/color-final-report.json).
Its current Rust raw file is preserved as `matea-color-final.jsonl`, with the same
digest recorded for `matea-current.jsonl` in that report. Later measurements are
published separately as the evolving report.json.
The first cache version regressed TrueColor/None p50 about 8/9 percent; the direct
paths corrected that measured regression. No samples were discarded.

CRLF first cancels delayed wrap, then advances to a valid next row. Bottom-row
same-row moves use CR; unknown positions use absolute addressing. Independent
replay rejects scrolling below the viewport. Shortened home addressing also saves
three bytes in the existing ASCII and cursor traces; update/move bytes stay 2/4.

Remaining: adversarial cache distributions, custom terminal palettes, all style
attributes, arbitrary color adaptation equivalence, real terminal I/O and native
Windows console validation. These results are one macOS arm64 host and do not
complete any whole terminal/style module. No image engine or CUDA is involved.
