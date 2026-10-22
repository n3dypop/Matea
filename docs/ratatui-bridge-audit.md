# Managed Ratatui widgets and owned cell scenes

The initial adapter executes the pinned widget code under Matea's existing runtime
and terminal lease. It returns an owned scene and retains no source widget, buffer
or state borrow. The facade's optional `ratatui` feature exposes the provisional
adapter; `cargo run -p matea --features ratatui --example ratatui_widgets` exercises
keyboard selection, theme conversion and native scene composition without a second
terminal. Publication and the Foundation gate remain closed.

ADR [0060](decisions/0060-owned-cell-scenes-and-consuming-frame-transfer.md)
adds immutable `View::surface` snapshots and whole-glyph row copying. A uniquely
owned root snapshot transfers its cell vector through `into_surface`; shared or
constrained scenes follow the documented copy/composition path. View remains 48
bytes and Cell 40 bytes. Seven native tests include 4,096 independently replayed
clipping/damage cases; targeted Miri repeats 96 of those cases. A facade integration
test compares complete terminal frame bytes across four color levels and Unicode
fallback, including cursor and wide-to-narrow transitions.

ADR [0061](decisions/0061-pinned-ratatui-widget-adapter-and-ui-local-state.md)
uses exact Ratatui d1ef782b48ddf0cb825c111d4d8b9adef572c0e1. The adapter disables
source backend defaults and always preserves underline color. Default, minimal and
all forwarded feature profiles compile/test locally. The scoped `StatefulWidget`
helper and UI-local `WidgetState<S>` impose no Send/Sync bound; contention returns
an error and panic releases the borrow. Generic foreign state is not rolled back.

Nine native adapter tests cover the actual stateless/stateful catalog, complete
named/indexed/RGB colors and representable modifiers, malformed geometry/symbols,
source ownership, themes, borrow/unwind behavior and batched-run boundaries. The
new row test crosses 256-byte chunks, styles, Unicode, continuation cells and rows,
then introduces late contradictory modifiers/control symbols. A failed conversion
publishes no partial scene. The initial eight adapter tests and seven snapshot
tests pass Miri seed 317. The optimized importer separately passes all nine
adapter tests in 140.15 seconds; both source-bound contexts and logs are retained.

The [77-declaration map](evidence/ratatui-bridge/declaration-map.json) links only
directly used public types and called builders/state methods. It does not complete
the containing module or its untested children. Raw diff/skip/forced-width and
halfwidth-katakana width-policy contracts currently return explicit errors. Other
source backend, no_std, unstable/optional behavior and compiler export closure
remain separate obligations. Reexports alone do not close them.

The [dependency audit](evidence/ratatui-bridge/dependency-graph.json) records 55
normal adapter packages on local Apple Silicon, license declarations, MSRV at most
1.88 and all shared-name version-set differences. The production root resolves
168 packages; the independent text reference workspace resolves 48, including its
43 frozen source packages and local callers. Its lock remains unchanged. Exact Git
source with a different resolved graph is distinct evidence. No image engine,
codec/decoder or CUDA package is active in the adapter tree.

The matched performance corpus uses existing 100-line ASCII/Unicode input, fresh
source buffers and selected=50 state for Paragraph, List and Table at 24×6 and
80×24. Both paths emit complete dimensions, lead-cell UTF-8/width, three canonical
colors and all nine source modifier bits, with explicit zero continuation markers.
Independent Python decoding verifies literal borders/body/selection/styles and
recomputes FNV64. Timings include projection/serialization and drop; they do not
isolate widget rendering or conversion. Physical terminal appearance is untested.

Three source/Matea/Matea/source cohorts retain all **144 timing records and 72
complete output-only records**, with unchanged caller/checker hashes and all 31
samples, calibration observations, cold times and allocation fields. The
[comparison](evidence/ratatui-bridge/comparison.json) keeps each run separately.
The baseline, checked-style/ASCII trial and batched-ASCII trial are all retained.
The importer now reuses one validated resolved style and batches equal-style
printable ASCII into a 256-byte stack buffer. It never joins Unicode cell symbols
or trusts unvalidated flags; no allocation map is introduced.

| Case, 80×24 | Baseline Matea p50 µs | Batched Matea p50 µs | Source p50 µs |
| --- | --- | --- | --- |
| ASCII Paragraph | 197.88–198.25 | 152.71–152.73 | 110.72–111.35 |
| ASCII List | 201.29–201.67 | 153.84–154.62 | 112.94–113.14 |
| ASCII Table | 214.27–214.42 | 166.58–166.71 | 124.88–125.44 |
| Unicode Paragraph | 211.31–211.57 | 170.18–170.76 | 117.56–118.30 |
| Unicode List | 214.46–215.03 | 171.53–171.57 | 119.08–119.88 |
| Unicode Table | 227.08–227.35 | 183.43–183.79 | 131.41–131.57 |

The 24×6 cases also improve in this corpus; raw adverse tails remain in the report.
Allocation counts are unchanged by these importer trials. Matea retains two extra
allocations versus direct source rendering (owned Surface storage and scene Arc).
Large ASCII Paragraph uses 123 versus 121 total allocations, including widget
builders and the complete output projection. This is measured conversion overhead,
not reference performance parity, zero-allocation rendering or a maximum-speed
claim. Separate render-only, residency, varied/hostile workloads and terminal I/O
measurements remain pending before a high module performance verdict.

All 44 whole-workspace native acceptance commands pass with 187 unchanged
execution inputs: Rust 254 tests plus seven doctests, isolated reference eight,
Python 135, 17 snapshot apps and 21 Unix PTY scenarios. LLVM records 90 source
paths / 96 compiler instances, retaining every feature-profile executable object.
Current Miri evidence covers [seven snapshot tests](evidence/ratatui-bridge/miri/context.json),
[nine optimized adapter tests](evidence/ratatui-bridge/miri/optimized-context.json)
and [one runtime/frame-byte test](evidence/ratatui-bridge/miri/facade-context.json).
The [native binding](evidence/ratatui-bridge/native/source-bindings.json) checks
147 program hashes, all 44 logs and the complete input set. Timing caller and
production hashes match the final trial; only the quality metadata and LLVM object
collector changed afterward. Both strict quality/coverage gates correctly fail.

Exact-source three-OS/MSRV CI remains pending. Signing/commit was not executed
because the automatic approval review service reported model capacity exhaustion;
this was a review failure, not an unsafe-action verdict. Earlier ed05ad9 CI
qualifies the highlight checkpoint only and cannot qualify this adapter.


The separate [viewport issue reassessment](evidence/ratatui-bridge/viewport-issue-refresh.json)
reviews historical reports #288/#879 after the viewport hash changed for highlights.
The default/flat denominator and height/range guards remain bounded and covered by
fresh native regressions. Viewport source/tests match the retained ed05ad9 three-host
binding exactly. Prior review maps are preserved, the original historical checkout
is not claimed reproduced, and the new adapter's portability remains pending.
This reduces pending issue assessments from 827 to 825 without changing source or
relaxing a gate. The native acceptance snapshot retains its original 827 count;
the current derived gate status separately records the subsequent 825 assessment.
