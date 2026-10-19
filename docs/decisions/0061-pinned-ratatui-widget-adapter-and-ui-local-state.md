# ADR 0061: Pinned Ratatui widget adapter and UI-local state

Status: initial implementation/qualification in progress. Package/API names are
working names; no registry availability or publication approval is asserted.

The 49 below-high axes and thousands of unlinked declarations show that local
primitive tuning alone cannot close the complete reference scope. Reevaluate
ADR 0004's narrow renderer by integrating the existing pinned Rust widget catalog
behind an independent adapter. Keep Matea's structured scene, theme/transport
boundaries and terminal lease. Reexports alone remain insufficient evidence.

Add the provisional matea-ratatui package, depending on render/style and exact Git
Ratatui d1ef782b48ddf0cb825c111d4d8b9adef572c0e1. Reuse its widget builders and
state APIs and render actual Widgets/StatefulWidgets to a temporary source Buffer,
validate/import cells, then produce an owned immutable Matea View (ADR 0060).
No source widget, buffer or state borrow escapes and no foreign Terminal is opened.
The facade offers this package behind an optional `ratatui` feature. Existing
native Matea components/styles stay independently usable.

The adapter's source facade disables backend defaults. std and underline-color
are mandatory; calendar, macros and layout-cache are default adapter features.
Making underline-color mandatory prevents dependency feature unification elsewhere
from silently adding a source field that a conditionally compiled importer ignores.
Calendar time is pinned to the approved source lock's 0.3.55 / MSRV 1.88. Further
serde/palette/portable-atomic/scrolling/unstable forwarding profiles require explicit
verification. Raw upstream modules are available for composition, not a claim that
all source configurations or backend/session capabilities are integrated.

The actual production graph unifies dependencies with Matea's API auditor. It is
not the isolated reference-text graph: preserve that workspace's 43 frozen source
package versions and record every production/reference version difference. Exact
Git code plus a different lock graph is explicitly distinct evidence. Verify
licenses, active dependencies, source and three-OS/MSRV behavior before a verdict.
No image engine, codec/decoder or CUDA dependency is enabled by this adapter.

Normalize a complete source Buffer's origin to local (0,0). Validate content length,
allocation limit, every diff/skip flag, symbols, modifiers and whole-cell coverage.
Map all resolved named/indexed/RGB colors, underline color and representable SGR
flags. Covered blank continuation cells have no independent visible style.
Reject control/multiple/zero-width symbols, overlapped or clipped glyphs, unknown
modifier bits, contradictory blink flags and differing source/Matea width policies.
In particular Ratatui compensates for halfwidth-katakana sound marks; Matea's current
unicode-width policy differs. Return a typed error rather than silently shifting
cells. Raw Skip/AlwaysUpdate/ForcedWidth and this width-policy gap remain open
capabilities, not completed reference parity. Publishing no partial scene does not
promise rollback of arbitrary foreign widget state mutated before an error/panic.

Model::view remains read-only. Stateful rendering has a scoped &mut state helper,
and WidgetState<S> provides UI-local RefCell render caches for existing model APIs.
Events can update state via exclusive state_mut without a dynamic borrow. A render
uses try_borrow_mut and returns StateBorrowed on contention; only owned pixels are
returned. No Send/Sync bound, mutex or background runtime is forced onto model
state. Unwinding releases the borrow; generic state is not cloned/rolled back.
This is a documented render-cache boundary, not hidden business-state mutation.

Validate executable stateless/stateful catalog scenes, literal color/modifier
mappings, independent ownership, hostile raw buffers, zero/extreme geometry,
borrow/unwind behavior, structured composition and complete terminal output. Then
measure direct pinned rendering versus the full adapter path with matched outputs,
allocation/storage distributions and explicit conversion overhead. Retain failures
and regressions; broad raw exports cannot promote the gate without module/profile
coverage. All APIs remain provisional pending the Foundation owner gate.
