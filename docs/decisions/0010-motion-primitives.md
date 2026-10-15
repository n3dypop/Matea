# ADR 0010: Standalone deterministic motion primitives

Status: accepted for implementation; crate name `matea-motion` is provisional.

Harmonica's pinned public API consists of cached damped springs and 3D constant
acceleration projectiles, plus a frame-duration helper. Keep these independent of
the runtime, renderer and terminal. Applications explicitly supply time steps;
the motion library creates no timer, worker or global animation state.

Use Duration and NonZeroU32 for frame durations; reject negative/nonfinite spring
parameters and nonfinite states with typed errors. Cache the four coefficients
of the damped-oscillator transition matrix. Evaluate underdamped, critical and
overdamped cases; use slow/fast eigenvalue exponentials in the overdamped case to
avoid an intermediate cosh overflow. These are algebraic formulas for
`x'' + 2ζω x' + ω²(x-target) = 0`. Zero frequency retains Harmonica's identity
behavior. Reduced-motion behavior is an explicit snap-to-target operation.

Preserve the public reference projectile's forward Euler step as `step`, making
trajectory comparisons meaningful. Also provide an analytical constant-
acceleration `step_exact`: upstream Euler error depends on frame rate, whereas
the analytical update composes across different time partitions. This is a
documented opt-in improvement, not an unannounced behavior change.

The independent Rust equations are informed by Harmonica and Ryan Juckett's
damped spring explanation. Preserve attribution and the original permissive
notice in THIRD_PARTY_NOTICES.md; identify the Rust implementation as altered.
No Go implementation is vendored. Tests compare equation identities, reference
trajectories, invalid inputs, overdamping and partition invariance. Benchmark
cached updates and coefficient construction separately before parity is claimed.

Follow-up measurement found per-step finite validation measurable at nanosecond
scale. Expose `sample` for the same IEEE numeric contract as the reference, and
keep `step` for explicit input/result errors. Neither uses unsafe code. Benchmark
the raw arithmetic separately from checked updates; the Go checked case applies
the same finite checks in the original research harness. Applications choose the
contract explicitly rather than silently dropping validation to win a benchmark.
