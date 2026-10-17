# Foundation performance evidence

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 3.89 / 4.50 | 28.68 / 39.95 | 0 / 1 |
| input/binding_f12 | 4.34 / 4.58 | 29.78 / 33.98 | 0 / 1 |
| input/binding_miss | 5.34 / 5.41 | 26.21 / 44.38 | 0 / 1 |
| input/binding_disabled | 1.54 / 1.55 | 29.15 / 41.05 | 0 / 1 |
| input/binding_repeat | 3.75 / 3.77 | 28.26 / 41.09 | 0 / 1 |
| input/binding_release | 1.60 / 1.89 | 2.04 / 2.06 | 0 / 0 |
| binding/value_hit | 4.63 / 4.67 | 6.82 / 7.50 | 0 / 0 |
| binding/value_miss | 4.05 / 4.13 | 6.73 / 6.93 | 0 / 0 |
| binding/value_disabled | 0.77 / 0.88 | 6.84 / 7.01 | 0 / 0 |
| binding/construct | 39.58 / 48.48 | 44.42 / 51.82 | 1 / 2 |
| binding/value_hit_owned | 6.50 / 6.51 | 6.91 / 7.07 | 0 / 0 |
| binding/metadata | 1.24 / 1.25 | 2.03 / 2.04 | 0 / 0 |
| motion/frame_duration | 0.71 / 0.71 | 2.08 / 2.26 | 0 / 0 |
| motion/projectile_update | 2.48 / 2.58 | 5.46 / 5.71 | 0 / 0 |
| motion/projectile_checked_update | 2.73 / 2.87 | 6.31 / 12.22 | 0 / 0 |
| motion/projectile_construct | 7.10 / 8.04 | 20.43 / 21.54 | 0 / 1 |
| pagination/range | 0.96 / 1.00 | 0.94 / 0.99 | 0 / 0 |
| pagination/navigation | 1.88 / 1.93 | 5.29 / 5.52 | 0 / 0 |
| pagination/arabic | 49.83 / 50.04 | 64.25 / 65.08 | 1 / 1 |
| pagination/dots_32 | 140.92 / 142.71 | 756.46 / 936.50 | 1 / 31 |
| pagination/dots_256 | 907.67 / 942.00 | 13269.38 / 13799.88 | 1 / 255 |
| pagination/key_dispatch | 9.28 / 9.68 | 148.99 / 167.74 | 0 / 3 |
| textinput/left_right_ascii_64 | 34.44 / 37.64 | 2736.30 / 2782.33 | 0 / 13 |
| textinput/insert_end_ascii_64 | 69.63 / 69.93 | 2870.16 / 2904.30 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 76.54 / 80.04 | 2904.36 / 2948.16 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 117.56 / 129.48 | 2786.54 / 2801.34 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 94.11 / 103.42 | 3056.24 / 3095.97 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.70 / 5.04 | 2322.91 / 2363.38 | 0 / 1 |
| textinput/word_roundtrip_unicode | 155.20 / 176.82 | 2341.58 / 2388.95 | 0 / 16 |
| textinput/suggestions_edit_pair | 114.12 / 124.35 | 3043.89 / 3090.24 | 0 / 18 |
| cursor/visible_move_80x24 | 27.21 / 27.29 | 42412.58 / 42997.83 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 28759.16 / 31642.50 | 351769.16 / 402369.16 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 29053.34 / 31042.50 | 354511.66 / 411428.34 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24660.84 / 26684.18 | 135185.00 / 247280.82 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23274.18 / 24076.66 | 121883.34 / 185967.50 | 0 / 349 |
| help/short_unbounded | 281.92 / 290.17 | 11272.58 / 11572.88 | 1 / 23 |
| help/short_40 | 244.88 / 272.92 | 8570.50 / 8791.21 | 1 / 18 |
| help/full_40 | 385.58 / 388.54 | 11435.00 / 11842.67 | 5 / 62 |
| help/short_unicode_80 | 466.25 / 489.08 | 6191.83 / 6372.58 | 1 / 12 |
| clocks/format_zero | 19.46 / 19.54 | 20.92 / 23.58 | 1 / 1 |
| clocks/reused_format_zero | 7.25 / 7.33 | 7.33 / 7.67 | 0 / 0 |
| clocks/format_nanoseconds | 31.83 / 32.04 | 15.54 / 15.92 | 1 / 1 |
| clocks/reused_format_nanoseconds | 16.54 / 16.62 | 10.29 / 10.88 | 0 / 0 |
| clocks/format_microseconds | 39.46 / 39.75 | 16.50 / 16.75 | 1 / 1 |
| clocks/reused_format_microseconds | 26.62 / 26.71 | 12.50 / 12.54 | 0 / 0 |
| clocks/format_seconds | 44.12 / 47.17 | 21.62 / 22.12 | 1 / 1 |
| clocks/reused_format_seconds | 28.46 / 29.54 | 16.25 / 16.29 | 0 / 0 |
| clocks/format_hours | 50.38 / 51.67 | 21.46 / 22.00 | 1 / 1 |
| clocks/reused_format_hours | 31.58 / 32.83 | 17.58 / 17.67 | 0 / 0 |
| clocks/format_maximum_go | 54.42 / 55.25 | 30.92 / 31.29 | 1 / 1 |
| clocks/reused_format_maximum_go | 39.08 / 39.21 | 26.58 / 26.71 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.26 | 6.10 / 6.36 | 0 / 0 |
| clocks/countdown_paused | 1.25 / 1.26 | 6.50 / 6.51 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 6.11 / 6.16 | 0 / 0 |
| clocks/countdown_foreign | 1.25 / 1.26 | 6.54 / 6.57 | 0 / 0 |
| spinner/plain_line | 2.19 / 2.20 | 643.80 / 721.58 | 0 / 0 |
| spinner/plain_dot | 2.19 / 2.20 | 636.72 / 644.29 | 0 / 0 |
| spinner/plain_mini_dot | 2.19 / 2.20 | 639.88 / 649.70 | 0 / 0 |
| spinner/plain_jump | 2.19 / 2.27 | 644.64 / 658.22 | 0 / 0 |
| spinner/plain_pulse | 2.19 / 2.29 | 642.00 / 651.22 | 0 / 0 |
| spinner/plain_points | 2.19 / 2.20 | 636.05 / 649.39 | 0 / 0 |
| spinner/plain_globe | 2.19 / 2.20 | 639.90 / 650.42 | 0 / 0 |
| spinner/plain_moon | 2.19 / 2.20 | 640.76 / 649.63 | 0 / 0 |
| spinner/plain_monkey | 2.19 / 2.20 | 638.83 / 648.35 | 0 / 0 |
| spinner/plain_meter | 2.19 / 2.20 | 642.34 / 663.57 | 0 / 0 |
| spinner/plain_hamburger | 2.19 / 2.35 | 643.43 / 659.22 | 0 / 0 |
| spinner/plain_ellipsis | 2.27 / 2.35 | 641.69 / 653.61 | 0 / 0 |
| spinner/foreign | 4.24 / 4.38 | 715.66 / 728.62 | 0 / 0 |
| spinner/stale | 4.07 / 4.37 | 723.98 / 1068.50 | 0 / 0 |
| render/empty_80x24 | 6198.21 / 6686.29 | 70751.83 / 72993.38 | 1 / 54 |
| render/ascii_80x24 | 13341.38 / 13908.00 | 108590.92 / 114129.12 | 1 / 54 |
| render/unicode_80x24 | 13871.00 / 14044.88 | 86861.46 / 90700.83 | 1 / 54 |
| unicode/width | 661.99 / 682.72 | 3471.41 / 3492.60 | 0 / 0 |
| terminal/unchanged_80x24 | 4.58 / 5.38 | 42932.42 / 43134.38 | 0 / 0 |
| terminal/one_cell_80x24 | 9721.33 / 9931.75 | 44657.17 / 46458.46 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.31 / 1.67 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.11 / 2.11 | 3.05 / 3.18 | 0 / 0 |
| motion/spring_construct_0.2 | 17.16 / 17.57 | 27.95 / 31.56 | 0 / 0 |
| motion/spring_update_1 | 0.84 / 0.84 | 1.41 / 1.49 | 0 / 0 |
| motion/spring_checked_update_1 | 2.15 / 2.16 | 3.27 / 8.63 | 0 / 0 |
| motion/spring_construct_1 | 11.92 / 12.35 | 14.67 / 35.36 | 0 / 0 |
| motion/spring_update_2 | 0.84 / 0.85 | 1.38 / 4.58 | 0 / 0 |
| motion/spring_checked_update_2 | 2.15 / 2.15 | 9.26 / 10.78 | 0 / 0 |
| motion/spring_construct_2 | 17.23 / 17.83 | 60.57 / 80.79 | 0 / 0 |

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

Additional [Rust-only subscription before/after costs](subscription-performance.md) measure adapter construction and retained timer tables. These five cases are separate from the 84 matched source fixtures.

A [clock-format follow-up](clock-format-performance.md) repeats the existing sixteen clock cases with forward/reverse binaries and twenty-times-longer batches, retaining the noisy short runs. These overlap the 84 cases.
