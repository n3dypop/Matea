# ADR 0067: Progress lifecycle, scale and owned-cell candidate

Status: implemented; native distributions complete; current Miri/CI qualification in progress.

Retain the existing six matched complete-frame workloads and six distinct
borrowed-cell controls. Fill the measurement gaps with nine Rust lifecycle
controls and six width/formatter scale controls. Each lifecycle operation includes
fresh construction/drop, every named state mutation and its validation; no setup
cost is subtracted. Include error rollback, obsolete ticks, repeated retargeting,
schedule/reduced-motion behavior and an animation reaching exact rest. Scale
covers zero/one/1,024/65,535 cells and 8 KiB ASCII/Unicode/control labels.

These additional Rust API contracts do not have an identical reference hot loop
and receive no cross-language speed ratio. Preserve all cold calls, 31 confirmed
batch samples, requested allocations and independent child RSS separately. Keep
ordinary timing separate from native builds, LLVM, Miri and downloads. CI output
validation is not performance qualification.

Investigate replacing the colored-cell View-node sequence with an immutable
owned Surface using the existing renderer extension contract. Preserve the
widget's fixed layout dimensions, complete cell styles, safe UTF-8/grapheme
handling, clipping and callback order. Compare against the unchanged public
frame probes before selecting the implementation. Additional eager cell storage
and borrowed/consuming rendering costs must be retained, including regressions.
No new unsafe code, mutable shared scene, renderer callback or terminal lease is
needed. Do not promote the module merely because six static frames are fast.

The first full-width snapshot reduces requests but retains unnecessary cells
when the bar is mostly empty. Keep only the colored segment as a Surface and
compose it with the existing empty/label text in fixed layers. This preserves
whole-text grapheme boundaries and avoids a density heuristic. The old public
composition independently checks nested/partial layouts and callback order.

The first single-pass normalizer uses CompactString pushes and regresses in
latency despite fewer requests. Retain that failed performance attempt. Use a
standard owned String for normalization, then move it into the label container;
complete normalization precedes grapheme clipping. Compare all C0/C1 controls
and representative clusters with the prior complete-text contract. Final
measurements resolve the observed regression and preserve adverse unchanged
controls. Five supplementary controls close configuration, clocked mutation,
subscription, Component adapter and iterator measurement gaps. See the
[scoped performance review](../progress-performance-audit.md).
