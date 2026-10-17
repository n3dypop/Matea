# Upstream issue analysis and applicable improvements

This is an initial, evidence-linked analysis. `coverage/upstream-issues.json`
contains all 841 **open reports** returned by the scoped repositories at collection
time, excluding PRs. Closed reports are not enumerated. Source scanning additionally
found 117 TODO/FIXME/XXX/HACK/skip locations. Neither count is a confirmed-defect
count. The remaining reports and module-level performance work remain pending.

The [input audit](input-audit.md) adds analysis of Bubble Tea #1865 and Crossterm
#1133. NUL is represented as a non-text key; explicit Kitty configuration avoids
the unbounded startup query path. Native Windows parsing and the upstream hang
have not been reproduced. New regression tests cover release-safe widgets,
modified-Tab focus, Form submission state and independent terminal-mode cleanup.

| Evidence | Analysis and Matea response | Verification / remaining work |
| --- | --- | --- |
| [Huh Select filter](https://github.com/charmbracelet/huh/blob/ffb6a97195b4a031925a31cc70c62fc1d309e69d/field_select.go#L749) | Source explicitly identifies missing diacritic/custom filter policy. `List::filter_with` permits caller-defined matching. Default lowercase substring matching caches immutable search terms and reuses visible-index storage. | `custom_filter_and_keys_share_enabled_help` exercises café/cafe normalization; full locale/case folding is not promised. Filtering performance remains unmeasured. |
| [Huh Select keymap](https://github.com/charmbracelet/huh/blob/ffb6a97195b4a031925a31cc70c62fc1d309e69d/field_select.go#L419) | Source identifies hardcoded key names ignoring custom mappings while filtering. Matea bindings own their match keys, enabled state and help metadata; List uses these bindings rather than a second hardcoded match. | Regression remaps j to n and disables it, including help metadata. Full editable Huh Select interaction remains pending. |
| [Harmonica Euler update](https://github.com/charmbracelet/harmonica/blob/0ff76e405449b47bd60f1df4820d880081692d3c/projectile.go#L75) | Position uses the previous velocity, so constant-acceleration displacement depends on step size. Preserve this behavior in `Projectile::step`; opt-in `step_exact` includes half the acceleration term. Nonfinite results fail without mutating state. | Analytical trajectory and 1×1s versus 100×10ms partition regression. This is numerical-method analysis, not an upstream bug claim. |
| [Harmonica example report #13](https://github.com/charmbracelet/harmonica/issues/13) | Report concerns missing native OpenGL/GLFW example dependencies. Matea motion has no dependencies and its terminal example uses the existing portable facade. | Motion example snapshot and three-OS build; no OpenGL example is supplied. Upstream failure not reproduced locally. |
| [Bubble Tea #1848](https://github.com/charmbracelet/bubbletea/issues/1848), [pinned view comparison](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/cursed_renderer.go#L886) | Pinned source directly compares interface-valued colors. Go dynamic incomparable values can panic. Matea's closed, value-based `Color: Eq` eliminates this class of dynamic comparison, and rendering occurs under the session cleanup boundary. | Existing style/diff and panic-restoration tests; no equivalent arbitrary color interface exists. Go panic example not executed here. |
| [Bubble Tea #1850](https://github.com/charmbracelet/bubbletea/issues/1850), [initialization](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L1064) | Report identifies early returns after terminal mode changes. Matea installs a session guard before enter, restores on partial-enter/draw errors and unwind, and tracks each acquired mode. | Failure-injection regression plus Unix PTY lifecycle tests. The report's resource-limit reproduction and Windows native console remain unverified. |
| [Bubble Tea #1857](https://github.com/charmbracelet/bubbletea/issues/1857), [renderer ticker](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L1460) | Report describes input echo waiting for frame ticks. Matea's synchronous driver draws after input dispatch without a render ticker, avoiding that specific scheduling wait. This does not bound slow application Update/View work. | Loop inspection and mocks; real keystroke latency comparison remains pending. Do not adopt the issue author's timing numbers as Matea results. |
| [Ultraviolet cursor TODO](https://github.com/charmbracelet/ultraviolet/blob/6b8d4baf91b4c933e6613b0eefc8f180f6eda002/terminal_renderer.go#L1114) | Optimization comments do not imply Matea is faster in every resource metric. Initial one-cell trace emitted 37 bytes in Matea and 2 in Ultraviolet. Added an exclusive-writer FrameEncoder with cursor/style caching and explicit invalidation. | Independent replay verifies both screens; optimized stateful trace emits 2 bytes in each. Other workload bandwidth remains pending. |

Rust-specific improvements already implemented: inline common cell graphemes,
borrowed safe text, no per-character sanitization allocation, a borrowed damage
iterator, immutable-frame identity shortcut, reusable output buffer and one stdout
lock per frame. The performance report limits these observations to measured
fixtures. Long combining clusters, controls, wide-to-narrow updates, sparse ASCII
styles and zero-size resize have regressions. No unsafe code was added to the
production renderer; allocator instrumentation is confined to the probe binary.
Typed key binding comparison also removes per-event string formatting for the
six measured shortcut fixtures, while preserving repeat and ignoring releases.
It does not establish full Bubbles key module parity.

The [motion/pagination audit](motion-pagination-audit.md) records executed source
edge cases for stale ranges, zero divisors and clearing data, plus extreme spring
root cancellation. These are distinguished from maintainer-confirmed issues.
Private page invariants, NonZero sizes, active-dot windows and transactional motion
updates have regressions. Exact dot capacity and inline checked arithmetic are
measured against retained pre-optimization samples.

The [input editing audit](input-editing-audit.md) records pinned reproductions of
Bubbles #1056, #950, completion limit/validation bypass, stale matches, empty
selection panic and NUL placeholder output. The older #562 adaptation did not
reproduce. #906 native CJK cursor misplacement and an adapted #1001 scroll-offset
case are reproduced; Matea now composes native position/visibility from the same
cell viewport as its text rendering. Shape/color/blink and real emulator checks
remain pending.

[ADR 0014](decisions/0014-composed-cursor-and-pure-encoding.md) also separates pure
frame encoding from Crossterm's conditional WinAPI command execution. A Vec sink
now receives ANSI bytes on every OS; mode acquisition/restoration still uses the
actual transport. Regression tests exercise a command that would panic if the
Windows console path were invoked, and preserve original I/O errors. Known same-axis
cursor moves use 4 bytes instead of 6 in the measured fixture. Unknown positions
reanchor absolutely; the later next-row CRLF case is described below. This does not
establish legacy console support.

The [frame color audit](frame-color-audit.md) records a Matea integration defect:
Crossterm's global NO_COLOR policy overrode an explicit frame color capability.
Direct value serialization fixes deterministic output without changing global state.
A bounded per-writer palette cache avoids repeated per-cell search and the reference
colorprofile's growing process-global map. Four matched profiles include retained
cache-only regressions and their direct-path correction. All four now measure zero
allocations; ANSI256 p50 changes from 1.58 ms to 28.78 us in this one memory-sink
fixture. A follow-up found the CRLF shortcut could scroll inside existing vertical
margins. The retained failing trace reproduces it; vertical changes now use absolute
CUP, including a shorter default-column form. This saves 41 bytes relative to the
original frame baseline, and nondefault margins are replayed independently. Other workloads and real terminals remain
pending; this is not full terminal/style parity.

The [key help audit](key-help-audit.md) executes the pinned width-overflow branch
(four cells at a one/two-cell limit), adds whole-item fitting, and assesses #571's
unstyled key-description gap. Independent gap styles have cell regressions in both
modes. Borrowed metadata, retained fragment widths and caller-owned output avoid
unnecessary key cloning and repeated width work. Plain-string fixtures agree;
the safety-scan timing regression and subsequent width-cache revision are retained.
Arbitrary rich style decorators and full module parity remain pending.

The [clock audit](clocks-audit.md) records executed Bubbles default-zero interval,
duplicate initial tick, stale pause/resume tick, multiple-start scheduling and
negative overshoot observations. Monotonic elapsed values, positive intervals,
generations and one filtered timeout outcome address these classes with tests.
Five related reports were assessed; the old real-machine timing figures were not
reproduced. Fresh duration formatting still trails source despite fewer growth
allocations, and active scheduling comparisons remain pending. Core's logical
Duration::MAX rescheduling boundary also has a regression and checked deadline
representation. These results do not close clock module performance.

Spinner source observations reproduce initial zero-tag replay (`/` then `-`),
zero-ID broadcast, empty/shortened frame error output and an accepted zero period;
a stale positive tag is correctly rejected. Matea validates animation data, resets
phase on replacement and reuses generation/timestamp filtering. Bubbles #999 is
actually a Huh title-spacing report; source inspection and Matea label regressions
are recorded separately, without claiming an executed Huh reproduction. See
[spinner audit](spinner-audit.md) and its pinned harness/fixture evidence.

An additional Matea defect was reproduced: a duplicate subscription ID discovered
partway through reconciliation removed an existing timer before returning an error.
Restoring valid configuration delayed that timer's original deadline. The saved
[known-negative log](evidence/subscription-atomicity-before.log) loses callback a;
current regressions retain a and b and cover zero-period/limit errors. Full
validation before mutation and retained map entries fix it. This is a Matea defect,
not an upstream attribution. [ADR 0021](decisions/0021-scoped-subscription-identities-and-retained-timers.md).
