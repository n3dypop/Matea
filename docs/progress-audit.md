# Progress component source audit

Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 exposes 35 declaration
candidates in progress. Matea does not yet implement this component; no parity
or module score is awarded by this audit. The retained inventory still records
the complete pending module, rather than treating this one finding as its scope.

The reference stores target, visible position, signed velocity, a cached
Harmonica spring and per-instance/tag identifiers. SetPercent invalidates older
target commands; Update checks identity/tag and advances one fixed spring step.
Public ViewAs supports static display; View displays the animated state.
Fill supports solid color, multiple CIELAB gradient stops, scaled gradients,
callback colors and foreground/background half blocks. Percentage formatting,
styles, width and fill glyphs are public configuration. Equivalent Rust
composition must preserve those user functions before this module can close.

## Reproduced negative-velocity rest

IsAnimating tests distance <0.001 and **signed** velocity <0.01. A sufficiently
negative velocity therefore passes the velocity condition. A public-API probe
sets spring frequency 18, damping 0.1 and target 0.5, then executes each returned
animation command and Update until IsAnimating returns false. At frame 142,
the displayed value is 0.50088622662, following 0.50300658651. The observed
backward finite-difference speed is about -0.1272 per second, while the component
reports rest. No private fields, unsafe reflection or edited reference sources
are used. Formatted output is stripped of ANSI before parsing; the first probe's
missing-strip parser failure is retained separately and is not an upstream bug.

[Pinned source/module checksums, exact probe and observation](evidence/progress-source-rest/context.json).
The finite difference is an observation through View, not direct access to the
private velocity. Source review identifies the signed comparison as its cause.
This is one native Go execution, not a cross-platform or performance result.

Matea's future progress state must use velocity magnitude as well as distance,
reject nonfinite percentages/spring parameters, and snap to the target when
settled. Regression tests must include this negative-velocity crossing and
stale/foreign/duplicate tick rejection. Reuse the independent motion crate and
existing typed clock/subscription contracts, with non-cloneable live identity.
Configuration and view ownership must remain independent of runtime internals.
Theme, reduced motion, clipping, resize, color fallback and literal output
fixtures remain required implementation work; this audit does not satisfy them.

Lip Gloss blending also needs a separate audit: Blend1D uses CIELAB segmented
interpolation and an early prefix shortcut, while filtering nil colors can
mutate the caller's Go slice. Rust should own or borrow immutable validated
stops and measure reusable interpolation separately from styled bar rendering.
No gradient equivalence, reproduced nil-stop failure or Matea fix is claimed yet.
