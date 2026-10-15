# Matea Foundation architecture

Status: implemented experimental Foundation; three-OS acceptance and owner approval pending. Image/CUDA implementation is deferred until the Foundation gate.

```mermaid
graph LR
    app[Application Model] --> core[matea-core]
    app --> widgets[matea-widgets]
    widgets --> core
    core --> render[matea-render]
    render --> style[matea-style]
    widgets --> style
    terminal[matea-terminal] --> core
    terminal --> render
    terminal --> crossterm[Crossterm]
    facade[matea facade] --> widgets
    facade --> terminal
```

## Ownership

| Layer | Owns | Does not own |
| --- | --- | --- |
| Application | Domain state, typed messages, event routing and effect requests | Terminal state or escape strings |
| core | UI-thread transitions, eight effect slots, 32 completion envelopes, 64 subscriptions, generations, timers and focus order | OS input or terminal writes |
| render | Immutable View tree, geometry, grapheme cells, clipping, layering, row damage | Model mutation, OS terminal commands |
| style | Color/style/theme values, integer fixed/fill allocation | Terminal I/O or mutable global theme |
| widgets | Input/list/table/viewport/spinner/form state and Component implementation | Runtime internals or terminal session lifecycle |
| terminal | Exclusive session, Crossterm translation, signal shutdown, output batching, restoration | Domain state or layout policy |
| facade | Short imports and convenient run entry point | Additional state machine or duplicated contracts |

The Cargo workspace is the repository root; engineering documentation lives in `docs/`. All packages have `publish=false`. Facade and internal crate names have not been checked for registry or trademark availability. Distribution license and API stabilization require owner decisions before publication.

## Public authoring path

See the runnable counter at `crates/matea/examples/counter.rs` and the facade rustdoc. Implement `Model` with an associated Message; Update mutates `&mut self` and returns Effect, View receives size/theme and returns an owned View. Unit tests can create Runtime directly, dispatch input/completion/resize events, advance logical time, and inspect Surface without a terminal. Applications need no executor annotations, macros, terminal setup, or message downcasts.

The live loop reads one input per poll, dispatches it, advances timers, drains up to 32 completions and draws if state may have changed. The renderer suppresses unchanged rows. Update/View remain on the calling thread; only effect closures/messages must be Send. Update must not perform blocking I/O.

## Effects and shutdown

`Effect::perform(id, closure)` produces one typed result. `Effect::batch` starts independent tasks; result ordering is unspecified. `Effect::sequence` consumes one worker slot and emits child results in order; it does not wait for Update before starting the next child. The first child failure stops the sequence. Replacing or canceling an ID invalidates its generation, so queued old results are discarded. At eight workers, new work fails with RuntimeError; canceled workers still count until they actually finish.

Closure errors and unwinding worker panics become `Event::Failure` with the task ID, message and panic flag. An application's failure handler decides recovery. Runtime errors and transport/geometry errors end run and trigger terminal cleanup. On UI-thread panic the driver restores first, then resumes unwinding. The normal panic hook may print before restoration; no global hook is replaced. Cleanup errors are returned on ordinary success/error paths; during panic, cleanup is best effort and the original unwind resumes. Success is never inferred after an I/O restoration failure.

Cancellation is cooperative. `Cancellation::wait` is interruptible; blocking network calls require their own deadlines. Runtime Drop/shutdown cancels tasks and does not wait indefinitely. Unfinished user threads are detached and may outlive the Runtime. They must never own terminal resources or mutate model state directly. The narrow Unix sigaction module saves/restores prior handlers; applications must not replace those handlers concurrently during a live session. SIGKILL/abort cannot run destructors. See ADR 0003/0006 for limitations.

Subscriptions currently implement periodic UI-thread messages, keyed by stable ID. Reconciliation retains timing/closure for an unchanged ID; changing behavior/interval requires a new ID. Removing an ID stops it immediately. A late clock coalesces missed ticks, preventing a catch-up flood. Zero intervals, duplicate IDs and over 64 subscriptions are errors. Custom stream subscriptions and async executor adapters are future work.

## Rendering semantics

Columns use natural/fixed heights unless `fill()` is requested. Rows share available width unless fixed; gaps consume cells first. Fixed space wins over weighted fill. Integer remainder is deterministic from left to right. Text clips rather than wraps. Panels are opaque, ordered layers paint back to front, and all descendants inherit a clip. A wide grapheme is atomic: overwriting either half erases its whole old footprint (including a blank just outside the overlay clip if necessary). A cluster that cannot fit at the right edge is omitted entirely.

Unicode uses extended graphemes and unicode-width's default non-CJK ambiguous-width rule. Combining clusters and ZWJ emoji are preserved. Standalone zero-width clusters are omitted. Tabs become four spaces, newlines are structural, C0/C1 control/escape characters become replacement glyphs. No raw ANSI view is accepted. Surfaces reject over 1,048,576 cells. Resize invalidates previous dimensions; each changed row is reset/cleared and leading cells are explicitly positioned. Default blank cells need no writes after the clear. This prioritizes correctness over bandwidth optimization.

## Portability and extensions

The terminal backend trait is public and tested with failure-injecting mocks. Native input handles key press/repeat (release ignored), paste, focus and resize. Mouse, function keys, enhanced keyboard protocol, clipboard, suspend/resume, true inline interactive layout and screen-reader prompt sessions are not implemented. Color capability detection uses environment hints and supports explicit overrides; no graphics support is inferred.

See extension-contracts.md for the non-image component example and design-only future graphics ownership. Architecture ADRs 0003–0006 remain open to owner API review; no image/CUDA implementation exists.
