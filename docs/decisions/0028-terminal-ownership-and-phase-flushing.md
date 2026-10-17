# ADR 0028: Own terminal cleanup through nested destruction and flush by phase

Status: accepted for Foundation implementation; public API remains provisional.

## Context

The native backend released its process-wide exclusivity flag inside `Native::drop`, before Rust dropped its fields. If explicit signal restoration failed, the remaining `SignalGuard` field could retry restoration after another Matea session acquired the flag. This ordering was found by source inspection, not by reproducing a kernel race. Mode application also flushed after each command, and the two-signal restoration journal allocated a vector despite its fixed bound.

## Decision

An internal `Exclusive<B>` owns the backend before a separate RAII lease field. Rust field destruction order keeps the lease until the entire backend, including nested signal resources, has been destroyed. Constructor errors and unwinding release the lease automatically. Contention returns `WouldBlock`. The existing backend/session trait and public facade do not gain lifetime or cleanup obligations.

Use two optional journal slots for SIGINT and SIGTERM. Restore in reverse acquisition order, clear successful slots, retain failed slots for retry and attempt all entries. No heap allocation is required to record or retry these two actions. Keep FFI confined to the existing Unix module; the handler still only stores a lock-free atomic flag. Other libraries can still change process signal handlers; Matea exclusivity only coordinates Matea sessions.

Queue native mode commands, preserving their byte order and Crossterm platform dispatch. Flush after entering modes and clearing the screen, and after attempting all restoration commands. On errors, the existing session restoration path retries the journal. This reduces logical flush calls; it does not establish fewer physical writes or a real-terminal latency improvement.

## Validation and limits

Regression tests assert that ownership outlives backend and nested destructors, reject overlapping constructors, release failed/panicking constructors, and exercise every two-signal failure mask and partial acquisition. The native phase benchmark checks exact output literals for default, minimal and full mode cycles. Record before/after timing, requested allocations and logical write/flush counts separately. Actual signal calls run only in an isolated Unix probe; synthetic journal tests do not establish that an OS syscall cannot fail. Native PTY cleanup, all-target checks and three-OS CI remain required after the implementation changes.
