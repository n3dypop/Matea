# Borrowed terminal-device implementation

Status: implementation and qualification in progress under [ADR 0069](decisions/0069-owned-terminal-device-state-and-password-eof.md).
The qualified preceding checkpoint is aff137a (48 unmet axes); the new device
module and shared lease require new native, Miri, performance and three-OS evidence.
None of the new raw-module/declaration conditions is labeled complete yet.

`terminal::device` borrows AsFd/AsHandle owners and keeps captured native state
opaque. A UI-local TerminalDevice shares the interactive backend's atomic lease,
rejects nested mutation, and drops restoration before releasing ownership. Read-only
state/size/terminal queries remain available without taking a mutation lease.
Explicit restore reports errors and remains retryable; Drop attempts cleanup.
No borrowed handle is closed or duplicated by the Unix implementation.

Non-echo input temporarily restores the caller's current mode, independently of
the outer original-state guard. EOF, Interrupted, byte-backspace, byte limits,
partial data and simultaneous read/restore errors have explicit contracts.
PasswordError's formatting omits partial bytes, which remain available through
explicit accessors. Growth is fallible, and reading does not buffer following lines.
Windows uses bounded UTF-16 decoding with owned lookahead/UTF-8 bytes and input
handle validation. Native Unix/Windows FFI is narrowly allowed and documented;
the orchestration/line parser stays under the crate's unsafe-code denial.

Pure tests cover retryable restoration, read/unwind cleanup, both line endings,
empty/partial EOF, invalid UTF-8, no overread, limits and interrupted surrogate
pairs. Native UTF-16 replacement tests cover all 65,536 single-unit cases plus
surrogate pair boundaries; Miri uses a documented finite subset. These tests and
the initial Unix Clippy pass do not qualify Windows console I/O or a full source
module. Real OS fixture and performance work continues.

Source observations retain a macOS PENDIN-only state difference after raw
restoration, byte-level backspace behavior and Ctrl-D calls requiring an extra
newline. Source's historical Plan 9 and other platform conditions remain explicit;
unsupported Rust targets return Unsupported. No image engine, codec/decoder or
CUDA work is included; UTF-16 decoding here is native console text input.

## Declaration and platform boundary

All candidates below bind `charm-x-term` 4542a189d0fbeb156bb1df8730d253f2ef443c00;
the source declaration IDs and immutable URLs remain in the inventory.

| Source candidate | Rust composition and observable contract |
| --- | --- |
| State | Opaque cloned TerminalState, captured through terminal_state/device.state. |
| State.state | Private embedded implementation state; no independent public constructor. |
| IsTerminal | is_terminal on the exact borrowed native handle. |
| MakeRaw | TerminalDevice.make_raw returns previous state, owns original restoration. |
| GetState | terminal_state and TerminalDevice.state. |
| SetState | TerminalDevice.set_state; non-null typed snapshot, errors remain retryable. |
| Restore | TerminalDevice.restore restores original state; set_state can apply a previous snapshot. Drop attempts cleanup. |
| GetSize | terminal_size/device.size; Windows requires an output-console handle. |
| ReadPassword | read_password(maximum_bytes), owned bytes and typed partial/read/restore errors. |
| passwordReader.Read | Exported method on private Unix receiver, not a separately reachable constructor/API. Its zero-read behavior is assessed through ReadPassword. |
| File | TerminalFile plus standard Read/Write/owner Drop composition. The native borrowing API does not require unused write/close operations. |
| File.io.ReadWriteCloser | Standard IO traits and owner Drop/explicit file ownership; no independent Go-like Close error promise. |
| File.Fd | AsFd/AsHandle; unchecked uintptr replacement is deliberately unavailable. |

The source Unix build condition is aix/darwin/dragonfly/freebsd/linux/netbsd/
openbsd/solaris/zos, with separate BSD versus other ioctl requests. Rust uses the
target libc constants rather than copied request numbers. Foundation qualification
requires Linux and macOS runtime evidence; other Unix targets are not qualified by
those two results. Source Windows uses console modes and an output viewport;
the Rust adapter validates input before raw/password operations and reads console
UTF-16 directly, without duplicated handles. Its native validation is still pending.
Source Plan 9 resolves /fd/<number>ctl paths, with rawon/rawoff control writes;
Matea currently returns Unsupported on that target. Source other-OS stubs also return
unsupported errors. These conditions remain pending rather than being hidden by
an all-platform parity claim.

## Native negative results and retest

The first macOS fixture rejected cfmakeraw's extra flag changes; exact native
before/raw/expected records and the former source are retained under
`evidence/terminal-device/`. Explicit source-matched edits now pass. The next
fixture timed out because its controller sent an unterminated canonical-mode
barrier; supplying the delimiter corrects the test without relaxing restoration.
Both failed attempts remain separate. The expanded current fixture passes twelve
macOS PTY cases: explicit state/raw/set/restore, Drop, unwind and nine input cases.
It rejects both a regular file and a captured stdout pipe, checks exclusive lease
reacquisition, actual dimensions, configured flags and preservation of the next
line. Only the named Darwin PENDIN transient flag is excluded from state equality.

Two compile-fail examples reject Send and closing a still-borrowed owner. These
are type-contract checks, not native cleanup evidence. The pure UTF-16/parser and
mock failure tests likewise do not qualify WinAPI. A new-console Windows adapter,
native acceptance hook and instrumented native hook are implemented for CI; their
results must be observed before portability is promoted.

## First native performance experiment

The [retained report](evidence/terminal-device/performance/attempts/3e0cd5c347b446ef8488926bdbd4a44d/report.json)
binds 46bac2a, both release binaries, 198 execution inputs and the fixed source
files. Three cohorts contain 27 timing records, each with 31 positive samples
and all 20ms calibration confirmations. Another 62 output-only children record
Darwin maximum RSS separately. No builds, tests, Miri, LLVM or downloads overlap
these ordinary measurements. All configured PTY flags restore after every child.

| Isolated 132×42 PTY operation | Matea p50 across cohorts | Source p50 across cohorts | Allocations, Matea/source |
| --- | --- | --- | --- |
| Terminal query | 277–279 ns | 305–308 ns | 0/0 |
| Owned state capture/drop | 276–279 ns | 318–324 ns | 0/1 (80 bytes) |
| Visible dimensions | 271–275 ns | 300–305 ns | 0/0 |
| Current-state capture, raw, previous-state restore | 9.23–9.28 µs | 9.43–9.51 µs | 0/1 (80 bytes) |

State records check successful opaque capture, independently observed native
before/after fields and explicit flag fixtures; they are not a serialization of
every state byte. The raw case already owns the Rust lease outside samples;
the source caller keeps its descriptor open. Each matched cycle performs three
native calls. All paired return values, declared semantics and fixture outputs
agree. Retained p95 values remain in the report; no universal speed advantage or
maximum-performance claim follows from one host's modest differences.

The separate Rust fresh-owned lifecycle uses four native calls and takes
9.53–9.58 µs, without allocations. It includes original capture, current capture,
raw transition, original restore and release. This is a control with additional
ownership work, so it has no source ratio. The extra current-state query suggests
a fused fresh raw constructor; the existing make_raw must still query the actual
current state when called later on an existing guard.

Whole-process RSS medians are 1,949,696 bytes for Rust and 5,144,576 for Go, with
31 distinct children each. These include each language's startup/runtime and
different available case sets; they do not measure per-operation peak memory.
Password latency, full native error-path cost and other OSes remain unmeasured.
The module's performance verdict therefore remains partial.
