# Foundation performance evidence

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 1.88 / 1.95 | 28.90 / 42.15 | 0 / 1 |
| input/binding_f12 | 2.12 / 2.48 | 30.07 / 33.51 | 0 / 1 |
| input/binding_miss | 2.91 / 2.99 | 26.49 / 43.03 | 0 / 1 |
| input/binding_disabled | 0.73 / 0.75 | 29.36 / 42.57 | 0 / 1 |
| input/binding_repeat | 1.95 / 2.07 | 28.97 / 40.12 | 0 / 1 |
| input/binding_release | 0.92 / 0.94 | 2.07 / 2.07 | 0 / 0 |
| binding/value_hit | 2.50 / 2.79 | 6.95 / 7.48 | 0 / 0 |
| binding/value_miss | 2.19 / 2.19 | 6.85 / 7.50 | 0 / 0 |
| binding/value_disabled | 0.47 / 0.47 | 6.97 / 7.53 | 0 / 0 |
| binding/construct | 29.82 / 31.78 | 45.82 / 51.52 | 1 / 2 |
| binding/value_hit_owned | 5.82 / 6.11 | 7.11 / 7.26 | 0 / 0 |
| binding/metadata | 1.18 / 1.18 | 2.07 / 2.08 | 0 / 0 |
| motion/frame_duration | 0.68 / 0.82 | 2.12 / 2.22 | 0 / 0 |
| motion/projectile_update | 2.43 / 2.51 | 5.56 / 5.83 | 0 / 0 |
| motion/projectile_checked_update | 2.79 / 3.16 | 6.22 / 6.57 | 0 / 0 |
| motion/projectile_construct | 7.56 / 9.57 | 20.44 / 21.32 | 0 / 1 |
| pagination/range | 0.96 / 1.06 | 0.95 / 1.03 | 0 / 0 |
| pagination/navigation | 1.78 / 1.93 | 5.32 / 5.60 | 0 / 0 |
| pagination/arabic | 50.42 / 50.62 | 66.21 / 67.38 | 1 / 1 |
| pagination/dots_32 | 140.29 / 145.83 | 767.71 / 1132.38 | 1 / 31 |
| pagination/dots_256 | 915.62 / 977.67 | 13344.96 / 14005.33 | 1 / 255 |
| pagination/key_dispatch | 8.12 / 9.68 | 149.75 / 167.96 | 0 / 3 |
| textinput/left_right_ascii_64 | 37.45 / 38.94 | 2763.36 / 2810.24 | 0 / 13 |
| textinput/insert_end_ascii_64 | 69.87 / 74.21 | 2916.43 / 2945.57 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 78.80 / 83.40 | 2969.09 / 2991.60 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.98 / 124.09 | 2812.71 / 2853.95 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 94.89 / 99.84 | 3107.64 / 3148.50 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.63 / 4.64 | 2348.51 / 2383.77 | 0 / 1 |
| textinput/word_roundtrip_unicode | 156.30 / 165.42 | 2351.04 / 2383.69 | 0 / 16 |
| textinput/suggestions_edit_pair | 113.18 / 119.55 | 3082.77 / 3127.29 | 0 / 18 |
| cursor/visible_move_80x24 | 28.42 / 28.50 | 43110.96 / 43305.38 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 29230.84 / 30877.50 | 353622.50 / 402354.16 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 29587.50 / 30804.18 | 355277.50 / 380476.66 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 25018.34 / 26249.16 | 141508.32 / 259101.66 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23886.66 / 27317.50 | 123902.50 / 152502.50 | 0 / 349 |
| help/short_unbounded | 266.21 / 278.46 | 11344.17 / 11706.46 | 1 / 23 |
| help/short_40 | 233.67 / 259.88 | 8673.96 / 8887.04 | 1 / 18 |
| help/full_40 | 372.08 / 423.79 | 11847.83 / 12334.92 | 5 / 62 |
| help/short_unicode_80 | 482.04 / 508.17 | 6259.54 / 6411.33 | 1 / 12 |
| clocks/format_zero | 22.67 / 31.67 | 20.29 / 22.17 | 1 / 1 |
| clocks/reused_format_zero | 7.75 / 11.92 | 7.21 / 7.25 | 0 / 0 |
| clocks/format_nanoseconds | 33.12 / 65.00 | 15.21 / 16.12 | 1 / 1 |
| clocks/reused_format_nanoseconds | 14.96 / 21.04 | 10.29 / 10.42 | 0 / 0 |
| clocks/format_microseconds | 35.75 / 37.75 | 16.42 / 16.75 | 1 / 1 |
| clocks/reused_format_microseconds | 17.12 / 17.38 | 12.50 / 12.58 | 0 / 0 |
| clocks/format_seconds | 41.08 / 42.08 | 21.67 / 22.17 | 1 / 1 |
| clocks/reused_format_seconds | 24.21 / 25.67 | 16.25 / 16.33 | 0 / 0 |
| clocks/format_hours | 40.92 / 43.04 | 21.58 / 30.75 | 1 / 1 |
| clocks/reused_format_hours | 22.92 / 23.08 | 17.62 / 18.58 | 0 / 0 |
| clocks/format_maximum_go | 44.42 / 45.50 | 31.12 / 35.00 | 1 / 1 |
| clocks/reused_format_maximum_go | 32.58 / 33.58 | 26.58 / 29.25 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.27 | 6.06 / 7.02 | 0 / 0 |
| clocks/countdown_paused | 1.25 / 1.26 | 6.62 / 6.63 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 6.22 / 6.60 | 0 / 0 |
| clocks/countdown_foreign | 1.25 / 1.26 | 6.67 / 6.85 | 0 / 0 |
| spinner/plain_line | 2.19 / 2.20 | 649.44 / 674.87 | 0 / 0 |
| spinner/plain_dot | 2.19 / 2.35 | 648.18 / 670.33 | 0 / 0 |
| spinner/plain_mini_dot | 2.19 / 3.28 | 648.37 / 656.68 | 0 / 0 |
| spinner/plain_jump | 2.19 / 2.20 | 648.42 / 660.00 | 0 / 0 |
| spinner/plain_pulse | 2.19 / 2.27 | 649.03 / 668.98 | 0 / 0 |
| spinner/plain_points | 2.19 / 2.20 | 650.37 / 670.00 | 0 / 0 |
| spinner/plain_globe | 2.19 / 3.23 | 652.42 / 751.69 | 0 / 0 |
| spinner/plain_moon | 2.27 / 3.69 | 648.45 / 677.63 | 0 / 0 |
| spinner/plain_monkey | 2.27 / 2.38 | 647.61 / 667.98 | 0 / 0 |
| spinner/plain_meter | 2.30 / 2.37 | 647.52 / 663.28 | 0 / 0 |
| spinner/plain_hamburger | 2.19 / 3.10 | 647.21 / 658.38 | 0 / 0 |
| spinner/plain_ellipsis | 2.19 / 2.27 | 646.04 / 662.16 | 0 / 0 |
| spinner/foreign | 4.13 / 4.15 | 723.43 / 735.51 | 0 / 0 |
| spinner/stale | 4.20 / 14.71 | 732.89 / 1152.60 | 0 / 0 |
| render/empty_80x24 | 6227.25 / 6459.25 | 70828.38 / 72219.25 | 1 / 54 |
| render/ascii_80x24 | 12961.96 / 13753.21 | 109718.67 / 112342.88 | 1 / 54 |
| render/unicode_80x24 | 13622.88 / 13856.46 | 86519.29 / 88031.75 | 1 / 54 |
| unicode/width | 663.68 / 678.80 | 3489.72 / 3512.57 | 0 / 0 |
| terminal/unchanged_80x24 | 5.58 / 5.62 | 42980.50 / 43154.21 | 0 / 0 |
| terminal/one_cell_80x24 | 9710.67 / 9818.12 | 44564.79 / 44966.62 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.99 | 1.31 / 1.31 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.20 | 3.02 / 3.22 | 0 / 0 |
| motion/spring_construct_0.2 | 16.83 / 17.75 | 47.02 / 59.53 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.82 | 1.31 / 1.65 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.15 | 3.02 / 3.05 | 0 / 0 |
| motion/spring_construct_1 | 11.71 / 12.29 | 13.88 / 14.84 | 0 / 0 |
| motion/spring_update_2 | 0.82 / 0.82 | 1.31 / 1.35 | 0 / 0 |
| motion/spring_checked_update_2 | 2.11 / 2.19 | 3.02 / 3.07 | 0 / 0 |
| motion/spring_construct_2 | 17.02 / 18.20 | 42.67 / 44.76 | 0 / 0 |

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
