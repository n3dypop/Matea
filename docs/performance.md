# Foundation performance evidence

Status: measured workloads only. All-reference/module performance parity remains pending.

Raw samples, source hashes, compilers, pins and limitations: [report.json](evidence/performance/2026-10-16/report.json).
One macOS arm64 host, sequential release executables, memory sinks, 31 batches. Figures are nanoseconds per operation.

| Workload | Matea p50 / p95 | Upstream p50 / p95 | allocations M / U |
| --- | ---: | ---: | ---: |
| input/binding_ctrl_x | 3.81 / 3.83 | 28.75 / 41.29 | 0 / 1 |
| input/binding_f12 | 4.34 / 4.71 | 30.27 / 35.23 | 0 / 1 |
| input/binding_miss | 4.64 / 5.34 | 26.11 / 43.20 | 0 / 1 |
| input/binding_disabled | 1.31 / 1.32 | 28.88 / 41.02 | 0 / 1 |
| input/binding_repeat | 3.21 / 3.53 | 27.66 / 40.67 | 0 / 1 |
| input/binding_release | 1.60 / 1.87 | 2.03 / 2.10 | 0 / 0 |
| binding/value_hit | 4.63 / 6.29 | 6.83 / 7.34 | 0 / 0 |
| binding/value_miss | 3.58 / 3.68 | 6.72 / 7.26 | 0 / 0 |
| binding/value_disabled | 0.77 / 0.77 | 6.78 / 7.30 | 0 / 0 |
| binding/construct | 37.75 / 48.26 | 46.64 / 51.93 | 1 / 2 |
| binding/value_hit_owned | 6.03 / 6.48 | 6.89 / 7.08 | 0 / 0 |
| binding/metadata | 1.24 / 1.26 | 2.10 / 2.12 | 0 / 0 |
| motion/frame_duration | 0.71 / 0.71 | 2.10 / 2.27 | 0 / 0 |
| motion/projectile_update | 2.44 / 2.49 | 5.48 / 5.65 | 0 / 0 |
| motion/projectile_checked_update | 2.71 / 3.00 | 6.10 / 6.42 | 0 / 0 |
| motion/projectile_construct | 8.50 / 9.81 | 20.08 / 21.40 | 0 / 1 |
| pagination/range | 0.82 / 0.88 | 0.94 / 0.99 | 0 / 0 |
| pagination/navigation | 1.75 / 1.83 | 5.30 / 5.68 | 0 / 0 |
| pagination/arabic | 47.29 / 47.38 | 64.29 / 65.17 | 1 / 1 |
| pagination/dots_32 | 138.04 / 141.71 | 749.96 / 853.00 | 1 / 31 |
| pagination/dots_256 | 910.04 / 977.21 | 12716.83 / 13554.62 | 1 / 255 |
| pagination/key_dispatch | 8.38 / 9.46 | 146.70 / 166.12 | 0 / 3 |
| textinput/left_right_ascii_64 | 35.21 / 37.85 | 2738.15 / 2803.66 | 0 / 13 |
| textinput/insert_end_ascii_64 | 69.24 / 72.48 | 2856.98 / 2974.24 | 0 / 5 |
| textinput/insert_middle_ascii_64 | 77.40 / 81.57 | 2912.51 / 2936.86 | 0 / 6 |
| textinput/word_roundtrip_ascii_63 | 116.47 / 123.79 | 2782.62 / 2819.65 | 0 / 17 |
| textinput/paste_clear_ascii_64 | 93.62 / 97.62 | 3050.85 / 3101.73 | 0 / 19 |
| textinput/ignored_focus_ascii_64 | 4.64 / 4.66 | 2322.32 / 2345.76 | 0 / 1 |
| textinput/word_roundtrip_unicode | 155.47 / 161.89 | 2331.44 / 2384.26 | 0 / 16 |
| textinput/suggestions_edit_pair | 112.40 / 125.01 | 3019.48 / 3107.80 | 0 / 18 |
| cursor/visible_move_80x24 | 27.17 / 27.29 | 42526.88 / 43054.83 | 0 / 1 |
| terminal/colored_full_ansi16_80x24 | 28665.00 / 30088.32 | 351164.18 / 408298.32 | 0 / 429 |
| terminal/colored_full_ansi256_80x24 | 28751.66 / 30150.84 | 351995.00 / 405213.34 | 0 / 489 |
| terminal/colored_full_truecolor_80x24 | 24460.84 / 25494.18 | 134541.66 / 179142.50 | 0 / 549 |
| terminal/colored_full_none_80x24 | 23166.68 / 24489.16 | 121420.82 / 176474.16 | 0 / 349 |
| help/short_unbounded | 264.00 / 275.12 | 11105.33 / 11450.17 | 1 / 23 |
| help/short_40 | 234.12 / 253.04 | 8431.46 / 8623.46 | 1 / 18 |
| help/full_40 | 374.17 / 399.00 | 10782.12 / 11339.17 | 5 / 62 |
| help/short_unicode_80 | 479.75 / 501.75 | 6231.29 / 6514.71 | 1 / 12 |
| clocks/format_zero | 22.62 / 22.71 | 11.04 / 11.79 | 1 / 1 |
| clocks/reused_format_zero | 7.25 / 7.29 | 7.33 / 7.88 | 0 / 0 |
| clocks/format_nanoseconds | 26.71 / 26.92 | 15.00 / 15.62 | 1 / 1 |
| clocks/reused_format_nanoseconds | 14.92 / 15.25 | 10.29 / 10.62 | 0 / 0 |
| clocks/format_microseconds | 30.96 / 32.08 | 16.50 / 17.96 | 1 / 1 |
| clocks/reused_format_microseconds | 17.00 / 17.25 | 12.50 / 12.54 | 0 / 0 |
| clocks/format_seconds | 37.12 / 37.25 | 21.83 / 22.83 | 1 / 1 |
| clocks/reused_format_seconds | 23.92 / 23.96 | 16.25 / 16.88 | 0 / 0 |
| clocks/format_hours | 37.08 / 49.04 | 21.29 / 22.25 | 1 / 1 |
| clocks/reused_format_hours | 23.92 / 25.04 | 17.62 / 18.33 | 0 / 0 |
| clocks/format_maximum_go | 47.29 / 47.50 | 30.96 / 32.21 | 1 / 1 |
| clocks/reused_format_maximum_go | 28.08 / 28.25 | 26.58 / 27.71 | 0 / 0 |
| clocks/stopwatch_paused | 1.25 / 1.26 | 6.08 / 6.34 | 0 / 0 |
| clocks/countdown_paused | 1.25 / 1.25 | 6.60 / 6.74 | 0 / 0 |
| clocks/stopwatch_foreign | 1.25 / 1.26 | 6.28 / 7.48 | 0 / 0 |
| clocks/countdown_foreign | 1.25 / 1.26 | 6.76 / 7.02 | 0 / 0 |
| spinner/plain_line | 1.54 / 1.57 | 639.16 / 664.07 | 0 / 0 |
| spinner/plain_dot | 1.51 / 1.51 | 638.40 / 651.65 | 0 / 0 |
| spinner/plain_mini_dot | 1.49 / 1.50 | 637.40 / 650.72 | 0 / 0 |
| spinner/plain_jump | 1.48 / 1.48 | 637.60 / 650.23 | 0 / 0 |
| spinner/plain_pulse | 1.47 / 1.48 | 642.42 / 659.10 | 0 / 0 |
| spinner/plain_points | 1.47 / 1.59 | 638.54 / 652.33 | 0 / 0 |
| spinner/plain_globe | 1.47 / 1.47 | 640.10 / 660.20 | 0 / 0 |
| spinner/plain_moon | 1.47 / 1.48 | 638.20 / 660.74 | 0 / 0 |
| spinner/plain_monkey | 1.55 / 1.75 | 642.58 / 666.32 | 0 / 0 |
| spinner/plain_meter | 1.48 / 2.80 | 638.47 / 649.75 | 0 / 0 |
| spinner/plain_hamburger | 1.47 / 1.48 | 638.76 / 656.79 | 0 / 0 |
| spinner/plain_ellipsis | 1.48 / 2.62 | 643.02 / 664.22 | 0 / 0 |
| spinner/foreign | 2.81 / 2.82 | 712.30 / 736.36 | 0 / 0 |
| spinner/stale | 2.81 / 2.82 | 720.95 / 1141.69 | 0 / 0 |
| render/empty_80x24 | 6218.83 / 6418.25 | 68787.54 / 70570.88 | 1 / 54 |
| render/ascii_80x24 | 12792.62 / 12906.71 | 106735.92 / 109400.62 | 1 / 54 |
| render/unicode_80x24 | 13572.08 / 13746.29 | 84317.08 / 86833.33 | 1 / 54 |
| unicode/width | 664.49 / 685.88 | 3463.66 / 3499.57 | 0 / 0 |
| terminal/unchanged_80x24 | 4.79 / 5.08 | 42720.71 / 43006.62 | 0 / 0 |
| terminal/one_cell_80x24 | 9704.00 / 9883.21 | 44153.29 / 44768.38 | 0 / 5 |
| motion/spring_update_0.2 | 0.82 / 0.82 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_0.2 | 2.22 / 2.23 | 2.97 / 3.00 | 0 / 0 |
| motion/spring_construct_0.2 | 17.03 / 18.04 | 26.72 / 28.06 | 0 / 0 |
| motion/spring_update_1 | 0.82 / 0.82 | 1.31 / 1.34 | 0 / 0 |
| motion/spring_checked_update_1 | 2.23 / 2.34 | 2.97 / 3.06 | 0 / 0 |
| motion/spring_construct_1 | 11.72 / 12.86 | 13.58 / 14.41 | 0 / 0 |
| motion/spring_update_2 | 0.84 / 0.84 | 1.29 / 1.29 | 0 / 0 |
| motion/spring_checked_update_2 | 2.27 / 2.37 | 2.97 / 3.11 | 0 / 0 |
| motion/spring_construct_2 | 17.49 / 18.72 | 25.16 / 47.84 | 0 / 0 |

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
