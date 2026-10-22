# Borrowed terminal-device implementation

Status: implemented contracts qualified on Foundation targets under [ADR 0069](decisions/0069-owned-terminal-device-state-and-password-eof.md)
and [ADR 0070](decisions/0070-canonical-terminal-input-batching.md). Whole-module
reference and performance assessments stay partial. The preceding qualified
checkpoint aff137a had 48 unmet axes; the expanded current 156-program/44-unit
scope has 50, including this new unit's reference/performance obligations.

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
On a limit error, the triggering byte has been consumed and only the accepted
prefix is returned; subsequent bytes remain queued. There is no implicit drain,
secret zeroization, nonblocking cancellation or recovery promise.
Windows uses bounded UTF-16 decoding with owned lookahead/UTF-8 bytes and input
handle validation. Native Unix/Windows FFI is narrowly allowed and documented;
the orchestration/line parser stays under the crate's unsafe-code denial.

Pure tests cover retryable restoration, read/unwind cleanup, both line endings,
empty/partial EOF, invalid UTF-8, no overread, limits and interrupted surrogate
pairs. Native UTF-16 replacement tests cover all 65,536 single-unit cases plus
surrogate pair boundaries; Miri uses a documented finite subset. These tests and
the initial Unix Clippy pass do not qualify Windows console I/O or a full source
module. The final qualification below adds the separately executed native fixtures;
full module performance/condition closure remains pending.

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
UTF-16 directly, without duplicated handles. Its finite native console validation
is recorded below; other source-platform conditions remain explicit.
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
native acceptance hook and instrumented native hook provide the separately
observed CI evidence in the final qualification below.

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

The [fused-constructor follow-up](evidence/terminal-device/performance-fused/attempts/64bd20d0debb4f2692af590fd73b0eda/report.json)
binds 493e2d6, with both fresh lifecycle variants retained in the same executable.
The ordinary four-call operation takes 9.52/9.58/9.61 µs across its three cohorts;
the fused three-call operation takes 9.22/9.30/9.36 µs, with zero allocations for
both. This is a 2.6–3.1% latency reduction in this experiment, alongside the
independently tested single-capture contract. It does not justify removing the
current-state query from later make_raw calls. All four source comparisons still
match their fixture outputs, and the constructor controls have no source ratio.
The additional 30 timing and 62 RSS records, together with the preceding experiment,
are [reconstructed from archived data](evidence/terminal-device/performance-verification.json):
57 timing records, 1,767 retained intervals and 124 distinct RSS children. This
verification executes no timers or binaries. The intermediate mixed-source LLVM
collection was rejected as source drift and retained, never promoted as a pass.

## Native byte-input measurements and redevelopment

[Four complete input experiments](evidence/terminal-device/password-performance-verification.json)
retain 144 timing records (4,464 positive intervals) and 1,488 separate RSS children.
Each compares six real PTY inputs: ASCII, UTF-8/supplementary text, invalid UTF-8
with NUL, empty input, byte-backspace retaining a partial scalar, and 64-byte text.
All returned bytes/FNV values, exact supplied call counts and configured terminal
states are checked. The helper's active dependencies match the root graph; only
its private executable is additional. Both use Cargo's default release profile.

The first fixed-one-call experiment retains a 32.3 µs Rust ASCII cohort versus
13.4 µs source, followed by 11.0/10.7 µs Rust and 11.2/11.8 µs source. Do not drop
that outlier or claim every native measurement wins. A second full fixed-128-call
protocol measures queued native throughput, including producer/kernel queue
interaction. It is separate from single-call latency and has no 20ms calibration.
Cold calls include producer startup and remain distinct from retained samples.

[ADR 0070](decisions/0070-canonical-terminal-input-batching.md) uses the documented
canonical read boundary to batch at most 256 bytes, limit reads to remaining
accepted bytes plus one, and reserve from a bounded chunk only when capacity is
needed. Windows and arbitrary readers keep single-byte reads. Ninety independent
length/limit expectations test the exact consumed prefix/position and following
line; all twelve real macOS PTY cases pass after the change.

| Same-protocol long-input comparison | Before Rust p50 | After Rust p50 | Source p50 after |
| --- | --- | --- | --- |
| Fixed 1 call, three cohorts | 40.3–41.2 µs | 12.3–13.0 µs | 47.9–49.0 µs |
| Fixed 128 calls, three cohorts | 41.3–41.9 µs | 15.4–15.7 µs | 43.5–44.7 µs |

Return-buffer growth changes from four allocations/120 requested bytes to
one/65; empty input stays zero-allocation. The fixed-128-call results improve
the other nonempty cases modestly. One optimized single-call empty cohort is
11.7 µs versus source 11.2 µs, and first ASCII cohorts remain slow in both
implementations. These negatives, p95 distributions and every raw record remain.
No universal optimality, all-case speedup or other-host performance is inferred.

The single-after collector initially archived a later Go caller while reusing
the exact earlier fixed-one source binary. A [separate corrective binding](evidence/terminal-device/password-single-source-binding-correction.json)
links that unchanged binary to its actual before caller; the original collector
and mistaken snapshot remain intact. Both execute one operation per sample in
this environment, but their source snapshots are not asserted identical. Other
experiments bind their actual caller snapshots. The initial comparison-caller
compile failure and inactive-target dependency-filter mismatch are also separate
from framework failures. The archive-only verifier checks these boundaries.

Native error/cancellation paths, source-only OS conditions and other-host
performance remain pending; the device performance assessment stays partial.
The [typed handle guide](terminal-device-usage.md) and three local rustdoc checks
address x/term documentation issue #76 without claiming historical bug or Windows
runtime reproduction. Its current body/update date is retained in the ledger.

## Final qualification of the implemented slice

[5bcdeb6 CI](https://github.com/n3dypop/Matea/actions/runs/39483552313) passes all
seven required jobs. The [source binding](evidence/terminal-device/ci-5bcdeb6/source-bindings.json)
checks 198 execution inputs against that Git commit on all three hosts. Each
host passes 47 acceptance commands. Real native and independently instrumented
device fixtures each pass twelve cases on Linux/macOS and nine in isolated
Windows consoles, including supplementary UTF-16 input. Native modes, exact byte
outputs, size, raw/set/restore, Drop/unwind, regular-file/pipe rejection and Unix
following-line/EOF behavior are verified within those finite fixtures.

Uploaded raw log hashes are checked (31 per Unix host, 32 on Windows); the sixteen
or fifteen logs not included by the artifact configuration are explicitly listed.
Unix LLVM reports retain 95 canonical source paths/102 compilation instances;
Windows retains 94/100. These are execution observations, not branch or physical
terminal proofs. Shared-runner timing is not used as benchmark evidence.

The post-assessment local suite also passes 47 commands, Rust 276 plus 11 doctests,
Python 144 with two conditional skips and LLVM 95/102. [Derived counts](evidence/terminal-device/final-metrics.json)
avoid treating stderr's delayed Doc-tests labels as stdout section boundaries.
[Miri seed 223](evidence/terminal-device/miri-canonical/verification.json) passes
seven pure device tests plus two lease tests, including all ninety canonical
length/limit positions. Native tests cover all 65,536 single UTF-16 units; Miri's
single-unit domain uses its declared 257-step subset and paired boundaries.

The six changed units' portability and the device's design/memory/behavior are
75 within this reviewed scope. Device reference/performance stay 50, with source
OS conditions, full exported contracts, error/cancellation costs and other-host
measurements still explicit. Stage A remains open; Stage B and F5 remain closed.

## Reopened promoted-export increment

The qualification above remains the c526cf4 checkpoint. A subsequent compiler
probe discovered that Windows State.Mode is accessible through the private state
embedding, even though the thirteen raw candidate records omit that selector.
The original caller compiles for Windows and fails for Linux/macOS at the pin;
[logs and exact caller](evidence/terminal-device/promoted-mode/context.json) retain
both positive and negative compiler results. This does not execute a console.

[ADR 0071](decisions/0071-promoted-go-state-mode-and-explicit-windows-api.md)
adds an explicit conditional ConsoleMode/read/snapshot API and a separate Go
target/export gate. The portable snapshot stays opaque. Known input/output flags
have names and unknown uint32 bits remain representable; the OS validates role
and flags only when applying through the owned device guard. The direct bitflags
edge uses the already pinned version. Windows fixtures now exercise input and
output mode editing/restoration and reject input-only operations on output
handles; pure Windows tests cover low words, bit/complement boundaries and
unknown-bit edits. The exact-source Windows qualification is recorded below.

The raw inventory count is unchanged and no complete Go export resolution is
claimed. The project-wide Go target/export review remains false; missing or
truthy nonboolean metadata cannot satisfy it. Two independent corrupted-ledger
tests confirm this condition can block F5 even when the synthetic other
conditions are complete. Seven affected assessments were reopened (57 axes);
the prior fully tested 50-axis checkpoint remains a separate historical record.

The follow-up bounded export-data experiment also exposes Darwin State.Termios
and seven data fields, and Linux amd64 State.Termios and eight (including Line).
The source embeds unix.Termios, so its descendants are promoted through the
private state as well. [All three compiler profiles](evidence/terminal-device/go-export-experiment/verification.json)
retain source locations and selection indexes. This experiment remains incomplete
for pointers, returned/private interfaces, generics and other reference packages.

[ADR 0072](decisions/0072-explicit-unix-native-attributes.md) adds conditional
Unix Attributes/read/snapshot operations with owned target-libc records, keeping
the portable type opaque. Native layout/field names/control counts can differ
from Go's kernel record; speed normalization and unsupported source OSes stay
explicit. The private guard's original is not exposed mutably. Pure tests cover
all 256 values per native control slot and copy independence; live PTY fixtures
change echo and EOF bytes, query them, restore, then run the previous twelve
cases. The driver now explicitly selects EOF=Ctrl-D instead of relying on a
host default.

## Qualified native attribute and console mode increment

[04e81bc CI](https://github.com/n3dypop/Matea/actions/runs/39495143723) passes all
seven required jobs. The [independent source binding](evidence/terminal-device/ci-04e81bc/source-bindings.json)
verifies all 198 inputs against Git and matching inputs on three OSes, 47 native
commands per host, twelve Unix/nine Windows fixtures in both ordinary and
instrumented runs, exact bytes and original-mode restoration. The conditional
attribute/mode edits run inside every exact-source fixture. Unix only excludes
Darwin's named transient PENDIN bit; all other fields are compared. Uploaded log
hashes are verified, while sixteen Unix/fifteen Windows logs absent from uploaded
artifacts remain explicitly listed. MSRV 1.88 and pinned-reference probes pass.

[Miri seed 223](evidence/terminal-device/miri-attributes/verification.json) passes
ten pure device/lease tests, including every byte value in each native Unix
control slot and snapshot independence. It does not interpret native FFI or the
Windows-only mode projection. [Post-assessment local execution](evidence/terminal-device/attributes-final-metrics.json)
passes 47 commands, Rust 277 plus 11 doctests, Python 146 with two conditional
skips, and LLVM 95 canonical source paths/102 instances. The three transitive
hashbrown zero-hash profile warnings are retained and diagnosed; no branch or
whole-dependency soundness proof is inferred.

The six affected portability assessments and device behavior return to 75.
Device design/memory remain 75 with refreshed bounded evidence; reference and
performance stay 50. The 156-program/44-unit inventory and threshold 75 are
unchanged. Fifty axes remain below high, and the Go target/export closure flag
stays false. Stage A continues; Stage B and F5 remain closed.
