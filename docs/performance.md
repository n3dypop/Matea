# Foundation performance evidence

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 1.78 / 1.85 | 28.45 / 39.99 | 0 / 1 |
| input/binding_f12 | 2.03 / 2.19 | 29.88 / 34.48 | 0 / 1 |
| input/binding_miss | 2.50 / 2.78 | 26.75 / 42.99 | 0 / 1 |
| input/binding_disabled | 0.70 / 0.72 | 29.54 / 41.32 | 0 / 1 |
| input/binding_repeat | 1.75 / 1.75 | 28.27 / 41.46 | 0 / 1 |
| input/binding_release | 0.86 / 0.86 | 2.03 / 2.49 | 0 / 0 |
| binding/value_hit | 2.50 / 2.83 | 6.83 / 7.02 | 0 / 0 |
| binding/value_miss | 2.19 / 2.31 | 6.72 / 6.81 | 0 / 0 |
| binding/value_disabled | 0.47 / 0.47 | 6.84 / 7.70 | 0 / 0 |
| binding/construct | 29.74 / 30.96 | 45.88 / 49.18 | 1 / 2 |
| binding/value_hit_owned | 6.03 / 8.01 | 6.95 / 8.09 | 0 / 0 |
| binding/metadata | 1.25 / 4.86 | 2.03 / 2.03 | 0 / 0 |
| motion/frame_duration | 0.71 / 0.77 | 2.08 / 2.22 | 0 / 0 |
| motion/projectile_update | 2.52 / 2.79 | 5.47 / 5.69 | 0 / 0 |
| motion/projectile_checked_update | 2.90 / 3.23 | 6.32 / 6.83 | 0 / 0 |
| motion/projectile_construct | 7.25 / 7.78 | 20.32 / 21.61 | 0 / 1 |
| pagination/range | 0.95 / 1.02 | 0.95 / 1.03 | 0 / 0 |
| pagination/navigation | 1.78 / 1.93 | 5.33 / 5.82 | 0 / 0 |
| pagination/arabic | 48.88 / 56.92 | 64.38 / 68.71 | 1 / 1 |
| pagination/dots_32 | 137.75 / 157.42 | 757.29 / 838.58 | 1 / 31 |
| pagination/dots_256 | 912.54 / 958.38 | 13595.25 / 17146.25 | 1 / 255 |
| pagination/key_dispatch | 8.12 / 9.75 | 148.21 / 168.74 | 0 / 3 |
| textinput/left_right_ascii_64 | 36.83 / 37.47 | 2757.62 / 2785.80 | 0 / 13 |
| textinput/insert_end_ascii_64 | 70.08 / 73.32 | 2889.89 / 2947.68 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 76.40 / 79.68 | 2934.07 / 2959.35 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 117.82 / 121.39 | 2795.36 / 2823.29 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 95.18 / 100.61 | 3093.82 / 3146.82 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.64 / 5.50 | 2343.97 / 2371.69 | 0 / 1 |
| textinput/word_roundtrip_unicode | 156.47 / 164.78 | 2338.84 / 2381.74 | 0 / 16 |
| textinput/suggestions_edit_pair | 113.87 / 118.50 | 3051.75 / 3147.75 | 0 / 18 |
| cursor/visible_move_80x24 | 28.46 / 28.54 | 42491.38 / 42741.33 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 29036.66 / 30585.00 | 348580.00 / 403322.50 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 29025.00 / 30295.84 | 351477.50 / 437810.84 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 25313.32 / 29693.34 | 135145.00 / 188272.50 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23530.00 / 25370.82 | 122262.50 / 127915.00 | 0 / 349 |
| help/short_unbounded | 271.33 / 285.50 | 11215.88 / 11533.75 | 1 / 23 |
| help/short_40 | 234.38 / 237.33 | 8460.54 / 8744.54 | 1 / 18 |
| help/full_40 | 372.00 / 412.88 | 11080.21 / 11551.96 | 5 / 62 |
| help/short_unicode_80 | 478.04 / 518.17 | 6118.79 / 6501.96 | 1 / 12 |
| clocks/format_zero | 22.62 / 23.67 | 10.29 / 10.79 | 1 / 1 |
| clocks/reused_format_zero | 7.25 / 7.29 | 7.21 / 7.29 | 0 / 0 |
| clocks/format_nanoseconds | 30.83 / 30.88 | 15.04 / 15.38 | 1 / 1 |
| clocks/reused_format_nanoseconds | 15.21 / 15.38 | 10.29 / 10.38 | 0 / 0 |
| clocks/format_microseconds | 36.00 / 36.46 | 16.38 / 17.92 | 1 / 1 |
| clocks/reused_format_microseconds | 17.17 / 17.33 | 12.50 / 12.58 | 0 / 0 |
| clocks/format_seconds | 41.46 / 43.67 | 34.83 / 40.79 | 1 / 1 |
| clocks/reused_format_seconds | 24.17 / 24.25 | 16.25 / 16.33 | 0 / 0 |
| clocks/format_hours | 39.83 / 40.96 | 21.62 / 22.12 | 1 / 1 |
| clocks/reused_format_hours | 23.29 / 23.50 | 17.62 / 31.00 | 0 / 0 |
| clocks/format_maximum_go | 47.46 / 47.83 | 31.38 / 32.25 | 1 / 1 |
| clocks/reused_format_maximum_go | 32.79 / 33.46 | 27.42 / 28.58 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.26 | 6.07 / 7.12 | 0 / 0 |
| clocks/countdown_paused | 1.25 / 1.26 | 6.50 / 6.51 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 6.12 / 6.15 | 0 / 0 |
| clocks/countdown_foreign | 1.25 / 1.26 | 6.54 / 6.55 | 0 / 0 |
| spinner/plain_line | 2.19 / 2.20 | 641.79 / 652.37 | 0 / 0 |
| spinner/plain_dot | 2.19 / 2.35 | 643.35 / 653.20 | 0 / 0 |
| spinner/plain_mini_dot | 2.27 / 2.35 | 639.02 / 653.08 | 0 / 0 |
| spinner/plain_jump | 2.35 / 4.49 | 638.19 / 651.91 | 0 / 0 |
| spinner/plain_pulse | 2.27 / 2.37 | 636.74 / 656.80 | 0 / 0 |
| spinner/plain_points | 2.19 / 2.28 | 640.75 / 651.25 | 0 / 0 |
| spinner/plain_globe | 2.19 / 2.30 | 636.48 / 648.96 | 0 / 0 |
| spinner/plain_moon | 2.19 / 2.20 | 639.00 / 673.45 | 0 / 0 |
| spinner/plain_monkey | 2.20 / 2.27 | 636.67 / 647.50 | 0 / 0 |
| spinner/plain_meter | 2.19 / 2.20 | 636.02 / 655.66 | 0 / 0 |
| spinner/plain_hamburger | 2.19 / 2.20 | 640.71 / 660.95 | 0 / 0 |
| spinner/plain_ellipsis | 2.19 / 2.20 | 637.96 / 655.53 | 0 / 0 |
| spinner/foreign | 4.06 / 4.07 | 884.25 / 891.49 | 0 / 0 |
| spinner/stale | 4.06 / 4.38 | 890.62 / 1275.78 | 0 / 0 |
| render/empty_80x24 | 6364.33 / 6814.12 | 71055.58 / 72580.04 | 1 / 54 |
| render/ascii_80x24 | 12787.62 / 12999.50 | 108524.58 / 111236.17 | 1 / 54 |
| render/unicode_80x24 | 13561.96 / 13791.71 | 86824.33 / 89299.92 | 1 / 54 |
| unicode/width | 674.25 / 684.65 | 3469.37 / 3563.88 | 0 / 0 |
| terminal/unchanged_80x24 | 5.58 / 5.62 | 42756.54 / 43034.71 | 0 / 0 |
| terminal/one_cell_80x24 | 9636.50 / 9763.04 | 44501.04 / 44853.12 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.11 | 3.02 / 3.05 | 0 / 0 |
| motion/spring_construct_0.2 | 16.83 / 16.92 | 41.89 / 60.03 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.84 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.15 | 3.02 / 3.03 | 0 / 0 |
| motion/spring_construct_1 | 11.70 / 12.24 | 13.84 / 14.42 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.31 / 1.32 | 0 / 0 |
| motion/spring_checked_update_2 | 2.11 / 2.11 | 2.97 / 3.02 | 0 / 0 |
| motion/spring_construct_2 | 17.00 / 17.25 | 42.09 / 45.68 | 0 / 0 |

All listed output fixtures match. ASCII transport is independently replayed for a→b→a.
Input cases use short batches and can show scheduling outliers; p50 improvements do not imply p95 or application-latency parity. No samples were discarded.
The stateful encoders each emit 2 bytes per one-cell update in this fixture. This does not establish bandwidth parity for other workloads.
The immutable unchanged-frame case uses pointer identity in Matea; equal newly allocated frames require comparison.
The native-cursor fixture also reuses unchanged cells. Four independently replayed states check cell contents, position and visibility; shape/color and terminal I/O are excluded.
Colored full-frame cases force repaint of twenty 80-cell ASCII rows on an 80x24 frame. Four output traces independently check glyphs, palette/RGB colors, background erasure and hidden cursor. RGB inputs deliberately choose matching profile outputs; they do not compare arbitrary nearest-color algorithms.
Explicit color profiles remain deterministic under NO_COLOR=1. A bounded eight-style cache removes repeated palette search without heap/global state. Retained intermediate samples include truecolor/none regressions, corrected by direct level fast paths; before/after distributions and frame bytes are in report.json.
Known-position horizontal cursor moves select shorter commands. The retained baseline records 6 bytes per move, compared with 4 after the change and 6 for pinned Ultraviolet. Vertical moves use absolute addressing: LF can scroll and CUU/CUD can clamp at existing margins. Column-zero absolute addresses omit the default column parameter. Matea's colored traces additionally replay with nondefault vertical margins; benchmark comparisons use default margins in both.
Spring tests cover 45 reference trajectories × 100 steps; the benchmark uses three finite damping regimes.
Projectile tests add 45 XYZ trajectories. Timed raw/checked updates and construction share the same finite fixture; checks remain enabled in both checked harnesses.
The motion/pagination baseline preserves source hashes and all samples before inlining, independent finite checks, invariant-based ranges and exact dot capacity reservation; see report.json.
Text-input fixtures exclude View construction, clipboard I/O, validators and virtual-cursor commands in both harnesses. They include eight event pairs; Unicode rune-vs-grapheme single-step behavior is not compared.
The text-input baseline retains samples/source hashes before ASCII query-buffer reuse. Unicode filtering keeps contextual lowercase semantics; that path is not covered by the ASCII allocation claim.
Key bindings add three custom-token matches plus construction and metadata access. Existing-string custom tokens do not format/allocate in either language. Constructors retain their result through black_box/global storage; Go escape analysis and Rust ownership differ. Short help storage is inline after the retained baseline.
Four help fixtures render fresh unstyled strings from existing metadata, excluding binding construction and structured-view/cell encoding in both. Ordinary outputs match byte-for-byte; narrow-width overflow and terminal-control safety deviations have separate regressions. Rich Lip Gloss style decorators and whole help-module coverage remain pending.
Clock fixtures cover six fresh compact duration strings, six caller-owned buffer reuse cases and four ignored-event guards. Go setup timers are drained outside timing. Go active Update arms a timer; Rust exposes a reusable schedule descriptor, so isolated active-update comparisons would be mismatched. End-to-end scheduling and full clock-module performance remain pending. Rust allocation bytes are requested sizes; Go includes rounding/tiny-object accounting.
Spinner fixtures cover twelve unstyled preset frame strings and two ignored-ID/generation guards. Both may share immutable frame storage; Rust exposes borrowed Cow output. Labels, rich styles, custom construction, active timer creation and notification latency remain outside this finite comparison.
Raw spring samples use IEEE arithmetic; checked-update cases validate inputs/results in both languages. Rust constructor validation is included. Very short timings are sensitive to harness effects.

## Reproduction

```sh
cargo build --release -p matea-perf-probe --locked
python3 scripts/prepare_upstream_bench.py --references ../references --destination /tmp/matea-foundation-upstream
# In the disposable Go module: go mod tidy; go build -o probe .
MATEA_PERF_ITERATIONS=1000 target/release/matea-perf-probe > artifacts/performance/matea-current.jsonl
MATEA_PERF_ITERATIONS=1000 /tmp/matea-foundation-upstream/probe > artifacts/performance/upstream-current.jsonl
target/release/matea-perf-probe --trace > artifacts/performance/matea-trace.json
/tmp/matea-foundation-upstream/probe --trace > artifacts/performance/ultraviolet-trace.json
target/release/matea-perf-probe --cursor-trace > artifacts/performance/matea-cursor-trace.json
/tmp/matea-foundation-upstream/probe --cursor-trace > artifacts/performance/ultraviolet-cursor-trace.json
NO_COLOR=1 target/release/matea-perf-probe --color-trace > artifacts/performance/matea-color-trace.json
/tmp/matea-foundation-upstream/probe --color-trace > artifacts/performance/ultraviolet-color-trace.json
python3 scripts/check_terminal_trace.py artifacts/performance/matea-trace.json artifacts/performance/ultraviolet-trace.json artifacts/performance/matea-cursor-trace.json artifacts/performance/ultraviolet-cursor-trace.json
python3 scripts/check_color_trace.py artifacts/performance/*-color-trace.json
python3 scripts/report_performance.py --artifacts artifacts/performance
```

Keep reference checkouts clean and at docs/upstream-lock.json revisions. The preparation script verifies pins.
No image processing, GPU or real terminal performance claim is made.

Additional [Rust-only subscription before/after costs](subscription-performance.md) measure adapter construction and retained timer tables, separately from the matched source fixtures.

The [clock-format follow-up](clock-format-performance.md) repeats the existing sixteen clock cases with longer batches and retains all short runs; these overlap the matched case set.

[Owned scene Text costs](view-performance.md) add twenty Rust-only construction/clone/complete-render comparisons, with early-normalization tradeoffs and retained regressions. These are separate from the matched source cases.
