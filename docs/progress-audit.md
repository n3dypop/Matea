# Progress component source audit

Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 exposes 35 progress declaration
candidates, with no file-level platform/build conditions. All candidates now
link to bounded Rust equivalents. This is partial feature coverage; new source
CI, distributions and remaining color/style/lifecycle conditions are pending.
See [the declaration ledger and source hashes](evidence/progress/source-review.json)
and [ADR 0046](decisions/0046-generation-safe-themed-progress.md).

| Reference declarations | Rust contract | Correctness |
| --- | --- | --- |
| ColorFunc, WithColorFunc | ProgressFill::ColorFn, one active owned policy | 120 real ViewAs cell/color fixtures; callback half/full cells |
| Three Default*Char constants, WithFillCharacters, Full, Empty | Three PROGRESS_* constants, validated set_glyphs | Literal glyph fixtures; wide, zero-width and control rejection |
| Option, New, Init, Model | Owned Progress, Default/new/new_at; composed Component | Runtime subscription, immutable owned View and identity tests |
| WithDefaultBlend, WithColors, FullColor | Theme/Solid/Gradient fill enum; explicit Gradient::new | Solid, default two-stop RGB8, half/full, scaled/unscaled fixtures; additional stop counts share independently tested Gradient |
| EmptyColor, PercentageStyle | Optional semantic-token ProgressStyles | Literal foreground/background fixtures; custom-style tests |
| WithoutPercentage, ShowPercentage, PercentFormat | show_percentage; Rust fraction formatter closure | Hidden/default/custom label, sanitation, grapheme/narrow clipping |
| WithWidth, SetWidth, Width | u16 set_width/width; owner handles resize | Zero/narrow widths, clipping and executable responsive example |
| WithSpringOptions, SetSpringOptions | Cached matea-motion Spring, validated set_spring | Negative-velocity crossing, finite/positive-frequency validation |
| WithScaled | Gradient enum scaled flag | Full RGB8 cell/color fixture outputs |
| FrameMsg, Update | ClockTick, schedule/subscription/update | Foreign, stale, duplicate, backward and coalesced observations; runtime cancellation |
| Percent, SetPercent, IncrPercent, DecrPercent | target/motion, set_percent[_at], increment/decrement | Clamping, invalid-input rollback, retarget generation invalidation |
| View, ViewAs | Borrowed ProgressFrame/cells and owned View/view_as | All 120 reference fixtures; exact/double-ended/fused iteration; reduced motion |
| IsAnimating | Clock running state; absolute distance AND velocity, exact snap | Reproduced signed-velocity regression, exact final rest and no schedule |

Rust intentionally uses theme defaults rather than hard-coded Charm brand colors.
Tests explicitly configure reference colors before claiming equivalence. Public
styling is composed with Matea Style, not the whole Lip Gloss layout API copied
into the widget. Go formatting signatures are replaced with a closure receiving
a fraction; nonfinite input, unresolved default colors and invalid geometry are
errors. Narrow labels are clipped; reference labels may overflow their width.
Finite out-of-range fractions clamp before callback invocation. The spring
advances one fixed 60 Hz step for each accepted later tick, matching reference
cadence; it does not perform elapsed-time catch-up. This limitation is explicit.
Zero damping is allowed as continuous oscillation; zero frequency is rejected.
The live widget has no Clone implementation. Neither views nor color sampling
start tasks or write escape sequences.

## Reproduced negative-velocity rest

Reference IsAnimating tests distance <0.001 and signed velocity <0.01. A public
probe with frequency 18, damping 0.1 and target 0.5 stops at frame 142, displaying
0.50088622662 after 0.50300658651. Observed backward finite-difference speed is
about -0.1272 per second. No private fields or altered sources are involved.
[Exact source probe and retained observation](evidence/progress-source-rest/context.json).
The finite difference is observed through View; source review identifies the
signed comparison. Its first missing-ANSI-strip parser failure remains retained.
Matea's regression encounters a near-target crossing with velocity < -0.01,
continues, eventually snaps to exactly (position=0.5,velocity=0), and cancels its
schedule. This bounded fix is not proof of every spring/scheduling condition.

The first new cell fixture test also found a Matea defect: default equal-fill
row layout gave the label the wrong width, dropping the percent sign. Explicit
widths now cover solid/empty runs, each colored cell and the label. The actual
before/expected observation is retained in the source ledger.

## Upstream issue review

Read issue bodies through GitHub's API; titles alone do not establish a defect
in the pinned version. [#278](https://github.com/charmbracelet/bubbles/issues/278)
reports width shrink on Windows with a 2022 example. Matea validates one-cell
glyphs, uses clipped cell geometry, and has a configurable ASCII fallback.
Native Windows terminal rendering and that historical GIF are not reproduced;
this report remains partial, not marked fixed.

[#143](https://github.com/charmbracelet/bubbles/issues/143) describes old
WithGradient string inputs rejecting ANSI indices and falling back to black.
Those API names differ from the pinned v2 reference. Matea adds explicit
Gradient::from_xterm_colors for mixed indexed/RGB stops, without parsing hex
strings. All 256 indexed starting colors are compared with eight samples from
pinned Lip Gloss/x/ansi. Unknown default terminal colors are rejected; actual
ANSI16 palettes can be customized by terminals, so callers needing exact local
colors must supply resolved RGB stops. Full alpha/high-depth color contracts
remain pending. Rendering uses the existing None/16/256/truecolor fallback.

[#395](https://github.com/charmbracelet/bubbles/issues/395) requests optional
fractional block geometry, distinct from half-block color resolution. Current
Matea preserves reference whole-cell fill rounding. Fractional occupancy and
darkened empty gradients remain an enhancement opportunity, not an implemented
or tested capability. There are no progress-local TODO/FIXME markers in this pin.

## Measurement contract and limits

Six new cross-language workloads start with an existing configured component,
render fraction 0.625 at widths 40/80/200 into a complete styled row, and consume
every UTF-8 symbol and foreground/background color using FNV64. Go includes
public ViewAs plus the pinned Ultraviolet ANSI-to-cell parser; Rust includes
structured View plus Surface rendering. Exclude terminal I/O, animation and
component construction in both. All complete outputs, semantics and first
returns must match. Six Rust borrowed-cell controls omit View/Surface
materialization and cannot receive an upstream speed ratio.

Use 20 ms calibrated batches with three confirmations and all 31 positive
batch-averaged per-operation values. Preserve reference/Matea/Matea/reference
order, actual iteration counts, cold calls, allocation accounting differences,
and adverse tails. Timing is pending; no performance score is promoted. Full
terminal output, resize/scheduling/error/constructor paths, RSS and native
Windows/physical-emulator behavior remain unresolved. Stage A, Stage B and F5
remain closed while the complete ledger has unmet requirements.
