# ADR 0070: Bound Unix canonical input batches without consuming the next line

Status: selected; native conformance and performance requalification in progress.

Six native password conditions now have source-matched byte outputs. The initial
single-call timing retained one large ASCII outlier. A separate complete fixed
128-call protocol keeps those negatives and measures queued native throughput;
it is not an interactive keystroke-latency or calibrated 20ms claim. Long 64-byte
input costs about 41 µs in Rust and 45 µs in the source, with four versus five
allocations. The Rust return buffer requests 120 bytes over its growth sequence.

The pinned source reads one byte per syscall. Matea's Unix password transition
explicitly installs ICANON. Canonical read returns at most one line, independently
of the requested length ([GNU libc terminal documentation](https://ftp.gnu.org/old-gnu/Manuals/glibc/html_node/Canonical-or-Not.html)).
Use that contract for a private 256-byte stack batch only after this transition
succeeds. Keep Windows console text and arbitrary Read implementations on the
single-byte path; no decoder or transport-wide read-ahead is introduced.

Limit each read to at most remaining accepted bytes plus one. This preserves the
existing limit error's exact triggering-byte consumption, including byte-backspace
editing and zero limits. Reserve fallibly only when appending requires capacity,
using the bounded remaining chunk as the reservation hint; newline-only input
allocates nothing. On allocation failure, additional bytes from the current line
may already have been consumed; no partial-data recovery or zeroization guarantee
is added. Following lines remain in the kernel queue.

Require independent expected positions/prefixes across 90 length/limit pairs,
chunk boundaries, byte editing, usize::MAX, actual PTY next-line preservation and
the existing EOF/invalid-byte/error/unwind tests. Reuse the same six native source
fixtures and both sampling protocols for the optimized path. Keep every trial,
and requalify Linux/macOS/Windows/MSRV before promotion.

This primitive's blocking/cancellation boundary remains explicit: no process-wide
signal handlers are installed by TerminalDevice, and abrupt process termination
does not run Drop. Existing interactive runtime signal ownership is separate.
Signal-cancellable direct native reads require a separate ownership/API decision
and native tests; no signal-cleanup guarantee follows from Drop/unwind fixtures.
