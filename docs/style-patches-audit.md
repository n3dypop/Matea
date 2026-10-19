# Explicit cell-style patches

Implementation/qualification in progress. No high performance or whole-style
module verdict is assigned. This is a separate intent value; the resolved Style
stored in every Cell remains 14 bytes, with a 16-byte Copy/Eq/Hash StylePatch.
Foreground/background/underline-color options distinguish inheritance from an
explicit terminal default. Attribute mask/value pairs preserve explicit false.
Underline/blink/font-frame groups have typed setters and no invalid raw numeric
constructor. `then` composes intent; `resolve` applies it to a theme/style.

Six ordinary integration tests cover inheritance/default/false, full reset,
complete overrides, Boolean/enum composition, associativity, const use and
concurrent copies. A separately executed finite test checks all 256 Boolean base
states against all 6,561 ternary patches (1,679,616 pairs). A literal terminal
encoder regression verifies explicit reset and disabled bold without introducing
manual escape emission into application APIs. See ADR 0058.

The independent reference workspace still uses pinned ratatui-core
d1ef782b48ddf0cb825c111d4d8b9adef572c0e1, its frozen 43 package versions and the
std-only profile. Native public Style/Cell observations compare 46,656 Boolean
resolutions, 531,441 intent compositions, 1,764 foreground/background resolutions
and 72 valid underline/blink choices: 579,933 observations. The conditional
underline-color profile is not silently enabled. Extra Matea decoration groups,
raw conflicting flags and serialization are not claimed equal by this corpus.

Six overlapping raw source add/sub controls confirm **removal** priority in both
`has_modifier` and `Cell::set_style`. An initial contrary assumption failed its
native assertion and was corrected; it is not an upstream defect. Retain the
correction record and complete control observations under `evidence/style-patches`.
Matea exposes deterministic last-setter intent and cannot construct overlapping
private mask/value states through safe APIs.

The measurement caller has six matched operations (build, resolve, compose,
inherit, complete override and runtime-selected reset), with complete canonical
16-byte foreground/background/six-Boolean records consumed via FNV. Two separate
25-byte expanded-decoration controls have no reference ratio. Source resolution
includes a Cell clone; representations and extra properties differ.

At c77fd31, all 39 native acceptance commands passed with unchanged inputs. The
isolated reference workspace's eight measurement unit tests ran as well. LLVM
observed 80 source paths / 86 compiler file instances; this is execution evidence,
not every branch or conditional profile. Miri seed 263 passed the six ordinary
patch tests and the terminal encoder regression. The finite test runs natively.
The source-bound native snapshot is under `evidence/style-patches/baseline/native`.
All seven native/MSRV/reference-probe jobs passed at this checkpoint:
https://github.com/n3dypop/Matea/actions/runs/39178730764. Independent artifact
bindings verify all 171 execution inputs / 135 program files against Git, logs,
fixtures and outputs. LLVM observes 80 paths / 86 file instances on Unix and
79 / 84 on Windows. The new finite test log was not uploaded at this checkpoint;
its successful command and hash are in the journal, with the raw local log
retained. The workflow now explicitly uploads that log for future checkpoints.

The first performance cohort is retained under `baseline`. Its resolved-state
read adapters converted a complete resolved state back to explicit style intent
before extracting the canonical fields. That extra conversion is unnecessary for
both implementations. A second cohort will use direct foreground/background and
modifier reads on both sides, with identical complete output bytes. The first
cohort's timings must not be used as a before/after library optimization claim
against this changed instrumentation. In particular, the initial 24.6–27.2 ns
Matea reset versus 10.7–10.8 ns source reset is still an investigation result,
not an attributed cause. No production library changes accompany this adapter
correction.

The direct-read-before cohort at e609fbe preserves this gap (Matea reset
25.6–25.9 ns, source 10.4–10.6 ns, medians from two orders). Build and complete
override are close; compose/resolve favor Matea in this bounded corpus. All eight
Matea and six source cases report zero tracked allocations. The next experiment
marks the small constructors/composition/resolution methods inline, consistently
with existing packed attribute operations. This is a compiler-hint experiment,
not evidence that a particular out-of-line call caused the gap; symbols alone
did not establish that cause. Keep the instrumentation fixed across the trial.

The inline-trial cohort at a080458 did not improve reset (25.5–25.6 ns);
the constructor/composition/resolution hints are therefore removed. The retained
arm64 disassembly shows the Matea reset loop calling `matea_resolved`, while the
source reset closure has no corresponding resolved-record helper call. No
StylePatch method call remains in that loop. This explains a concrete
instrumentation asymmetry, not a demonstrated library defect. The next measurement
forces both resolved-read adapters inline. Its timings are a new instrumentation
cohort and must not be labeled a before/after production optimization. The actual
production library returns to the c77fd31 implementation.

Style decorators, arbitrary padding/borders/layout, all Stylize conversions,
conditional underline-color/serde/anstyle/palette profiles, complete raw modifier
composition and full module performance remain open. Stage A and F5 stay closed
to advancement; the image engine and CUDA remain gated.

The final direct-inline and confirmation cohorts use identical instrumentation
and production code. Reset medians are 4.45–4.47 ns in Matea and 5.51–5.58 ns in
the source; resolve is 5.86–5.88 versus 8.56–8.64 ns. These are bounded current
comparisons after repairing the reader, not a production speedup. Compose favors
Matea here, while build and complete override generally overlap. One retained
Matea override run is 13.19 ns versus the other 6.79–6.82 ns observations; all
samples and calibration history remain published. The confirmation does not erase
that adverse result. All cases report zero tracked allocations. Complete style
profiles, retained memory and module coverage still prevent a high verdict.
