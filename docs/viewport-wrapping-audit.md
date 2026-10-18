# Viewport wrapping and deferred geometry reuse

At 32a67c0 the eight new integration tests and existing 25 viewport tests pass;
the full native driver passes 37 checks (220 workspace tests plus four doctests,
130 Python tests, 16 snapshots, 21 Unix PTY cases). Eight wrapping tests pass
Miri seed 257. LLVM retains 77 canonical sources / 83 compiler instances. All
seven three-OS/MSRV/reference jobs pass; the independent source binding verifies
132 programs / 168 execution inputs and all uploaded native logs. See
`evidence/viewport-wrap/optimized/ci/source-bindings.json`.

Wrapping uses complete graphemes and a lazy shared byte-start index. Logical
line access remains stable; display count/vertical positions use wrapped rows.
Explicit width is needed. Zero width produces zero rows without float-to-int
conversion. One oversized cluster becomes one replacement cell; original text
is retained. Fill-height adds borrowed empty display rows. Invalid ensure ranges
do not mutate state. See ADRs 0056/0057 for lifetime, invalidation and API choices.

The pinned Bubbles c3bde0d matrix retains 1,008 complete raw source Views and
Matea states: 586 paired matches and 422 literal deviations on the source Mac
snapshot. Of 840 nonzero-width states, 570 match and 270 retain deviations for
complete clusters, logical ensure mapping and per-EGC script width. Each OS
reproduces those 840 **source** observations exactly; this does not declare all
840 equal to Matea. The 168 zero-width Go numeric observations are retained per
host because out-of-range float-to-int conversion/overflow is target dependent.
The Mac snapshot has 16 matches / 152 explicit deviations in that group. Source
public totals include -1 and MaxInt at width zero; Matea's count/positions stay 0.
No source panic occurred in these bounded sequences. Physical shaping is untested.

Ten matched performance cases cover fresh construction/read, initialized count,
scroll, complete visible frame/read and width 4→8→4 geometry for ASCII and
Unicode. Two borrowed-iterator controls have no reference ratio. Each document
has 100 equal, aligned lines; no ratio uses deliberately different wrapping.
Both orders preserve 31 confirmed-calibration samples, cold and allocation data.
Independent Python reconstructs all documents/frames and checks 88 timing /
44 output-only records, complete text and FNV state digests.

| Warm operation | Matea p50 | Pinned source p50 | Scope |
| --- | --- | --- | --- |
| ASCII construct/read | 3.38 us | 25.1–25.3 us | Includes complete owned snapshot/geometry/drop |
| Unicode construct/read | 13.04–13.14 us | 33.6–33.8 us | Per-EGC geometry plus snapshot |
| Initialized display count | about 1.3 ns | 9.0–14.1 us | Immutable geometry memo; no render |
| ASCII frame/read | 426–427 ns | 14.7–14.9 us | Scene/Surface/plain versus View/plain adapter |
| Unicode frame/read | 554–559 ns | 21.2–21.5 us | Complete three-row owned output |
| ASCII geometry resize cycle | 721–724 ns | 18.3–18.4 us | Both complete totals consumed |
| Unicode geometry resize cycle | 17.44–17.53 us | 29.2–29.3 us | Full reflow, not just setter time |

Baseline reflow allocates five times. Defer rebuilding until a query, retain the
old Arc in an optional spare mutex and reuse only when Arc::get_mut proves unique
ownership. Shared clones preserve their previous geometry. Initialized reflow
then allocates zero times; fresh construction remains six allocations. ASCII
reflow p50 improves from 876–880 to 721–724 ns; Unicode improves from 17.68–17.77
to 17.44–17.53 us. Small constructor/tail adverse changes remain in
`evidence/viewport-wrap/performance-comparison.json`; they are not discarded or
called statistically causal. No lock/Arc clone is added to initialized readers.

Requested System bytes are not RSS, stack layout or OS first-lock allocation.
Go counters average TotalAlloc; Rust counts one allocation pass. Repeated aligned
corpora do not close varied/wide/script/ANSI/large-module performance or maximum
possible speed. Styles, gutters, highlights and complete public module closure
remain pending. Stage A, Stage B and F5 are not complete.

A follow-up qualification change explicitly executes the excluded reference-text
workspace's own eight unit tests in native/CI/MSRV and LLVM, beyond its existing
80-cell conformance caller. That validation-only change is being qualified
separately; the 37-check source binding above remains immutable.
