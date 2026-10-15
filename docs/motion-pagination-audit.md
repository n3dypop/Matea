# Motion and pagination public-capability audit

Pins: Harmonica `0ff76e405449b47bd60f1df4820d880081692d3c`, Bubbles
`c3bde0d166c2ca607d9864493453fd77bdcae7e4`. All production declarations in
Harmonica's three root Go files (20 candidates) and Bubbles paginator (29 candidates)
are linked in capabilities.json. These small source groups have no build tags,
generated exports or source issue markers. This does not resolve other modules.

## Harmonica mapping

| Source public capability | Rust contract and evidence |
| --- | --- |
| FPS | frame_duration(NonZeroU32) uses integer-nanosecond quantization, converted to seconds where needed; tests include 1, 60, 1GHz and u32::MAX |
| Spring, NewSpring, Update | Copy cached coefficients; sample is raw IEEE arithmetic, step checks input/results; 45 reference trajectories × 100 updates, analytical regime/partition tests and nine timed spring cases |
| Point/Vector and all XYZ fields | Public Point3/Vector3 value fields; fixtures exercise every axis |
| Gravity/TerminalGravity | Immutable constants instead of mutable global vectors; both orientations are tested |
| Projectile, NewProjectile, Update | Copy value; raw sample and checked transactional step; constructor and raw/checked update are measured separately |
| Position/Velocity/Acceleration | Value getters exercised by fixtures and timed updates; no borrowed mutable internals |

The 45 generated projectile trajectories cover five periods, three step counts
and three acceleration vectors, with fixed nonzero XYZ position/velocity. Both
checked and raw Rust updates match all six reference position/velocity components.
Exact constant-acceleration integration is an additional opt-in method, with
analytical and time-partition tests; it is not substituted for Euler behavior.

Deliberate Rust differences: frame periods use Duration, rates use NonZeroU32,
negative/nonfinite parameters return typed errors, constants replace shared mutable
gravity, and checked failures leave the whole state unchanged. The raw methods
retain IEEE overflow propagation. Finite representable frame periods/coefficient
matrices are the numeric contract; arbitrary Go float inputs are not a literal
signature/bit-pattern compatibility promise. Very small nonzero frequencies retain
their mathematical motion instead of Harmonica's epsilon identity cutoff.

Source-derived numerical improvement: Harmonica forms the slow overdamped root as
two nearly equal terms, which cancel at extreme damping. Matea computes it using
the reciprocal root sum. The executed source fixture at damping=1e12 reports zero
velocity upstream; Matea's regression preserves the nonzero slow decay. This is an
edge-case analysis, not a claim about typical animation accuracy or an upstream
maintainer-confirmed defect.

[Harmonica #13](https://github.com/charmbracelet/harmonica/issues/13) concerns an
OpenGL example's development packages, as confirmed by the maintainer's responses.
The root motion package has no external Go module dependencies; Matea motion has
no non-stdlib dependencies or OpenGL example. Its portable TUI example is built by
CI. The reporter's native environment was not reproduced.

## Bubbles paginator mapping

| Source declarations | Rust composition |
| --- | --- |
| Type, Arabic, Dots, Model.Type | PageDisplay and Paginator.display |
| KeyMap fields, DefaultKeyMap, Model.KeyMap | PageKeys and typed Binding; same default pgup/left/h and pgdown/right/l |
| Model Page/PerPage/TotalPages, New, Option, WithTotalPages, WithPerPage | Private Pagination invariants, item/explicit-page constructors and setters, Paginator::new/default; NonZeroUsize prevents zero divisors |
| ActiveDot, InactiveDot, ArabicFormat | Custom glyphs plus a Rust formatter callback; padding/radix/localized labels use format!, styling uses Theme/View |
| SetTotalPages, ItemsOnPage, GetSliceBounds | set_total, items_on_page, range/range_for; all ranges clamp to actual length |
| PrevPage, NextPage, OnFirstPage, OnLastPage | previous/next/is_first/is_last; all mutations preserve valid page state |
| Update, View | Component event/view and standalone label, shared focus/repeat/release/help contracts |

The source-edge probe actually reproduced PerPage=0 panic, stale bounds 27..4,
ItemsOnPage=-23 and a page count that stays 10 after clearing items. Matea's typed
state prevents these cases: zero cannot be constructed, clear means empty, stale
collection ranges are empty at length, and virtual offset overflow saturates.
The bounded dot window retains the active page; it does not silently hide it after
first-N truncation. These are intentional safety/UX improvements.

Tests exercise custom/disabled keys, repeats, releases, command modifiers, focus,
edge navigation, custom labels, dot budgets, empty state and index-size limits.
The pagination example also runs through a real Unix PTY. Six matched performance
fixtures separate slice bounds, state helpers, Arabic/dot labels and key dispatch.
Dot fixtures render all 32/256 pages identically; a structured styled View is not
included in the string-only comparison. See [ADR 0012](decisions/0012-pagination-state-and-formatting.md).

## Evidence and limits

[Executed source edges](evidence/motion-pagination-source-edges.json),
[performance distributions](performance.md), numeric fixtures in
`crates/matea-motion/tests/`, tests in motion/widgets, and the coverage ledger.
Reproduce source edges with `probe --source-regressions` in the pinned disposable
Go module; `--projectile-fixtures` produces the numeric trajectory CSV.

Measurements use one macOS arm64 host and retain all samples. They are module
workload evidence, not universal speed or real-terminal latency guarantees.
All-reference closure, base-library exports, real-emulator input and F5 remain
open. No image processing, decoding or CUDA is involved.
