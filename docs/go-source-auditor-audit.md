# Go source inventory: containment and measurement

The private Go inventory tool now rejects linked/non-directory roots, skips
linked and nonregular source entries, and rejects relative paths whose bytes
cannot be represented losslessly in JSON. Before the fix, an external `.go`
symlink published an outside declaration, a directory symlink root succeeded
with empty output, and a FIFO blocked. [Retained negative observations](evidence/go-auditor-containment/before.json).
The contract assumes a fixed source tree; it is not protection against concurrent
filesystem replacement. An explicitly selected directory named `examples` is
scanned; descendant example directories retain the inventory exclusion policy.

`WalkDir` already obtains the root's entry without following links. Its first
callback validates that entry before constructing the output buffer/encoder,
avoiding a duplicate root lookup. The private `auditSource` function separates
AST parsing, declaration extraction and NDJSON serialization from discovery,
reading and process startup. Syntax errors publish no rows from that file;
short-write and final-flush errors remain observable. The nine pinned Go
corpora retain exactly 4,882 declarations and identical complete output.
[Regression tests](../tools/audit-go/main_test.go), [independent CLI tests](../scripts/test_source_auditors.py),
[ADR 0065](decisions/0065-aligned-fixed-tree-contracts-for-go-source-inventory.md).

## Measured scope

The measurement test uses the existing upstream Go caller byte for byte, checked
by a native test. Each phase has 100 warm-up calls, a confirmed 20 ms calibrated
batch, 31 retained observations, and separate requested-allocation/byte passes.
Six independently constructed constant/struct cases cover 1,000, 10,000 and
100,000 declarations; nine cases cover all pinned Go repositories. Preloaded
source bytes exclude filesystem reads from these fifteen parser phases. Full
owned NDJSON, byte counts and FNV checks prevent measuring only a count.
[Method and source identity](decisions/0066-go-source-phase-measurements-with-common-caller.md),
[all phase and balanced CLI records](evidence/go-auditor-phases/report.json).

Four complete CLI cohorts retain 84 records across 21 cases, including startup,
all source corpora, scale, wide/deep trees, links, malformed input and missing
roots. They retain 2,604 distinct-child peak-RSS observations, independently read
from native `/usr/bin/time -l` logs. A rotated/reversed four-binary crossover
retains another 40 records/1,240 warm observations to distinguish containment,
parser extraction and root-entry reuse from startup drift. Small-process
differences and adverse tails remain visible; this is not a broad speedup claim.
[Crossover context](evidence/go-auditor-phases/root-crossover-context.json).

The final root-wrapper supplement at `4bbf317` retains another 21 CLI records
and 651 RSS observations. Its parser body is byte-identical to the separately
measured `3df2002` body; only that phase evidence is reused. Selected complete
CLI results follow; the linked report retains every case and p95.

| Case | p50 ms | Median child peak RSS MB |
| --- | ---: | ---: |
| 1,000 constants | 4.670 | 6.60 |
| 10,000 constants | 13.612 | 14.19 |
| 100,000 constants | 95.266 | 71.57 |
| 100,000 struct fields | 121.962 | 59.31 |
| Ultraviolet corpus | 23.013 | 11.88 |
| 1,000 source files | 20.947 | 8.86 |
| Depth 64 | 5.570 | 5.67 |

[Final complete-CLI source binding and output checks](evidence/go-auditor-phases/root-final-report.json).
All 3,255 raw RSS logs are retained losslessly in per-cohort archives, with
logical member hashes and archive-aware reproduction commands. Allocation
requests are not peak residency; neither substitutes for the other.

These are bounded measurements on one macOS arm64 host, with builds, native
tests, LLVM and Miri excluded from timing cohorts. They establish neither a
Rust/Go framework ratio nor a universal optimum. Native Linux invalid-byte and
Windows directory-junction regressions require exact-source CI qualification;
Windows file symlinks/FIFOs and physical terminal behavior remain unverified.
Lexical inventory is not a compiler-resolved public export graph, and does not
complete Foundation capability parity. Stage A, Stage B and F5 remain closed.
