# Reference coverage ledger

This is a **source inventory and incomplete capability mapping**, not a parity
claim. F5 is closed. `python3 scripts/check_coverage.py --gate` currently fails.

| Pinned reference | Source directory groups | Declaration candidates | Source issue markers |
| --- | ---: | ---: | ---: |
| Bubble Tea | 1 | 405 | 7 |
| Lip Gloss | 5 | 518 | 1 |
| Bubbles | 18 | 873 | 22 |
| Huh | 5 | 572 | 9 |
| Glamour | 5 | 275 | 1 |
| Harmonica | 1 | 20 | 0 |
| Ultraviolet | 6 | 968 | 45 |
| Ratatui | 31 | 1960 | 9 |
| Crossterm | 16 | 613 | 17 |
| x/ansi | 5 | 1238 | 6 |
| x/term | 1 | 13 | 0 |

Totals: 94 directory groups, 7,455 declaration candidates, 117 markers. Directory
groups are navigation units, not a resolved Rust module graph. Exported methods
on private types may be included; macro-generated APIs, reexports, promoted Go
methods, cfg/platform availability and public reachability require further review.
Tests/benchmarks and private internals must be classified, not silently counted as
public user features. The API tools record locations and conditions without copying
implementation bodies. All sources remain in external reference checkouts.

`capabilities.json` maps 401 declaration candidates to initial capability slices;
7,054 remain unlinked. A link does not mean the whole declaration is implemented:
partial Kitty/Sixel and paginator/key coverage explicitly stays partial. The raw
inventory's initial `unmapped` labels are preserved; capability links live in the
separate file so regenerating an inventory cannot erase manual evidence.

`upstream-issues.json` records 841 open reports as public metadata. Their status is
unconfirmed until investigated; initial analysis is in `../upstream-defects.md`.
Each source marker has a fixed-SHA link. Current issue URLs can change independently
of the pinned source. Closed issues have not been enumerated.

Build `tools/audit-go/main.go` to a temporary executable and `cargo build -p
matea-api-audit`, then run `scripts/audit_references.py --references ../references
--go-tool /path/to/matea-audit-go`. The script validates the reference SHAs and
clean checkouts. `scripts/fetch_upstream_issues.py` requires authenticated gh access;
full issue bodies stay in a temporary research cache, outside the repository.

The complete remaining execution order is in `../coverage-plan.md`. Normal CI
validates links and current code; it does not pass the strict coverage gate.

`module-verdicts.json` records Harmonica, Bubbles paginator and key as implemented
and measured within their documented Rust contracts; 91 groups remain pending.
The three reviewed groups cover 66 declarations and have explicit correctness paths,
matched cases, source audits, deviations and limits. Forty-six total workloads
do not establish all-reference or universal performance parity. The strict gate
validates these links and additionally checks per-module correctness/performance,
issue-report dispositions and source-marker assessments, rather than relying only
on declaration links. Initial manual findings need to be reconciled into that
ledger as their reproduction and performance limits are resolved.

TextInput adds 77 declaration links split into editing, suggestions, presentation
and lifecycle capabilities. The latter two stay partial; cursor appearance/blink/
clipboard/light defaults remain pending. Native position/visibility adds 14 links,
with clipping, composed layout coordinates and cursor-only transport tests. Whole
source Cursor types remain partial because their appearance fields are pending.
The source inventory regeneration keeps
manual marker assessments only when their fixed-SHA URLs still match.

Key bindings review all 17 declarations, with generic borrowed Eq tokens, custom
help/enable/unbind and short inline metadata. Twelve matched workloads include
shared/distinct string storage, construction and getters. Empty retained storage
is disabled deliberately. Timed vim-style sequences are an upstream extension
request rather than a pinned public feature; see `../key-bindings-audit.md`.
