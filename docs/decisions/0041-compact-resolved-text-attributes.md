# ADR 0041: Compact resolved text attributes without backend types

Status: implemented; public API remains provisional pending Foundation review.

The resolved Style currently carries foreground/background and three booleans.
Pinned Lip Gloss `19f011f1cc573e5d2bf9d7a9c4f14d429647759e`, `style.go`, exposes
faint/italic/strike/blink and six underline styles, including underline color.
Pinned Crossterm `9a397df6dc4f5bd3ff900bb6308b728186cb914c`,
`src/style/types/attribute.rs`, additionally exposes conceal, Fraktur, overline,
framed/encircled text and separate slow/rapid blinking. These capabilities are
currently missing. Their requested SGR states do not guarantee physical terminal
support or a particular font appearance.

Keep a resolved, Copy cell style separate from future rich text/layout decorators.
Add a private u16 TextAttributes representation with typed TextAttribute flags,
UnderlineStyle, BlinkStyle and TextFrame choices. Underline, blink and frame
choices are exclusive; setters cannot construct invalid numeric combinations.
No raw bits constructor, backend dependency, allocation or unsafe conversion is
needed. Italic and Fraktur are independent requested flags as in the reference;
their visual interaction remains terminal-specific.

Add underline color and adapt it with the same explicit color policy. Preserve
existing bold()/underline()/reverse() builder call sites. Provisional public
boolean fields become typed attributes; direct reads/assignments migrate to
contains/setter operations. This is an unpublished API change to review before
publication, not a compatibility or registry-availability claim.

Proposed widget usage (to become an executed example before this ADR is accepted):

```rust
fn view(&self, context: ViewContext<'_>) -> View {
    let style = context.theme.accent
        .italic()
        .underline_style(UnderlineStyle::Curly)
        .underline_color(Color::Indexed(10));
    View::styled(format_args!("Count: {}", self.0), style)
        .panel("Matea attributes", context.theme)
}
```

The example must preserve keyboard focus, resizing and plain/color fallback.
Reduced-motion terminal output suppresses hardware blink requests. Existing
themes, basic styles and independent widget integration must keep working.

Appending booleans/choice fields can increase every render cell's stride; exposing
Crossterm's attribute type would couple independent renderers to one transport
and mix resolved state with commands that disable/reset attributes. A private
packed representation avoids both. Aim to retain the existing cell stride on
the three supported 64-bit hosts; measure actual Style/Cell layout rather than
assuming the compiler's representation or promising a stable ABI.

Before changing the implementation, commit logical-output probe fixtures and
capture style/scene/frame distributions, allocation counters and representation
sizes. Normalize the legacy style probe's expected output independently of
derived Debug field names, then retain that new baseline. Test the finite typed
state space, reference SGR bytes, disabling/reset transitions, underline colors,
clipping, Unicode, motion preferences and all three OSes. Retain adverse results.

This slice does not complete Lip Gloss's transforms, layout, borders, inheritance,
space-specific decoration or every referenced public capability. Those remain
pending in the same Foundation scope and are not promoted by adding attributes.

Implementation at 4d1a397 retains 40-byte render cells (Style grows from 11 to 14
bytes on the measured 64-bit host). Attribute-only changes no longer evict a
palette result: the bounded cache keys foreground/background/underline colors
and reattaches the caller's attributes. This preserves ownership without a global
map or allocation. Style/TextAttributes are must-use values, so accidentally
discarded setters warn. Native 13,824-state construction and SGR tests, targeted
Miri, grapheme/clipping, reduced-motion repaint and interactive example checks
are recorded in [the audit](../text-attributes-audit.md).

[Measurements](../evidence/attributes/report.json) retain 157 legacy controls and
171 before/after cache controls. Two legacy theme Debug strings use an explicit
default-field transformation; the other 155 outputs match raw bytes. A balanced
14-case repeat retains every sample, including mixed truecolor tails. Short
panels request 20 more bytes due to their retained complete Theme; some legacy
costs regress. Neither module parity nor maximum performance is established.
