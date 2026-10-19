# ADR 0062: Fixed source trees and separate auditor phases

Status: implemented; source/performance and multi-platform qualification pending.

The Rust source-auditor recursively called `Path::is_dir`, following symlinks.
A fixed fixture with an external directory and a cycle returned 31 declarations,
including 15 outside the requested root, instead of one local declaration. The
result is successful but invalid input containment; retain the known-negative
observation. Non-UTF8 paths also used lossy conversion and could collide.

Extract the existing parser/serializer into the internal package's library and
keep the CLI a thin writer/argument boundary. `audit_source` measures an existing
UTF-8 string through AST, declaration extraction and complete NDJSON separately
from directory discovery, filesystem reads, process spawn and pipe transport.
Both library and CLI retain the identical declaration schema and valid-input
ordering. This package remains unpublished; no new framework crate API is locked.

Replace directory recursion with an explicit deterministic DFS stack. Use
`symlink_metadata`, skip symlink and non-regular entries, reject linked/non-directory
roots, and reject non-UTF8 relative names instead of emitting ambiguous IDs.
The contract applies to fixed input trees. It does not claim an adversarial
filesystem sandbox or eliminate metadata/open TOCTOU races. Concurrent mutation
requires a frozen snapshot or separate OS handle-based design.

Test Windows directory junctions without requiring developer-mode privileges;
Unix tests additionally exercise symlinks and a FIFO ending in `.rs`; Linux tests
exercise invalid path bytes (the local APFS filesystem rejects creation of those
names). Rust 1.88's [Windows FileType implementation](https://github.com/rust-lang/rust/blob/1.88.0/library/std/src/sys/fs/windows.rs#L1052)
marks name-surrogate reparse points as links. Native Windows CI remains required;
the source inspection is not runtime proof.

Measure the complete CLI on both pinned Rust references, empty/ignored/linked/
malformed trees and generated source at increasing scale. Independently reconstruct
every generated declaration/byte stream, retain full output digests and compressed
raw data, and separately measure parser/serialization with allocation counters.
Collect per-process peak RSS in a distinct pass; do not call profiler timing
ordinary latency. Compare unchanged callers against the recursive baseline, retain
all failures and adverse tails, and investigate regressions before raising a score.
