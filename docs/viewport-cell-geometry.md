# Viewport cell geometry and input checkpoint

Pinned reference: Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4,
viewport/viewport.go and keymap.go; x/ansi
faa4adf95555331dfb439b716cc83767ecdadeb4, ansi/truncate.go.
This extends the owned-content/vertical slice; 50 of the 70 declaration
candidates are now partially linked, with 20 still unlinked. Full module parity,
performance and Foundation gates remain pending.

Explicit `set_size`/`set_width` enables horizontal cell coordinates. Default
parent sizing retains the earlier vertical path. Empty/fitting fractions are
finite, movement saturates, explicit resize/replacement clamps both positions,
and reversed ensure-visible columns fail before mutation. A lazy owned OnceLock
caches the compositor's per-grapheme width policy, preserves Send/Sync and Clone,
and is invalidated exclusively with owned content. The vertical path does not
initialize it. A lazy Cow line iterator borrows ASCII/aligned Unicode slices;
only a partial wide grapheme at the left edge needs blank-prefix ownership.
The owned View assembles clipped rows into CompactString without a child View
or allocation per logical row. This describes implementation, not a speed claim.

ViewportKeys keeps constant default chords/help and only allocates explicit
Binding overrides. Matching, enabled state and help share the same configuration.
Pager defaults include half pages and Ctrl-U/D. Existing Home/End and Shift
navigation remain supported. Focused wheel events support all four directions,
Shift-wheel, enable/delta/step control and exact command-modifier policy.
See ADRs 0050 and 0051 for the provisional API and alternatives.

The initial native cohort passes 24 viewport integration methods, the lazy-cache
unit test, warning-free Clippy and a runnable API doc example. Twenty-three
integration methods passed Miri seed 239 before the final source-edge test was
appended; that exact test source is preserved. The appended edge test and the
cache/thread unit test are qualified separately. Full source-bound native/LLVM
and the new three-host probe cohort are recorded after this source checkpoint.

Public Go observations retained under native fixtures:

| Cohort | Exact comparisons | Explicit differences / limits |
| --- | --- | --- |
| Horizontal position controls, six texts, six widths, three heights | 1,404 complete state rows | No wrap/style/gutter/highlight equivalence inferred |
| Raw View cuts, five texts, five widths, eight offsets | 161 plain rows after trimming only ASCII right padding | 39 literal deviations retain all raw bytes and expected Rust cell windows |
| Default/custom/disabled key and wheel input | 33 complete x/y states | One literal Ctrl-wheel policy difference |
| Resize/replacement/overflow and height edges | Five nonnegative height states/views | Two signed-only source observations and three literal Rust improvements |

The Cut deviations include a left boundary inside CJK/ZWJ/flag graphemes,
keycap width policy, and zero-width byte retention. The pinned source can expand
a one-row View into additional rows or split the keycap sequence. Matea keeps
whole graphemes within its fixed surface and blanks partial cells, preserving
following cell coordinates. Its existing compositor assigns two cells to
keycaps and skips zero-width clusters. This is a documented width policy, not
a claim about every physical terminal/font or the reference's negotiated modes.
No fixture has been regenerated from Matea output or broadly normalized to make
the comparison pass. All 39 literal native expectations derive from independent,
hand-specified cluster widths and original cell positions.

Separate public observations show stale x=7 after width 3→20 (View starts at
`789`) and after content replacement with `tea` (View becomes blank). Matea
commits x=0 and shows the complete fitting text. Starting at x=1, positive
ScrollRight(MaxInt) wraps Go's signed position to x=0; Matea saturates at x=7.
These bounded sequences are improvements, not an assertion about every legacy
version or every source viewport feature.

Issue bodies #879 and #288 describe older negative-height slicing and near-fit
NaN behavior. Both are absent in the retained current-pin sequences; negative
sizes are unrepresentable in Matea's u16 size API. Use saturating subtraction
when deriving available dimensions. x/ansi #541 also concerns malformed byte
strings and ANSI truncation. Its proposed TruncateLeft fuzz predicate treats a
removed-prefix length as an output cap, which is not that API's contract. The
raw ANSI path remains pending; this text viewport accepts valid UTF-8 and uses
the shared control sanitizer. Do not call the issue fixed from these text tests.

The new performance workloads separately compare complete construction, content
replacement, horizontal movement, ensure-visible, plain View consumption, key
dispatch and wheel dispatch for ASCII/Unicode documents. Four Rust-only borrowed
iterator controls are distinct and receive no upstream ratio. Constructor
metadata differs because other viewport capabilities are still missing; Go
GC/TotalAlloc and Rust requested-layout bytes are not resident memory. Preserve
all cold calls, calibrated distributions, allocations, output checks and adverse
controls before drawing any bounded speed conclusion.

## Optimized source checkpoint

Source 53cf699 keeps the root API while moving the component into
`crates/matea-widgets/src/viewport.rs`. The typed default-code selection validates
canonical chord modifiers, and custom overrides retain their original priority.
ASCII maximum width comes from the LF index; adjacent equal Unicode rows skip
repeated segmentation without another allocation. A varied-row fixture protects
the ordinary width path, empty/trailing indexes and replacement invalidation.

The final native cohort passes 33 checks: 200 Rust methods plus three doctests,
129 Python methods, 16 snapshots and 21 Unix PTY cases. Miri seed 239 passes all
25 viewport methods plus the cache/thread unit. LLVM retains 72 distinct Rust
source paths and all 77 compiler instances. The 127-program/157-input source
snapshot is preserved; three-host qualification is reported separately.

Both finite corpora use the same tracked caller/dependency bytes and the same
Go executable before/after. Sixteen complete raw runs retain reference/Matea/
Matea/reference ordering, 31 calibrated positive batch means, cold calls and
allocation passes. Every complete output, state and checksum matches. An
independent verifier runs with Python `-O` and checks journals, all Git input
hashes, raw/report hashes, case sets, samples and before/after comparability.

| Workload | Before p50 | After p50 | Observation |
| --- | ---: | ---: | --- |
| Repeated Unicode construction | 26.46–26.47 µs | 7.18–7.19 µs | Reduced repeated segmentation |
| Repeated Unicode replacement | 26.43 µs | 7.12–7.13 µs | Same requested allocations/bytes |
| Default Right input, both corpora | 65.1–66.2 ns | 25.6–26.0 ns | Canonical-chord fast selection |
| ASCII construction, repeated | 5.98–6.00 µs | 5.59–5.60 µs | Reuse LF index |
| Varied Unicode construction/replacement | 32.15–32.55 µs | 32.42–32.64 µs | Observed p50 increases 0.3–1.3%; retained |

Small movement/ensure/wheel differences and adverse p95 values are retained too.
The varied Unicode comparison prevents treating the repeat case as universal.
No causal/statistical claim is made for small differences without affinity or
frequency control. Partial-wide borrowed Cow rows still allocate three times in
the three-row Unicode control; aligned iterator controls allocate zero. There is
no new RSS measurement. Full wrap/fill/gutter/styles/highlights, platform/font
conditions and the module verdict remain pending; quality axes remain below-high.

[All raw before/after distributions and source checks](evidence/viewport-horizontal/optimized/performance-comparison.json).


The final [seven-job three-host/MSRV/reference cohort](https://github.com/n3dypop/Matea/actions/runs/39158524308)
passes. An independent Python `-O` binder matches all 127 programs and 157 known
inputs to Git 53cf699, verifies 17 native logs on Unix and 18 on Windows, every
retained source fixture, both complete output corpora, calibrated-mode schemas,
all 14 attribute outputs and fixed representation sizes. Linux/macOS LLVM keeps
72 source files/77 instances; Windows keeps 71/75. The normal Windows acceptance
cohort also verifies 36 native console frames/2,160 cells and restoration.
Raw Windows hashes and retained LF hashes are separate. Compiler profiles and
complete measurement-driver qualification remain their earlier recorded cohorts.
[All host/source/log/fixture evidence](evidence/viewport-horizontal/optimized/ci/source-bindings.json).

An earlier dispatch supplied `reference_probes_only` without the parent
`reference_api` flag, so reference jobs were skipped. It is not used as reference
qualification; the successful run above supplied both flags. Its conclusion does
not change the coverage/quality gates: full public capability mapping and all-high
Stage A remain open, and Stage B/F5 remain closed.
