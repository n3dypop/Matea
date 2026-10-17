# ADR 0024: Exact bounded color quantization

Status: implementation decision, no public API change or Foundation acceptance.

The owner requests repeated measurement and redevelopment of the individual
quality deficits before whole-framework optimization. Public style primitives
previously had no dedicated measurement. The new 46-case `--style` probe covers
semantic colors/profiles, foreground/background attributes, varying colors,
layout sizes and theme construction. Baseline commit `581560b` is retained.

Matea already specifies minimum **squared RGB distance** to the canonical xterm
palette, selecting the first index on a tie. The former ANSI256 path scans all
256 entries. Keep this contract, API and configurable-terminal-palette caveat.
Use a 16-entry ANSI-prefix search plus at most one cube and one gray candidate:

* Squared distance separates by channel on the 6×6×6 cube. Select the nearest
  value from [0,95,135,175,215,255] independently, with lower-channel midpoint ties.
* On the 24 gray entries v=8+10k, the unconstrained minimizing k is
  (r+g+b-24)/30. Nearest-integer selection with lower-index ties and [0,23]
  clamping chooses one candidate. All intermediates fit u16/i32.
* Inspect the ANSI prefix first and replace only for strictly smaller distance.
  Black/white duplicates keep indices 0/15, and cube/gray ties keep cube indices.

This is safe Rust with constant local state, no new dependency, global map or
lock. The independent test constructs its own full palette and search; it does
not call private production palette/distance/candidate helpers. Native boundary
tests cover profiles, all indexed inputs, attributes and midpoint neighborhoods.
The explicit release exhaustive test checks all 16,777,216 RGB triples. Its
command/result and targeted Miri execution are recorded separately from normal
unit tests; an ignored-by-default test is never claimed executed without that run.

Alternatives: retain the 256 scan, introduce a color cache, use a lookup table, or
change to perceptual distance. A cache needs a memory/eviction contract; the frame
encoder already has an eight-style cache, and varying colors still need efficient
miss handling. Large lookup tables add storage and initialization cost. Changing
the metric would change visible colors and is outside this optimization.

The pinned x/ansi `faa4adf95555331dfb439b716cc83767ecdadeb4`, `ansi/color.go`
`Convert256` compares cube/gray candidates using HSLuv and excludes the ANSI
prefix. Its `Convert16` maps through the indexed palette. These are different
policies, so the Rust-only before/after timings are **not** a speed ratio against
Charm. The corresponding reference-equivalence assessment remains partial.

Existing counter/dashboard/external-widget examples continue using
`View::styled(..., context.theme.accent)` and `Theme::plain()` without new
boilerplate or escape sequences. Resizing/focus/monochrome/16/256/truecolor tests
and independent terminal traces validate unchanged rendering contracts. The
46 measurements include unchanged paths and retain regressions/noise; they do
not assert universal maximum performance or open the whole-framework stage.
