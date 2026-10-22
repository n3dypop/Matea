# ADR 0066: Go source phases with the common measurement caller

Status: implemented; output/native/performance qualification in progress.

Separate the existing Go AST/declaration/NDJSON body into private `auditSource`.
Production passes nil source to retain the parser's file-reading behavior; phase
measurement passes preloaded bytes and keeps discovery/read/spawn outside that
operation. The caller owns encoder buffering/flush. Parse errors publish no rows
from the invalid file; writer errors remain observable. This is an internal tool,
not a public Matea framework API.

Use the existing Go upstream measurement caller verbatim: bounded confirmed
calibration, the same non-inlined hot loop, first retained call, 100 warm-up calls,
31 observations, allocation/requested-byte passes and an explicit output-only
mode. A source-identity test fails on caller drift. Do not add a second benchmark
method or retry adverse samples. One ordinary Go test file hosts this measurement
code; it is included in the complete program inventory and native output checks.

Independent Python literal row construction verifies complete synthetic
constant/struct-field outputs at 1,000/10,000/100,000 scale through byte counts and
FNV. The nine fixed Go reference ledgers verify complete real-source output before
and after extraction. External source manifests are loaded before measurement;
the default native/CI path exercises the six independent projections without
collecting CI timing as performance evidence.

Measure all fifteen parser cases separately from full CLI traversal/process work
and actual distinct-child peak RSS. Preserve every distribution, input/source/
binary hash, failure and incompatible corrected outcome. A source scan is not a
compiler export graph; requested allocations are not residency. Native OS/race
checks and finite-host performance evidence do not establish universal optimality.

The first complete CLI cohorts retained small-process drift and modest overhead.
A four-binary rotated/reversed crossover keeps original, containment, extraction
and root-entry-reuse outcomes separate. Reuse `WalkDir`'s already-Lstat-derived
root entry in its first callback instead of issuing another Lstat. Initialize
buffer/encoder only after the root is validated; error callbacks still return
before any output. Existing file/link/FIFO/root/byte regressions and Race tests
pass. The parser/serializer source body is byte-identical to 3df2002, so retain
its separately measured fifteen phases and refresh the full CLI/RSS once for
this wrapper. No timing subtraction or universal speed claim is permitted.
