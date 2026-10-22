# Corner patterns and rectangular side alignment

Current corner integration is a candidate;full current-root/native/Miri/LLVM/CI
qualification is pending. The previous block increment at f9c34b7 has qualified
3OS/MSRV evidence. Its results are historical and are not relabeled as this code.
The active axis remains matea-style/lib reference50;the baseline47 below-high
axes are unchanged. Five dependent units need current portability revalidation.

[ADR0080](decisions/0080-whole-corner-patterns.md) adds CornerMode::WholePattern
and borrowed named BorderParts input. BorderSet owns validated strings and cached
whole grapheme ranges;clones share storage but have independent immutable mode
metadata. Checked u16 sums reject full corner widths above65535. FirstCluster
remains the default. The13-part array API remains accepted. BorderSet width
queries use the actual rendered corner;empty parts report0 and use a one-cell
space fallback when drawn. Right side glyphs now align to the outer right edge.
Cell/Style fields and the approved root manifests/locks are unchanged.

The pinned source is Lip Gloss19f011f1. An actual observer reproduces
[issue114](https://github.com/charmbracelet/lipgloss/issues/114):an empty body's
middle row is shorter than its wide-corner edges,and requested Width(5) around
`ab\nc` produces width4. Matea reserves complete corner extents and pads adjacent
rows,including the empty-body case. The whole mode supplies the multiple-glyph
corners requested by[issue605](https://github.com/charmbracelet/lipgloss/issues/605).
Default truncated source behavior is preserved by the inherited exact corpus;
new whole patterns and corrected extents use independent literal rectangles.

The isolated prototype passed both style/render suites and doctests,all3573
inherited source observations(with the existing42 explicitly modeled corrections),
all-target Clippy,and new Miri5 methods at seed251. One65535/65536-cell boundary
corpus runs natively and is explicitly ignored under Miri. Additional tests cover
named borrowed input ownership,shared caches,mode changes,iterator lifetime,
combining/CJK/ZWJ crop boundaries,each corner gradient coordinate and signed offset
extremes. Prototype failure logs are retained. These scoped runs are not current
root integration qualification or benchmark results.

[The exact integration record](evidence/block-corners/integration.json) binds the
four adopted prototype files and facade. No source issue is closed until current
integration evidence and its ledger hashes are checked. The large style source
frontier,hyperlinks,raw tabs,precise colors,remaining utilities,conditional exports
and matched module performance remain open. StageB/F5/image engine/codec/CUDA
remain gated. No performance ratio or universal optimum is asserted.
