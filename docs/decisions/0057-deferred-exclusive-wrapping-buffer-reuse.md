# ADR 0057: Deferred exclusive wrapping buffer reuse

Status: implemented; measurements/qualification pending.

The first ten matched wrapping workloads run with complete equal outputs.
Initialized row count, scroll and borrowed iteration allocate zero times, but
width 4→8→4 rebuilds allocate five times while the reference allocates zero.
Keep the complete d2e0e3e baseline and address this specific lifecycle cost.

Retain an invalidated Arc in a private spare slot. Only the next geometry request
consumes it; multiple zero-offset width updates can coalesce without eager
rebuilding. The normal OnceLock reader still neither locks nor clones an Arc.
Create the optional mutex only after an initialized cache is invalidated. A
fresh/default/unused wrapping mode never creates or locks it. Geometry misses
take the spare and release the guard before doing any width/index work.

Arc::get_mut explicitly checks exclusive ownership. Reuse the Arc allocation and
clear/repopulate its Vec buffers only when unique. Otherwise build a new index,
preserving every other viewport clone's immutable snapshot. Borrowing display
rows prevents simultaneous mutable setters through ordinary Rust lifetimes;
there are no unchecked references, self-referential structs or unsafe code.
Only published geometry is shared by Viewport cloning; clones start with an empty
spare slot, including when cloned between invalidation and the next query.

Content replacement, clear_width and wrap-mode changes drop both active/spare
geometry. Plain width changes may retain previous peak capacity until one of
those operations. A poisoned spare mutex can be recovered: row buffers are
cleared and rebuilt before publication. No caller callback runs under the lock.

Preserve all 1,008 literal source/Matea observations and ownership/concurrent
clone/resize/empty-size tests. Measure the same original ten cases plus two
iterator controls in both orders, retaining any cold/layout/regression costs.
Zero warm reflow allocations do not imply zero first-use OS synchronization or
resident memory. Full module and physical-terminal performance remain open.
