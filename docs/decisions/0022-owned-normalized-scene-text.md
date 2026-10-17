# ADR 0022: Owned normalized scene text with inline storage

Status: implementation decision; public API provisional and final API review open.

View text/title currently owns String, including short literals and clones. The
initial Rust-only probe measures one allocation for short leaves/clones/counter
formatting and four for a short panel. Capture a committed baseline with both
construction and complete build/render workloads before changing this boundary.
Construction alone can mislead if normalization merely moves between phases.

Introduce an immutable Text value with private CompactString storage, using the
already locked compact_str 0.9.0 dependency. Normalize tabs to four spaces and
C0/C1 controls to replacement characters, preserving structural newlines and the
existing renderer policy. Constructors own their result; short values can be
inline and safe long owned String buffers can be reused. No mutable string access
is exposed. Clone preserves the normalized invariant. It is display content with
the documented normalization, so application editing state can remain String.

Accept Text through View::text/styled/panel. Support ordinary borrowed/owned
strings, Cow/Box forms, CompactString and std::fmt::Arguments. Format Arguments
synchronously into the owned value, avoiding an intermediate String for short
dynamic text. Formatting errors panic consistently with format!, before a partial
Text is returned; terminal cleanup on view panic remains the Runtime contract.
Keep all implementation safe Rust. Display/AsRef/Borrow/Deref expose immutable str.

For example, the existing executable counter becomes:

```rust
fn view(&self, context: ViewContext<'_>) -> View {
    View::styled(format_args!("Count: {}", self.0), context.theme.accent)
        .panel("Matea counter", context.theme)
}
```

This changes provisional constructor bounds from Into<String> to Into<Text>.
Standard string call sites remain supported, but a custom type implementing only
Into<String> must explicitly convert to String or implement Into<Text>. Record
this in API review; do not claim source compatibility for every custom conversion.
There is no published/stabilized API or name-availability claim. A facade reexport
avoids requiring application authors to use the storage dependency directly.

Keep Canvas::text as the untrusted-string drawing boundary. Share one normalization
policy with Text and provide an internal typed path for an already normalized
leaf, avoiding repeated control scans on reused/cloned scenes. Input focus,
Unicode width, clipping, native cursor and style/color adaptation stay in their
existing layers. Panels and their children still use theme tokens. Validate both
focused independent widgets and resized/color-fallback frames; this is not a rich
ANSI parser or completion of Lip Gloss/Ratatui style capabilities.

Compare short/long/control/Unicode construction, clone, complete render, formatted
counters and panels with output checks. Preserve regressions, allocation counts,
requested bytes and p50/p95. Normalization now occurs at construction, so report
that tradeoff and full-pipeline results. Large strings may still allocate; this
does not promise zero allocation for an application or full reference parity.

References: local immutable View/Canvas contracts; pinned compact_str 0.9.0
src/lib.rs (From<String>, From<Cow<str>>, fmt::Write), crate checksum in Cargo.lock.
The storage dependency predates this change; no additional crate is introduced.

A first normalized implementation regressed the long constructor from 28.02 to
109.75 ns because validation moved earlier. Retain its source patch/hash and all
samples. A safe byte scan folds ASCII control/high-bit flags and checks C1's exact
UTF-8 pair range; exhaustive Unicode-scalar tests verify equivalence to the
standard control policy. It measures 86.01 ns for that constructor in the next
run. Longer clone and constructor regressions remain explicit; do not turn lower
allocation counts into a universal CPU claim. Full pipeline cases are reported.
