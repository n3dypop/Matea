# ADR 0073: Owned clock text without a formatting adapter

Status: provisional implementation; actual-library performance, Miri and
three-OS qualification pending. Foundation remains before F5.

Clock views currently convert ClockText through formatting arguments into Text.
Both that path and the proposed direct path own immutable, normalized display
text. The existing bounded duration formatter already produces the complete
default-format UTF-8 value, so a formatting adapter adds work to this path.

Add ClockText::into_text returning the existing renderer Text and use it in
Stopwatch/Countdown view construction. Feed the bounded formatter's checked
string slice through Text's existing validation and ownership conversion. Keep
Display for width/precision formatting and into_string for callers needing a
String. Scheduling, identities, intervals and timeout semantics are unchanged.
No unsafe constructor, allocator replacement, dependency or crate is introduced.

An original private prototype compares String, existing Text formatting, direct
Text and actual pinned Bubbles Timer.View on six shared numeric inputs in three
orders. All 269 fixed source outputs and Rust Duration::MAX pass before timing.
The [retained experiment](../evidence/clocks-owned-text/prototype/context.json)
contains 72 records and 2,232 positive samples. Direct Text improves p50 over the
existing Text path in all six cases; zero is about 16 to 5.7 ns and microseconds
about 31 to 21 ns. Both Text paths allocate zero in this finite native domain.
Four direct-path cases remain slower than source; all adverse samples remain.

These are prototype observations, not execution evidence for the changed public
library. Rebuild and measure the actual API before accepting the implementation;
retain the prototype and unchanged controls. Preserve all shared outputs and
theme/cursor/Runtime behavior in native tests and exercise the new path under
targeted Miri. Qualify current program inputs on the three Foundation OSes.

Short inline storage is an implementation opportunity, not a public allocation
guarantee; wider Rust durations can allocate. Allocation counters across Rust and
Go do not establish physical-memory parity. Active scheduling, cancellation,
notification latency and retained memory remain separate performance gaps. This
decision does not promote the clock performance score or close the Go export,
Foundation, image-engine or CUDA gates.
