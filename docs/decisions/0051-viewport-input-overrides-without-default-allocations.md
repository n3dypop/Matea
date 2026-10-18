# ADR 0051: Viewport input overrides without default allocations

Status: implemented input slice; provisional API.

Represent pager actions with a typed ViewportKey enum and a ViewportKeys map.
Default chords and help are immutable constant slices/strings. Store only
explicit per-action Binding overrides in a Vec, initially empty. This reuses the
existing binding matcher/help/enable semantics without allocating eight or ten
Vecs for every text viewport, or introducing a mutable global keymap. Set/reset
changes one action; disabled/empty overrides suppress matching and help together.
Chords and help are borrowed on inspection. Overrides clone with their viewport.

Match the pinned pager defaults in their documented precedence: page down/up,
half page up/down, down/up, left/right. Retain Matea's existing Home/End and
Shift-arrow/page/Home/End behavior through explicit additional default chords.
Repeat is a down event; release never matches. Custom modifiers match exactly;
Ctrl-U/D are enabled only by their documented bindings rather than by accepting
every command-modified key.

Focused wheel events use a configurable enabled flag and positive line delta
(default three). Shift-wheel and explicit horizontal wheel use the horizontal
step and require an explicit width. Zero delta/step is an ignored disabled
movement. Other mouse events and unfocused input are ignored. Ctrl/Alt/etc wheel
is ignored to leave command-modified gestures available to the application.
This is an explicit difference from the reference's unrestricted wheel modifier
handling; it does not imply physical mouse reporting in every terminal.

Consumed navigation preserves the existing Changed response at a boundary.
Focus ownership/hit testing remains with the parent component/runtime; the
viewport does not install subscriptions, terminal mouse modes or a global hook.
Test all default chords, precedence, disabled/empty overrides, exact modifiers,
release/repeat, focus, all four wheel directions and machine-limit counts.
Public reference input traces must be compared independently of View bytes.
