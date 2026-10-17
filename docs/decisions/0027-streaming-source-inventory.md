# ADR 0027: Stream source inventories and retain output failures

Status: accepted for the internal source-declaration auditor.

## Decision

The Rust auditor writes its unchanged JSONL through one owned `BufWriter` over
a locked stdout. Pass `&mut impl Write` through traversal and declaration emission;
return the original I/O error at any failed write and explicitly flush before
reporting success. No worker/global buffer or new dependency is introduced.

Format JSON string fields from borrowed `&str` segments using a private `Display`
adapter. The UTF-8 scan only slices at `char_indices` boundaries. Copy unescaped
segments as a whole and preserve the previous quote, slash and control escapes.
This avoids an owned temporary String for each file/kind/symbol/cfg field.

## Evidence

Each complete process includes spawn, warm filesystem reads, Syn AST parsing,
JSON generation, pipe output, flush and exit. One warm process is excluded, then
31 samples are retained. JSON validation/hashing happens outside the timer. The
two real inputs are clean pinned Crossterm (613 declarations) and Ratatui
(1,960); a separately generated 10,000-constant input supplies an independent
expected record for every declaration. Complete output bytes match all stages.

| Input | Before p50 ms | Buffered p50 ms | Borrowed JSON p50 ms |
| --- | ---: | ---: | ---: |
| Crossterm | 34.335 | 33.857 | 33.568 |
| Ratatui | 171.671 | 170.062 | 169.137 |
| 10,000 constants | 67.124 | 60.740 | 58.194 |

Ratatui p95 increases from the buffered stage's 171.390 ms to 173.308 ms;
both distributions are retained. The source parser dominates these real inputs;
these are small complete-process improvements, not a claim that parsing or
all development tooling became fast. Allocation/RSS and cold filesystem costs
remain unmeasured. No ratio to an upstream framework is appropriate for this
internal tool.

Three native Rust tests cover Unicode/control escaping, output errors at every
byte of a nested finite fixture, and a final flush failure. Existing Python
fixtures independently parse the emitted JSON and check platform/feature fields.
Source declaration candidates remain distinct from compiler-resolved exports.

## Measurement corrections

A first flush test wrongly required nonempty output from this private-only CLI's
own source. Its failure is retained and the assertion corrected; empty inventories
must propagate flush errors too. A failed test prevented a chained release build,
after which a stale executable was measured. That purported after run is retained
as `discarded-stale-binary-control*` and is excluded from optimization conclusions.
Its dirty source context cannot pass committed-source verification.

The measurement driver now requires committed input hashes, builds the exact
release executable itself with `--locked --offline`, and stops on build failure.
The refusal for uncommitted source is retained. Valid before/intermediate/after
contexts contain Git, source, binary, raw and upstream-ledger hashes.
