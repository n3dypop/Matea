# ADR 0076: Clock formatting selection and the validation boundary

Status: experiment review and investigation direction; no production adoption or
public API change. Foundation Stage A continues.

Four arithmetic candidates retain correct duration strings, but fixed/varied
timing shows adverse cases against the unchanged actual API and pinned Timer.View.
Three standalone trials and the broader twelve-fixture comparison retain all
468 records/14,508 intervals. Forced integer inlining helps some workloads;
changing helper boundaries also changes code layout and ordinary small cases.
A fast result on six fixed values is insufficient for selecting a module change.

Keep every candidate private until fixed controls, varying numeric scales and
the actual production compilation boundary have been checked. Preserve adverse
cohorts rather than choosing a favorable aggregate. Keep identical ownership,
output bytes, consumed results and operation boundaries when comparing source.
Whole-module active scheduling and retained memory require separate evidence.

Two dedicated actual-library CPU profiles identify substantial samples in both
numeric formatting and UTF-8/Text conversion. These samples guide investigation,
not exact CPU percentages or an assertion that validation is unnecessary. Retain
checked UTF-8 and terminal-text normalization. Do not introduce unchecked byte
conversion or a public trusted-string escape to make timings look better.

Investigate whether typed construction can preserve these invariants while
avoiding redundant work. Start with a private prototype using safe Rust/string
operations and explicit ownership. If this needs a new builder or conversion
contract, review its crate ownership, arbitrary-input behavior, DX and provisional
public API before implementation in the maintained workspace. No API is locked
in by this ADR, and no renderer-wide optimization phase is started here.

Qualify any later adoption through actual-library comparisons, maintained
regressions, targeted Miri and current-source native/three-OS evidence. Existing
clock issue assessments must be revalidated if their source proofs change.
See [the complete experiment audit](../clock-digit-pair-experiments.md).
