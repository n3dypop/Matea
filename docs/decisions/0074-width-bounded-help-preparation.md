# ADR 0074: Stop help preparation at a proven width rejection

Status: implementation decision; public API unchanged, reference scope partial.

The short help capacity hint scanned every enabled binding even after reaching
its finite byte cap. Full help counted and prepared every row in a column before
checking its width. A clipped column could therefore allocate and sanitize
thousands of rows that cannot appear. The pinned Bubbles help implementation
also prepares the complete column before its width decision; this is a source
observation, separate from the already reviewed width overflow defect.

Keep the existing `Help::short_text`, `Help::full_text` and structured View APIs.
For example, `Help { width: Some(40), ..Help::default() }.full_view(&groups)`
continues to accept borrowed binding metadata and produce a terminal independent
View. There is no new runtime, transport or escape sequence responsibility.

Stop the short capacity estimate once it reaches the existing four bytes per
cell hint; return immediately for zero width. The byte hint is never a fitting
decision. Keep grapheme preparation and the actual width test unchanged.

While preparing a full column, accumulate maximum key and description cell
widths. Their sum plus the mandatory gap can only increase. Once it exceeds the
remaining column budget, discard that entire column and stop; a later row cannot
make it fit. The initial candidate delayed allocation until a row fit and capped
the initial capacity at 16. Its matched 1,024-row case increased allocation
requests from three to nine and requested bytes from 72,863 to 137,375. Retain
that candidate's complete three-cohort distributions rather than accepting the
clipped-input improvement as sufficient.

The revised candidate holds four prepared rows on the stack. If a fifth row fits,
count only the remaining enabled entries and allocate the exact final capacity
once, moving the prefix into it. Smaller accepted columns allocate once at their
actual row count. Early rejected columns allocate no item vector. A column that
fits five rows and fails later may have reserved the remaining count before its
rejection; no universal hidden-memory bound is claimed. All rows of a visible
tall column remain present. The fixed stack prefix uses safe Option values;
there is no uninitialized buffer or unsafe lifetime conversion.
Disabled rows do not contribute. Ellipsis placement, padding, style roles,
sanitization, clipping and resize behavior retain the same rendering contracts.

Alternatives: retaining the full scan wastes work on clipped datasets; caching
prepared columns complicates borrowed metadata invalidation and retains hidden
data; truncating rows would change visible height and alignment. No unsafe code,
cache, new public type or dependency is needed.

Validate different rows setting the two maxima, rejection of a whole prepared
prefix, unchanged invisible suffixes, disabled wide metadata and a 1,024-row
visible column. Measure ordinary, Unicode, clipped, tall, disabled and zero-width
inputs; retain regressions and allocation growth. Compare only equal-output
pinned source cases. Rich Lip Gloss decorators and whole-framework performance
remain separate unmet work; this change does not advance F5.
