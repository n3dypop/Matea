# ADR 0060: Owned cell scenes and consuming frame transfer

Status: implementation/qualification in progress; API provisional.

ADR 0004/0016's widget bridge needs an owned scene boundary without an allocation
per imported cell or a borrowed foreign renderer in the runtime. Surface already
owns validated, sanitized whole-grapheme cells. Add an Arc<Surface> snapshot View
node and Canvas::surface cell composition. This is a text-cell surface, not an
image processing/rendering engine or a terminal graphics resource API.

View::surface accepts an owned Surface or an immutable shared Arc. Natural height
comes from the source; clipping never scales or splits a glyph. Source cells are
opaque, including styled blanks. Cursor metadata follows the existing cursor-node
layout/clip policy; the low-level Canvas operation copies cells only. Clones
retain immutable cells, and caller Arc::make_mut detaches subsequent mutation.
The node keeps View at 48 bytes and Cell at 40 bytes on the tested 64-bit host.

An immutable surface already satisfies glyph ownership. Trim clipped source edge
continuations, repair old destination glyphs touching both interval edges, clear
old cursors overwritten in the interval, then clone one contiguous row. Interior
old glyphs are entirely replaced. This avoids repeated segmentation and per-cell
erasure while preserving the existing painter's atomic wide-glyph damage rules.
No public raw-cell constructor or unsafe copy is added. Heap-backed graphemes
still clone their owned strings when cells are copied; zero allocation is not a
general snapshot claim.

Borrowed View::render clones an unconstrained, equally sized root snapshot without
initializing and repainting another blank grid. Add consuming View::into_surface:
Arc::try_unwrap transfers uniquely owned matching root cells, while shared roots
clone and constrained/composed scenes render normally. Runtime::frame already
owns the temporary Model::view result, so it consumes that scene. No mutable model
borrow or backend/session API changes. All geometry checks remain in force;
an equally sized existing Surface has already passed the allocation limit.

Seven renderer tests include 4,096 independent per-glyph/row-copy comparisons,
clipping extremes, complete glyph/style ownership, styled blanks, heap graphemes,
concurrent old snapshots, cursor order/layout and unique/shared/constraint cases.
A runtime/terminal test compares complete bytes across four color levels, Unicode
fallback and wide/narrow/empty transitions. Existing renderer tests pass. Full
native/Miri/three-OS qualification and performance distributions are still pending.
Pointer identity verifies the consuming transfer; it is not measured throughput.

This primitive does not import Ratatui raw buffers or implement Skip/AlwaysUpdate/
ForcedWidth. Those contracts require explicit adapter validation/implementation.
The approved Foundation and image/CUDA gates remain unchanged.
