# ADR 0071: Promoted Go exports and explicit Windows console modes

Status: selected; native Windows qualification pending.

The pinned x/term public State embeds a private state type. On Windows that type
contains exported Mode uint32, so callers can select and modify State.Mode even
though the raw declaration scanner lists only the private embedding. A finite
original caller compiles its Mode reads/writes for Windows and fails, as expected,
for Linux/macOS. This is compiler reachability evidence, not native execution.
The thirteen raw candidates must never be interpreted as a resolved export graph.

Add an explicit Go target/export closure requirement to the Foundation coverage
gate. Missing metadata is pending; truthy strings/numbers are rejected. A synthetic
ledger with all other conditions satisfied must remain closed until this distinct
review is true. Keep it false for this project until promoted/returned/embedded
members across relevant target profiles are resolved and mapped. Do not silently
rewrite raw candidate counts as compiler export counts.

The ordinary portable TerminalState stays opaque. An explicitly Windows-only
device::windows module exposes a ConsoleMode bitflag value, lossless mode read
and snapshot construction. Constants distinguish input/output names, whose bits
can overlap. Retain unknown bits to represent future OS flags and preserve the
source's full uint32 customization capability. Value construction performs no
native IO and does not certify flags for a particular handle role. TerminalDevice
still applies modes through the checked OS call and owns restoration before
releasing the shared lease. Raw integers/pointers are not used as file handles.

This uses the already pinned bitflags dependency, with no second version or new
FFI. The Windows native State gains only scalar projection/construction helpers.
Test every low word and individual bit/complement, unknown-bit retention, named
flag editing, real input-console application and original restoration. Keep all
three OS/MSRV checks and benchmark/source conditions visible. No portable raw flag
fields, image processing, image decoding or CUDA work is added.
