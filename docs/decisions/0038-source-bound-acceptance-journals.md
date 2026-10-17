# ADR 0038: Source-bound, fail-closed native verification

Status: accepted for Foundation verification tooling.

The acceptance runner previously wrote its result only after all commands. An
executable launch error could leave an older successful report behind. The
remaining-measurement driver also rejected any review-unit count other than 40,
so the valid 41-unit cursor addition disabled its assessment workload.

Publish a running, unsuccessful journal before source discovery or tool launch.
Record every command result, including launch failure and timeout with partial
output; continue independent checks. Success requires every check and tool
probe to succeed with unchanged program/build/check inputs. Bind each log to
its raw-byte hash. Quality publication validates those sources and logs, and
rejects missing, stale, running, duplicate or boolean exit-code evidence.

Discover inputs through Git, including untracked source, root/build/benchmark
programs, external-example manifests, checksum files and the quality/pin plans.
Use the actual plan's complete scope for measurement. Growing the project or
eventually reaching all-high must not make the workload fail. Measurement
operations return immutable bytes and retain exactly 31 positive intervals;
separate Python heap passes always release instrumentation after failure.

LLVM coverage also replaces old success with a running journal first, clears
only its generated raw profiles and recollects test/build/executable outputs.
Remove the unbound `--existing` shortcut: earlier profiles cannot be asserted to
belong to a fresh invocation. Empty objects/profiles/exports, failed commands,
malformed LLVM output and source changes fail the journal. Use a profile-list
file to avoid Windows command-length limits. Canonical source instances remain
separate; instrumented durations supply no performance evidence.

Run the complete acceptance and LLVM drivers on all three native CI hosts.
Independent tiny Git/process fixtures cover startup, timeout, nonzero result,
partial output, old-success replacement, source drift and path/hash corruption.
These tests do not prove every possible filesystem failure or process schedule.
Atomic per-file replacement prevents partial JSON; this is a single-writer
contract, not a transaction spanning all logs or protection against manual
forgery of evidence.

Hash-bound files must survive Windows checkout byte-for-byte. Set the checkout
text policy to LF and test inventory bytes with `core.autocrlf=true`; retain the
original CI failure rather than weakening the summary hash check.
