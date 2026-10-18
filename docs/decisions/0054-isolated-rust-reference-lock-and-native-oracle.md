# ADR 0054: Isolated Rust reference lock and native oracle

Status: accepted for rich-text reference qualification; implementation in progress.

A direct Ratatui Git dev-dependency in the main workspace selects the right
source SHA but changes fifteen reachable dependency versions from the reference
lock: the existing API auditor intentionally pins newer syn/quote/proc-macro2.
Do not silently treat this unified graph as the approved locked reference.
The experiment is removed; existing production dependency versions are retained.

Use tools/reference-text as a private, excluded Cargo workspace, with its own
tracked lockfile and the approved Ratatui Git revision. Keep every reachable
reference dependency version compatible with the frozen source observation;
record the additional Matea renderer dependencies separately. This package
contains only original caller/measurement code, not vendored reference sources.
No Ratatui runtime dependency is added to Matea's libraries.

The ordinary native acceptance/CI flow explicitly formats, checks and executes
the reference package, including MSRV. Its default conformance mode reproduces
the full raw 80-cell reference table; comparison tests retain four exact aligned
cases and four literal cross-span cluster deviations. Measurement is an explicit
mode with the shared calibrated, output-checked instrumentation. LLVM collection
executes correctness/output-only modes; timing from CI is excluded from verdicts.

This also keeps consumers' ordinary framework builds independent of the reference
oracle. Source/lock/feature/license hashes, complete outputs and bounded measured
conditions remain required; the full Ratatui API/feature/renderer bridge is open.
