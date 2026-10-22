# Prepared scene text

Text is an owned immutable value exposed through `matea::prelude`. View text,
styled text and panel titles accept ordinary strings and Text. Short values can
use inline storage; the exact storage threshold is not a stable public contract.

```rust
use matea::prelude::*;
let theme = Theme::default();
let count = 42;
let view = View::styled(format_args!("Count: {count}"), theme.accent)
    .panel("Matea counter", &theme);
let label = Text::from("抹茶\tready");
assert_eq!(label.as_str(), "抹茶    ready");
let cloned_scene = View::text(&label).clone();
```

`format_args!` is consumed synchronously and its result is owned; no application
borrow or formatting closure remains in the scene. Short dynamic text can avoid
an intermediate String. Existing `format!` calls remain valid. The executable
counter, facade doctest and independent-widget crate demonstrate the new form
with existing theme/focus contracts.

Tabs become four spaces, C0/C1 controls become U+FFFD and newlines remain
structural. Other Unicode is preserved. Formatting raw arguments precedes that
normalization; formatting an existing Text uses its already normalized contents.
Unicode width, grapheme clipping, native cursor composition and color fallback
remain rendering/transport policies. No ANSI escape parsing is introduced.

Constructors accept str/String, common borrowed forms, Cow, Box<str>,
CompactString, char, Text and fmt::Arguments. Safe owned long String buffers can
be reused. A failed custom Display implementation panics consistently with
format!; it does not return partial Text. Unit and Unix PTY tests cover this path
and terminal cleanup. Immutable str access, Display and conversion into String
are available; there is no mutable access to prepared contents. Editing state can
remain String and be converted when a view is built.

The provisional View constructor bounds change from Into<String> to Into<Text>.
A custom type implementing only Into<String> must explicitly convert, for example
`View::text(String::from(custom))`, or implement Into<Text>. This remains part of
the open public API review. No package is published.

Normalization now happens during construction. That can make an isolated long or
control-containing constructor slower while reused-scene rendering avoids repeat
scans. Construction, clone and complete rendering are measured separately in
[the performance report](view-performance.md), including regressions. Surfaces,
large strings and panel containers can still allocate.

[ADR 0022](decisions/0022-owned-normalized-scene-text.md) records alternatives,
ownership/error semantics and the API tradeoff. [Tests](../crates/matea-render/tests/text.rs)
compare normalized output, every Unicode scalar's control policy, immediate
formatting and complete cells/styles under multiple sizes and focus themes.
