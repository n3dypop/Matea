# Source inventory: containment, lifetimes and measured phases

This is the internal declaration-inventory tool, not framework throughput or a
compiler-resolved public API claim. The complete Foundation reference ledger and
quality gate remain open.

The prior recursive directory walk followed external links and cycles. A fixed
negative fixture returned 31 declarations including 15 outside its root, rather
than one local declaration. The current explicit DFS skips links and non-regular
entries, rejects linked/non-directory roots and rejects non-UTF8 relative IDs.
It preserves deterministic file ordering. The contract is a fixed tree, not an
adversarial concurrent-filesystem sandbox. Windows junction, Unix FIFO and Linux
invalid-path-byte fixtures are platform-specific; APFS rejects creation of the
invalid-byte fixture.

The pinned parser stack is Syn 2.0.119, Quote 1.0.47, proc-macro2 1.0.107 and
unicode-ident 1.0.27. No macro expansion or evaluated export/cfg closure is implied.
[ADR 0062](decisions/0062-fixed-source-tree-walk-and-auditor-phase-measurement.md)
records the CLI/library boundary and traversal contract.

Attribute formatting first removed duplicated token groups and temporary joined
strings. The retained `before`/`after` observations improved parsing but left
about 481 MB flat / 579 MB nested peak RSS for 100,000 declarations. Keep these
results and the failed sandbox RSS attempt; they do not justify a high score.

The next implementation drops each parsed item before reading the next and
handles module bodies without retaining their complete item vectors. NDJSON
bytes are retained until file syntax is valid. Later syntax errors publish no
partial declarations from that file; successful earlier files remain visible.
Short writes and output/flush failures propagate. BOM and uncommon shebang
prefixes preserve Syn behavior. The shebang fallback still retains a full AST.

The completed `baseline-1`, `streaming-1`, `streaming-2`, `baseline-2` sequence
compares 2948000 with 222a2f9 using identical benchmark callers/dependencies and
independent full-byte checks. It retains 32 parser timing records, 32 output-only
records, 52 complete CLI records and 1,612 distinct-child RSS observations.

| 100,000 declarations | Whole-AST CLI p50 | Incremental CLI p50 | Whole-AST peak RSS | Incremental peak RSS |
| --- | --- | --- | --- | --- |
| Flat | 531–535 ms | 473–475 ms | 481 MB | 182 MB |
| Nested | about 643 ms | 508–509 ms | 574 MB | 234 MB |

All cohorts and adverse tails are in the
[verified comparison](evidence/auditor-completion/streaming-comparison.json).
The 1,000-tiny-file case rose from 22.39–22.61 to 22.89–23.05 ms. An attribute-only
implementation also allocated less than incremental parsing on some flat cases:
Syn's public cursor clones identifiers during module lookahead. These costs are
retained and distinguished from the substantial AST residency reduction.

[ADR 0063](decisions/0063-incremental-source-items-with-file-atomic-output.md)
evaluated a complete-AST path up to 8 KiB. Exact boundary/Unicode and both-path
syntax/short-write tests passed, including a targeted Miri observation. Its
balanced trial did not establish a stable tiny-file gain, added 1,426 allocator
requests on Crossterm and 7,853 on Ratatui, and increased RSS near the boundary.
The experiment is **rejected**; [all four cohorts](evidence/auditor-completion/adaptive-comparison.json)
remain intact. A green seven-job CI for 9c93ed1 does not override that verdict.

[ADR 0064](decisions/0064-reuse-directory-entry-types-with-owned-walk-state.md)
restores uniform incremental parsing and reuses `DirEntry::file_type()` in owned
DFS state. No directory handles are retained; unique sibling names allow an
unstable deterministic sort. It also fixes a reproduced POSIX path collision:
`a/b.rs` and `a\b.rs` previously produced identical rows. The CLI preserves
OS-native root arguments while requiring UTF-8 relative IDs. Independent POSIX
identity/order and Linux non-UTF8-root regressions supplement the link/FIFO tests.

The 88781a7 implementation retains the exact parser/serializer prefix bytes from
222a2f9. Eight parser projections are rechecked, while the prior separately scoped
parser timings remain explicit. Fresh complete CLI and child RSS collection uses
16 cases and `dir-control-1`, `direntry-1`, `direntry-2`, `dir-control-2` order.
All four cohorts are independently checked. The bounded utility performance assessment is 75; this is not a framework speed/parity verdict.

Measurements separate existing-string parser/serialization latency and allocator
requests from complete CLI latency and real child-process RSS. The parser uses
the common allocator forwarding wrapper; the CLI uses the ordinary allocator.
The first retained call follows correctness setup and is not cold-filesystem
proof. macOS `/usr/bin/time -l` observations are separate from timing, not
cumulative parent-process resource counters. One host, 31 observations, no
frequency/affinity control, finite source scale: no universal maximum-speed claim.

Archived measurement, reproduction and independent-verification procedures live
in [the evidence directory](evidence/auditor-completion/procedures). Earlier
Miri observations retain their own source hashes; the 9c93ed1 byte-boundary test
passes one seed. No full-workspace Miri claim is made. LLVM's diagnostic identifies
three missing hash=0 profiles in transitive Hashbrown hasher methods, outside
repository source paths; [the complete warning and limits](evidence/auditor-completion/llvm-diagnostic.json)
remain explicit.

The short crossover alternates three binaries through all six order permutations
per case (30 records, 930 warm samples). Whole-AST→current medians are 23.24→21.62 ms
for 1,000 files and 4.57→4.40 ms for 64 levels. Empty startup remains 2.75→2.80 ms;
all 31-sample arrays and adverse p95s remain in
[the raw crossover](evidence/auditor-completion/crossover-cli.jsonl). No startup
latency is subtracted from the primary CLI results. The
[88781a7 CI](https://github.com/n3dypop/Matea/actions/runs/39415338843) passes all
seven required jobs, independently bound in
[its evidence](evidence/auditor-completion/ci-88781a7/source-bindings.json).
Later Go-auditor containment changes require fresh whole-tree qualification.
