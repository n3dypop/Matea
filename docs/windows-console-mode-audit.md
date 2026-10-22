# Explicit Windows console modes and promoted source fields

Status: qualified at 04e81bc for the stated three-OS fixture scope.
The prior c526cf4 checkpoint and intermediate 95dd7b7 CI remain separate.

Pinned x/term State embeds private state. The Windows implementation has exported
Mode uint32, which is promoted to State.Mode. Its private embedding does not make
the promoted selector inaccessible. An original source caller compiles reads,
writes and XOR for Windows, while the same caller fails on Linux/macOS, where
the member does not exist. [Compiler results](evidence/terminal-device/promoted-mode/context.json)
bind the exact caller and fixed revision; no native execution is inferred from
the cross compile. Thirteen raw declaration candidates are not a resolved export
graph, and the inventory count is not silently replaced by compiler output.

The portable TerminalState stays opaque. On Windows, device::windows exposes
ConsoleMode, mode and snapshot. Bitflags have named input/output constants;
unknown bits are retained, and a constructed snapshot performs no native IO.
Flags can share bit values across input/output roles. The OS validates them for
the actual handle when TerminalDevice applies the state, with original restoration
still owned on success, error and Drop. Snapshot construction certifies a value,
not a valid console configuration. The existing pinned bitflags version is reused;
the lock changes only by adding the direct terminal dependency edge.

An application that deliberately disables Quick Edit can use the advanced API
without numeric file descriptors or a separate Windows bindings dependency:

```rust,ignore
use matea::terminal::device::{TerminalDevice, windows::{ConsoleMode, mode, snapshot}};
let input = std::fs::OpenOptions::new().read(true).write(true).open("CONIN$")?;
let mut console = TerminalDevice::open(&input)?;
let mut flags = mode(&console.state()?);
flags.insert(ConsoleMode::INPUT_EXTENDED_FLAGS);
flags.remove(ConsoleMode::INPUT_QUICK_EDIT);
console.set_state(&snapshot(flags))?;
// Use the borrowed console within this guard's lifetime.
console.restore()?;
```

This snippet is explicitly Windows-only illustrative code, not a claimed
executed doctest. The executable device_probe fixture exercises the actual API
on disposable consoles. Its new checks toggle input echo and output wrap,
independently query the resulting state, restore both roles, and reject
input-only operations on an output handle. A Windows pure test covers every low
word, each bit/complement and unknown-bit edits without IO.

[04e81bc CI](https://github.com/n3dypop/Matea/actions/runs/39495143723) succeeds in
all seven required jobs. [Independent artifact verification](evidence/terminal-device/ci-04e81bc/source-bindings.json)
binds all 198 execution inputs, 47 acceptance commands per host, and the nine
isolated Windows console fixtures in ordinary and instrumented binaries. Input
echo/output wrap edits, role rejection and mode restoration execute inside every
fixture. Six reopened portability assessments and device behavior return to 75;
device reference/performance remain 50. This does not qualify physical emulator,
IME, font or remote transport behavior, nor any shared-runner timing.

The Foundation gate now independently requires a boolean Go target/export review.
Missing metadata is pending, and strings/numbers/other truthy values fail closed.
One synthetic ledger closes every other condition and proves that the Go review
alone can block entry. Another rejects malformed values in normal and optimized
Python execution. The project flag stays false until relevant promoted, embedded
and returned-member exposure is resolved across target profiles. This strengthens
the existing whole-public-feature requirement; it does not lower a threshold or
infer that a raw candidate list is complete.

See [ADR 0071](decisions/0071-promoted-go-state-mode-and-explicit-windows-api.md)
and [the retained terminal-device qualification](terminal-device-audit.md).
