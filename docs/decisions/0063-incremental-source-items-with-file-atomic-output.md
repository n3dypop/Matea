# ADR 0063: Incremental source items with file-atomic output

Status: implemented; balanced measurements and exact-source CI pending.

The source auditor previously retained a complete `syn::File` while extracting
declarations. Removing redundant attribute token groups improved allocations and
latency, but the retained 100,000-declaration observations still reached about
481 MB (flat) and 579 MB (nested) peak RSS. Attribute-only tuning does not address
the dominant lifetime. Preserve those adverse results, rather than raising the
performance score from the latency improvement alone.

Keep the pinned Syn parser and parse one item at a time. Recognize module
boundaries using Syn's borrowed token cursor, parse module headers with Syn,
then recursively consume the body without constructing its complete item vector.
Drop each other item after extraction. Modules hidden by the existing lexical
test filter still receive syntax validation. This is a declaration inventory,
not a replacement Rust parser or compiler-resolved export graph.

Retain per-file NDJSON bytes until parsing succeeds, then use `write_all` to
publish them. A later syntax error must not leak declarations from the invalid
file, and short writes and I/O errors remain observable by the caller. Successful
earlier files are not rolled back. Uncommon shebang/comment prefixes use Syn's
whole-file disambiguation; this fallback retains the previous AST memory cost.

Memory remains proportional to source tokens, output bytes, and the largest
individual item. Nested module parsing remains recursive. This change is neither
constant-space parsing nor a bound on hostile syntax depth or concurrent filesystem
mutation. It adds no unsafe code and preserves the existing UTF-8/schema/order
contract. Compare all emitted bytes with whole-file parsing and independent
fixtures, including inner/outer attributes, doc comments, visibility, raw names,
external/unsafe/test modules, BOM, shebangs, malformed bodies and output failures.

Measure unchanged callers and dependency versions, separately recording parser
latency/allocations, complete CLI latency, and distinct-child peak RSS. Use a
baseline/changed/changed/baseline sequence, keep cold and all 31 warm observations,
and retain raw outcomes and adverse tails. Shared-runner CI timings are execution
checks only. No score or maximum-performance claim follows from a single RSS
observation.

The first balanced trial confirmed 481→182 MB flat and 574→234 MB nested CLI
peak RSS at 100,000 declarations, with improved large-file latency. It also found
small-file overhead: 1,000 tiny files rose from 22.39–22.61 to 22.89–23.05 ms.
Syn's public cursor clones identifiers, and per-file output buffering adds work.
Retain this trial; do not discard the adverse small-file path.

Use the complete-AST path for inputs up to 8 KiB and incremental items above it.
This explicit byte budget bounds the small-file AST lifetime without depending on
an unmeasured item-count heuristic. It does not bound syntax nesting. Verify the
exact boundary and non-ASCII spans, test syntax/short-write behavior on both
paths, and measure small/wide/deep inputs as well as the unchanged large-file
controls. The threshold is a provisional engineering budget, not a proven optimum.
