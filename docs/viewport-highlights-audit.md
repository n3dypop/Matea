# Viewport search matches and typed highlight intent

Implementation in progress; full native/Miri/three-OS qualification and performance
remain pending. No whole-widget parity or high score is assigned.

The API accepts sorted, disjoint byte ranges in normalized content and validates
them transactionally. Scalar-boundary matches inside EGCs style complete clusters;
empty matches navigate without painting, and empty input removes stale matches.
Immutable geometry is shared across clones, while selection stays independent.
Selected StylePatch intent composes over the ordinary highlight, preserving
unspecified colors/attributes. See ADR 0059 for the contract and limitations.

Ten native integration tests cover invalid/panicking iterators, explicit false
intent, combining/emoji boundaries, multiline and empty ranges, wrapped navigation,
clipped wide padding, replacement glyphs and concurrent clones/old View snapshots.
The finite scalar-boundary/range/width/wrap corpus checks that styling preserves
plain geometry. Existing 25 viewport and eight wrapping tests also passed after
sharing clipping bounds; new qualification is still required.

Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 public View output was captured
for five corpora × three widths × two heights × two wrap modes × seven actions:
420 raw observations, including all ANSI bytes. Ultraviolet's pinned public styled
string/ScreenBuffer path provides cells. A separate strict ASCII SGR replay verifies
252 of them. Source reproduction is not Matea equivalence: 190 complete projected
states match and 230 have retained literal differences. The deviation ledger keeps
row identity/reasons; style projection includes every cell's symbol/width and all
three requested flags, with canonical zero placeholders for wide continuations.
Unicode output is not independently proved for physical terminal fonts.

Differences include retaining highlight intent under selected style, clearing on
empty input, whole intersecting EGC matching, logical/display navigation and the
previously specified whole-EGC clipping/wrapping policy. They are explicit Matea
contracts; source behavior is retained rather than rewritten to manufacture parity.
Six end-to-end install/view/next-previous frame workloads have matching complete
outputs under an explicitly complete source selected style. Native distributions,
requested allocations and adversarial tails are still to be collected. Large and
varied documents, residency, gutters and all source styles remain open.

At af4d043 the source/Matea/Matea/source native baseline retained 24 timing
records and 12 complete output-only records. Independent Python reconstruction
checks all 72 projected cells and final x/y FNV for each. ASCII install/frame
medians were 47.0–47.5 / 9.01–9.04 microseconds in Matea versus 261–264 /
42.8–45.5 in the source. Unicode install/frame were 34.9 / 9.55–9.61 versus
218.6–218.7 / 42.9–45.0. These are full projection workloads, including
serialization and source ANSI interpretation, not isolated algorithm ratios.
Matea install/frame tracked 15/8 allocations versus source 404–413/195–204.

The next trial bounds input size-hint reservation at 256 entries, after validation
of the first match, and bounds initial visible-span reservation at 64 entries.
ASCII anchor lookup uses the owned logical-line byte index. ASCII painting walks
only intersecting match intervals, while Unicode retains whole-EGC segmentation.
Adversarial hint tests and all 420 literal states must remain unchanged. No speedup
is assigned before collecting the unchanged measurement caller again. Full ASCII
detection still scans the content on replacement; giant prefix-only match sets,
resident memory and varied style densities are not evaluated by this corpus.

The optimized ed05ad9 cohort uses exactly the same caller/measurement hashes as
af4d043. All 48 timing and 24 output-only records pass independent 72-cell/x/y
reconstruction. ASCII install/frame medians change 47.0–47.5 / 9.01–9.04 to
8.42–8.47 / 7.55–7.61 microseconds. Install/frame tracked allocations change
15/8 to 8/6, requested bytes 16,613/6,493 to 10,021/5,981. Unicode frame/cycle
latencies overlap their baselines; Unicode install includes a slightly slower
35.49 microsecond observation and no latency improvement is assigned. Source
measurements and every adverse tail remain in the report, without a universal
maximum-speed claim.

At ed05ad9 all 39 native acceptance commands passed with unchanged 177 inputs;
Rust 237 tests plus six doctests, isolated reference eight tests, Python 131,
16 snapshots and 21 Unix PTY scenarios pass. LLVM observes 83 source paths / 89
compiler instances, not every branch/profile. Miri seed 269 passes eight targeted
tests; the finite scalar range corpus and literal 420-row matrix execute natively
and were deliberately skipped in Miri. Three-host CI 39196946306 is in progress;
af4d043's earlier unfinished candidate jobs are superseded, not all-pass evidence.
See the immutable `evidence/viewport-highlights/optimized/native` snapshot.
