# Progress: implemented-module performance qualification

This review covers the implemented Progress contract. Full reference color,
alpha, style and public-capability equivalence remains partial. It does not
advance Stage B or F5. Native distribution evidence comes from one macOS arm64
host; Linux/Windows/macOS source qualification is tracked separately.

## Operation coverage

| Implemented public operation family | Executed workload |
| --- | --- |
| Default/new/new_at, identity, dimensions, target/motion/rest observations | Two fresh-constructor controls; all lifecycle controls validate their final public state |
| Width/glyph setters, glyph rejection, formatter installation/reset and drop | `api_configure_formatters_glyph_errors`; zero/one/1,024/65,535-cell scale controls |
| Cached spring configuration and validation | Three damping regimes; nonfinite spring/percent/frame errors with rollback checks |
| Target setting, increment/decrement, reduced-motion controls and clocks | 64 retargets; six real-clock controls plus explicitly ordered backward observation |
| Schedule, generation validation, accepted steps, obsolete/duplicate/backward ticks | Three rejected-tick controls, 64 descriptor reads, exact-rest animation, theme reduction |
| Subscription ownership | Construct/drop the actual boxed subscription; theme and component reduction omit it; no worker/thread is started |
| Frame/view, Component view/event, owned geometry and RGB8 cell output | Six matched complete frames, live Component frame and ignored focus input; all 120 pinned cell/color fixtures |
| Borrowed cell/frame observations, iterators, nth/back/exhaustion | Six distinct borrowed-cell controls and explicit iterator-boundary control |
| Formatter normalization/clipping and scale | 8 KiB ASCII and Unicode/control labels; 544 old/new normalization conditions |
| Nested placement, clipping, Unicode child boundaries and callback order | Old-composition/full-surface comparisons, including regional indicators and combining/emoji labels |

Simple observations and enum/error representation also receive source review;
this table does not assert a separately timed distribution for every getter or
formatting trait. Arbitrary user callback bodies are outside a finite speed
guarantee. Terminal transport, runtime scheduling and end-to-end framework
optimization retain their own ownership and pending assessments.

## Measured changes and rejected attempts

The unchanged caller retains 100 warm-ups, confirmed 20 ms calibration, 31
positive observations, first calls and a separate requested-allocation pass.
Every complete output and return value matches. Each experiment uses
old/new/new/old order, with 27 cases per cohort: six matched frames, six
borrowed-cell controls and fifteen native lifecycle/scale controls. These
experiments compare Rust implementations; existing pinned Go distributions
remain historical and no fresh Go ratio is assigned.

1. **Whole-width owned scene, `b1b00ae`:** reduces colored-frame requests from
   six to four. It unnecessarily retains a whole row even when little is filled.
   Preserve [all 108 records and 124 child-RSS observations](evidence/progress/lifecycle/report.json).
2. **Colored segment/layers plus CompactString character pushes, `1868828`:**
   limits retained cells to the colored segment, but long Unicode normalization
   regresses from about 32 to 51 microseconds despite fewer allocation requests.
   Reject that normalization buffer. [All adverse observations](evidence/progress/lifecycle/segment/report.json).
3. **Colored segment/layers plus standard String buffer, `34b1ac3`:** keeps
   whole-text grapheme handling for the empty run/label, uses one normalization
   pass, then moves storage into the inline-capable label type. The label reaches
   15.5–15.9 microseconds versus 32.3–32.8, with requests 19→8 and requested
   bytes 52,702→36,326. [Final 108 records and 124 RSS observations](evidence/progress/lifecycle/string/report.json).

Final colored-frame requests are six→five and requested bytes are 5,401→2,869,
4,951→2,743, 10,346→5,388 (two cases), and 26,385→13,177. Four colored-frame
cases have clear median gains in both pairs; the shortest gradient case is
nearly unchanged in one pair. Unchanged controls retain small adverse medians
and tails, including one maximum-width cohort; none is removed or clamped.
Borrowed cells retain zero allocation requests. No universal speedup follows.

The five additional public-API controls retain two cohorts/ten records and 31
distinct-child RSS observations. Constructors/configuration, clock reads,
validation/assertions, callback/subscription ownership, drop and the live frame
are included where named. They receive no upstream ratio. Representative p50
costs are 671–673 ns for configuration, 145–146 ns for clocked controls,
44.9 ns for subscription construction/reduction and 2.314–2.316 microseconds
for the live frame. [Exact accounting and production-source identity](evidence/progress/lifecycle/api/report.json).

RSS is measured after ordinary timings using actual `/usr/bin/time -l` child
logs. It covers a whole output-only fixture group, not one frame's residency.
Both old/new scale groups are about 8.3 MB; allocation requests are separate
from residency. All 403 logs across all attempts are losslessly archived with
member hashes and archive-aware reporters. Builds, native tests, LLVM, Miri and
downloads do not overlap ordinary measurement cohorts.

The final production library/manifests/lock bytes are unchanged between the
27-case measurement and the supplementary API caller. The latter records this
identity explicitly; it does not relabel an older whole-binary timing as current.
Native/Miri/CI evidence remains source-scoped and failures stay retained.

[Decision and ownership boundaries](decisions/0067-progress-lifecycle-scale-and-owned-cell-candidate.md).
