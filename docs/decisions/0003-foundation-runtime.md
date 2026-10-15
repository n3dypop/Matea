# ADR 0003 — Typed state and cooperative effects

Status: Implementation decision; provisional public API, owner review pending (2026-10-16).

## Evidence and alternatives
Bubble Tea v2 `tea.go:53` defines Init/Update/View; `tea.go:724` starts one goroutine per command and explicitly cannot cancel arbitrary commands. `commands.go:15` distinguishes unordered Batch and ordered Sequence. See the pinned audit.

Compare a callback application builder, a generic Model trait, and dynamic message downcasts. Choose `Model` with an associated `Message`, mutable Update, immutable View, and `Event<Message>` separating input, resize, completions and failures. The model stays on the calling thread and need not be Send. Effects alone cross threads. No Tokio requirement.

## Decision
`Runtime` is terminal independent and can be driven deterministically in tests. Commands have string IDs and cancellation tokens. A bounded completion channel and fixed active-task limit prevent unbounded spawning. Batch is unordered; Sequence uses one task slot and emits results in order, without waiting for application updates between steps. Overflow is an actionable runtime error. Task errors/panics become failure events. Cancellation discards queued stale completions using generations. Subscriptions are reconciled by stable ID; changing their behavior requires changing the ID. Timers run on the UI clock, coalesce missed ticks, and do not create worker threads.

Cancellation is cooperative: tasks must check the token or use its interruptible wait, and provide their own I/O deadlines. Shutdown cancels every task and detaches unfinished workers; arbitrary blocking Rust functions cannot be forcibly killed. This limitation is part of the contract, not a claim of leak-free arbitrary user tasks.

## Consequences
Small explicit API, testable transitions, no hidden executor. No async-await adapter or dynamic ABI promise in this milestone. APIs are 0.1 experiments and must be reviewed before publication.
