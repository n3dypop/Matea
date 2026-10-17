# ADR 0023: Indexed viewport content and explicit quality evidence

Status: provisional implementation; no public API freeze or F5 approval.

The owner requests evaluation and redevelopment of every implemented program.
Quality scores must distinguish tested behavior, pinned-reference parity and
measured performance; native green tests cannot stand in for the latter two.
Use an explicit program inventory and six independent ordinal assessments.
Missing programs/dimensions/evidence fail validation, and any applicable score
below 75 fails the separate quality gate. No average or automatic score inflation.

The new Viewport model tests expose lost trailing blank lines, CRLF replacement
and an obsolete offset after public `rows` grows. The pinned Bubbles
`c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
`viewport/viewport.go:225–264` uses split lines and normalizes CRLF. Preserve these
user-visible text semantics; deliberately clamp Matea's observed offset before
navigation after a resize. Other terminal controls still use Matea's safe text
policy. Rich Bubbles viewport features remain pending.

Store one owned sanitized String and UTF-8 line-start byte offsets. Build each
owned scene from a contiguous visible slice rather than cloning every line into
a temporary Vec and joining it. The offsets are private, constructed only at LF
boundaries, and the immutable String keeps them valid. Returned View still owns
its Text; no retained self-borrow or lifetime parameter reaches the application.
For example (also exercised by the dashboard and integration tests):

```rust
use matea::prelude::*;
let mut notes = Viewport::new("First tea\r\nSecond tea\n");
notes.rows = 2;
let scene = notes.view(&Theme::plain(), true);
let frame = scene.render(Size::new(24, 2)).unwrap();
assert_eq!(frame.plain(), "First tea\nSecond tea");
```

Theme/focus rendering, zero heights, clipping/Unicode and resize are checked
through the unchanged Component contract and plain/default dashboard snapshots.
There is no new escape-sequence responsibility for app authors. An alternative
shared mutable line cache would complicate ownership and invalidation; retained
Vec<String> storage would keep the per-line allocation cost. A rope has no
measured benefit for this immutable content API and is deferred.

The finite Rust-only probe shows fewer allocations and faster median construction
for 1/6/24 ASCII/Unicode rows. Keep initial parsing/resident storage unmeasured,
complete render cases and adverse p95 samples explicit. This is not a whole-module
or upstream performance verdict. See `quality-evaluation.md` and the linked raw
evidence; the independent external-widget example still validates extension
composition without runtime internals.
