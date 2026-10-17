# ADR 0025: Adaptive indexing for ordered focus IDs

Status: accepted for the existing Foundation API; full reference coverage remains open.

## Problem and evidence

`FocusRing::new` retained the first occurrence of every ID with `Vec::contains`.
That preserves predictable traversal and owns each retained string once, but
construction was quadratic for many unique IDs. The 37-case input probe measures
construction separately from typed events and Crossterm translation. Each
constructor workload includes cloning the already prepared source strings and
dropping the result; there is no event loop, terminal parsing, I/O or renderer.

On the recorded macOS arm64 host, 1,024 unique IDs took a median 865,527.50 ns.
An initial indexed implementation reduced that to 102,929.16 ns but regressed
three-ID and empty constructors. The intermediate raw samples and source patches
are retained under `docs/evidence/optimization/input`.

## Decision

Keep linear scanning through 128 unique IDs. Construct a randomized hash index
only upon the 129th unique ID, so duplicate-heavy streams do not trigger it based
on their input length. Outline the indexed path and inline the small constructor.
Initialize temporary tables from the already allocated output capacity; do not
reserve from an arbitrary iterator's size hint.

Strings remain owned only by the output vector. Hash table heads and collision
links contain integer indices. Every hash match checks the complete string;
neither hash collisions nor table iteration can remove distinct IDs or reorder
them. There are no self-referential borrows, raw pointers, unsafe blocks, global
caches or new dependencies. The temporary index is dropped before returning.

The final measured 1,024-ID median is 95,225.84 ns (about 9.1 times faster than
this Rust baseline). Requested allocation bytes increase from 58,186 to 133,474,
and allocation calls from 1,033 to 1,039. These are cumulative requested bytes,
not measured peak/resident memory. Three unique IDs improve from 184.42 to
145.38 ns with unchanged allocations; empty construction changes from 2.08 to
3.58 ns, and the duplicate-heavy 1,024-ID case from 26,787.50 to 26,837.50 ns.
No universal speed or memory improvement is claimed. The threshold is a bounded
tradeoff supported by these fixtures, not an optimum for every application.

## Validation and limits

Independent first-occurrence models check output and multiple complete traversal
cycles. A deliberately constant hasher forces every unique ID into one collision
chain. A panic-on-construction hasher checks the small/duplicate-heavy path never
builds an index. Native and targeted Miri checks cover these cases. All 37 input
outputs, operation contracts and iteration counts must agree across revisions.

This preserves the existing Matea behavior. It is not a performance comparison
with Bubble Tea's application-specific focus handling, a proof of worst-case
linear complexity, or complete input/reference parity. Adversarial collision
chains can still take quadratic CPU. Full retained memory and application input
latency measurements remain outstanding.
