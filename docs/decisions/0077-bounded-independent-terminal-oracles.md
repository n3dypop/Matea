# ADR 0077: Bounded independent terminal replay and range erasure

Status: accepted for the Foundation test tools; no framework API change.
Date: 2026-10-22.

The below-high review found a concrete scaling defect in both independent ASCII
replayers: `EL` visited every screen cell for an operation confined to one row.
The colored replayer also accepted incomplete/out-of-range RGB values, or raised
an incidental `IndexError` instead of rejecting an unsupported stream. Absolute
cursor addresses were not clamped, and `ECH` at the delayed-wrap position erased
nothing. These defects weaken the tools used to check production output.

Use row ranges and list slices for `EL`, `ED` and `ECH`. The ASCII and colored
models keep their own implementations and remain independent of Rust encoders.
Blank color cells share immutable tuples, never mutable row lists. Erasure keeps
the current foreground/background/attributes. Absolute addressing clamps to the
grid; erasure at the pending-wrap cursor acts on the last column. A separate flat
cell model tests all positions/modes in three finite grids, including one column,
one row and the delayed-wrap position. Existing nondefault-scroll-margin and
Unicode-fallback regressions remain required.

Command arities and the `ED`/`EL` modes and default `ECH` count are checked against
the primary [XTerm control-sequence reference](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
(patch 411, 2026-08-23, read 2026-10-22). The supported model is deliberately
smaller than that specification; malformed input rejection is a tool policy,
not a claim to reproduce every terminal's error recovery.

These are bounded fixture interpreters, not general terminal emulators. Reject
geometry outside 1..4096 per axis / 1,048,576 total cells, more than 4,096 streams
or 16,777,216 input characters, more than 32 CSI parameters or ten decimal digits
per parameter. Only the existing supported commands/parameter arities are
accepted. Colored `REP` is capped at 1,048,576; indexed/RGB channels must be complete
and within 0..255. These explicit supported domains prevent accidental unbounded
allocation/replay; they do not reduce Foundation's public feature scope. Production
terminal APIs, encoders and supported screen sizes are unchanged.

The first candidate validated digit/count limits with another Python scan after
the CSI regex. Its short-control workloads regress. Fuse those lexical limits
into the regex grammar and retain semantic arity/color checks in the interpreter;
keep both candidates' measurements rather than deleting the failed trial.

Keep the known-negative logs and the first aborted test. The initial negative
runner omitted a subprocess timeout and was terminated while replaying an
oversized repeat. Subsequent negative/CLI/RSS children use an explicit timeout.
Never benchmark an old unbounded rejection case merely to obtain a ratio.

Compare unchanged valid complete outputs before/after, retaining every sample
and adverse cohort. New rejection/clamping/wrap fixes have no equal-behavior old
comparator and receive no speed ratio. Measure small and large replay, all three
maintained oracle programs, full CLI/probe/publication paths, and separate Python
traced heap / fresh whole-fixture process RSS. The latter includes interpreter,
imports and input setup and is not retained-only memory or Rust allocation data.
The unchanged Ratatui checker is a control, not an optimization claim. No upstream
framework speed ratio or physical terminal/Unicode conformance is inferred.

High performance within the implemented oracle scope requires these missing
measurements and native/CI qualification; timing a helper alone is insufficient.
Whole-framework Stage B and revised F5 remain separately gated.
