# ADR 0069: Borrowed native devices with owned terminal restoration

Status: selected for the next Foundation implementation; public names provisional.

The existing runtime manages stdin/stdout lifecycle, but the pinned x/term public
device-state/descriptor functions are not exposed as complete Matea capabilities.
Its [13 declaration candidates](../coverage/charm-x-term.json) include the private
embedded state and private receiver's exported Read method; classify those
explicitly instead of presenting every candidate as a reachable public function.
The public State, File, IsTerminal, MakeRaw, GetState, SetState, Restore, GetSize
and ReadPassword contracts need actual equivalents and platform conditions.

[Six real PTY observations](../evidence/terminal-device-source/context.json) bind
x/term 4542a189d0fbeb156bb1df8730d253f2ef443c00. The source returns correct sizes,
rejects regular files and accepts state/raw/restore operations. On this macOS
host, a native PENDIN bit changes after raw restoration; opaque state equality
must not be promised as a cross-kernel restoration invariant. Raw states and the
initial strict-comparison failures remain retained.

Two source password observations require an additional newline after empty or
partial Ctrl-D input. A 250ms observation followed by newline releases each call;
this is not an infinite-time claim. The Unix passwordReader forwards `(0,nil)`
from unix.Read, while readPasswordLine only recognizes EOF through an error.
The missing zero-read termination is a source-backed explanation of the observed
empty EOF case. Partial canonical EOF behavior needs separate exact input tests.
CR is affected by kernel ICRNL; byte-backspace can retain partial UTF-8. Do not
silently sanitize or render secret bytes, or infer behavior from the private
line parser alone.

Add a provisional `terminal::device` API with a borrowed native descriptor/handle,
opaque owned state snapshots and a restoration guard. Use Rust's AsFd/AsHandle
and borrowed-handle lifetimes instead of unchecked integers or owning/closing the
caller's file. State capture/sizing should allocate nothing. Use the existing
Unix libc dependency for a small reviewed syscall boundary, and an explicitly
pinned Windows console dependency/profile. No platform flags leak into the
portable facade. Unsupported targets return explicit errors; Linux, macOS and
Windows are required, while source Plan 9/other OS conditions stay visible in
the ledger and cannot be described as tested Rust targets.

The initial Darwin cfmakeraw trial failed the independently computed pinned
raw-state fixture: it also changed IGNBRK/IMAXBEL and echo-edit bits. Retain that
negative before switching to explicit pure Rust flag edits matching the source.
Do not assume the platform convenience function has the same observable state.

The device guard and interactive backend must share the same exclusive lease.
Reject nested ownership before changing a device. Capture the original state
once, restore on explicit completion, error and Drop/unwind, and retain the lease
until the native resource/cleanup has finished. Keep state restoration retryable
after a failed syscall. Expose current state, checked application of a captured
state, raw transition and viewport size through this guard; callers do not need
to coordinate terminal restoration themselves. Non-null typed snapshots replace
source nil state/panic cases. Snapshot equality is not a public promise.

Non-echo input temporarily saves/restores the current mode within the device
guard. Preserve caller-visible byte input and platform line endings; reject a
zero-byte read as empty EOF or finish a partial value, and retry Interrupted
without spinning. A read error must retain accessible partial bytes and any
restoration error through a typed error whose Debug omits byte contents. Use an
explicit maximum byte count and fallible growth; callers can select a large limit
when needed. Do not claim zeroization or change UTF-8/backspace semantics without
separate documented policy. Avoid reading beyond the completed input line;
buffering optimizations require measured native semantic evidence first.

The first native measurement found that a fresh open+make_raw cycle queries the
same initial state twice, whereas a later make_raw must requery current state.
Add open_raw to fuse only fresh acquisition/transition: one captured original,
one raw set and one eventual restore. Failure keeps cleanup owned before release.
Keep both APIs, with mock query-count/error tests and real Drop/unwind fixtures.
The Windows-only benchmark lint failure is corrected with cfg-local control flow,
without suppressing unreachable-code warnings. Exact Linux state checks exclude
no Darwin-only transient flag; the PENDIN exception is limited to Darwin.

Acceptance requires pure mock failure/unwind/lease tests, real isolated Unix PTYs,
native Windows console checks, exact configured flags/size restoration, regular
file/pipe errors, UTF-8/invalid-byte/backspace/CR/LF/empty/partial EOF and boundary
limits. Compare actual pinned source and Rust operations on matched native devices,
including all output/state and distributions, allocation and RSS evidence. Run
Miri on supported pure paths and qualify all three OSes/MSRV before module closure.
This is Foundation terminal I/O, without image processing, decoding or CUDA.
