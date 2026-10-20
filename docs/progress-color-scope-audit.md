# Progress color/style scope: observations at the pins

This is additional Stage A source analysis. Reference equivalence remains 50;
the observations do not close the public-capability ledger or advance F5.
[Executable calls, raw outputs, hashes and limits](evidence/progress/source-color-scope/context.json).

## Reproduced low-intensity color jump

The pinned x/ansi `shift` helper treats values at most 255 as already 8-bit,
and shifts larger values by eight. Valid Go `color.RGBA64` channels 255 and 256
therefore emit RGB(255,255,255) and RGB(1,1,1), respectively. This loses the
declared 16-bit interpretation and makes conversion nonmonotonic.
[Pinned helper](https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/util.go#L39).

The effect reaches the real public Progress renderer. A three-cell full-block
gradient from RGB(0,0,0) to RGB(1,1,1) emits foregrounds (0,0,0),
(128,129,129), (1,1,1). Pinned Lip Gloss/go-colorful returns middle RGBA16
channels (128,129,129); the ANSI heuristic interprets them as bright RGB8.
Matea's actual Gradient and rendered Progress frame emit (0,0,0), (0,0,0),
(1,1,1). Existing unambiguous 16-bit rounding followed by `>>8` avoids the
brightness jump. Counts 2/3/6/17 are independently checked for bounded,
monotonic Matea channels. This intentional correction must stay explicit in
reference comparisons; do not make the Rust encoder reproduce the heuristic.

The reproduction retains 47 source observations and five actual Matea
projections. Cached ANSI `style.go`/`util.go` bytes match the pinned Git blobs;
the successful executable's build-info records ANSI v0.11.9 and go-colorful
v1.4.1. An optional full unused/test module-graph query fails offline; its
incomplete output stays retained and is not dependency-closure evidence.
No upstream ticket was posted and no statement about later revisions follows.

## Percentage style geometry is still a real scope gap

Source `percentageView` calls `PercentageStyle.Inline(true)`. The Inline
documentation disables borders, padding and margins, but it is insufficient
to infer that every dimension is disabled. Actual public calls with a border,
padding, margin, Width(60), Height(4) yield a two-line label; a model width of
12 does not contain it. MaxWidth(3) instead clips the label to `  5` and changes
the remaining bar width. Preserve these observations instead of assigning
style parity from the six default frame probes.

Matea currently promises a single-line, clipped label. General geometry,
MaxWidth/transform composition, hyperlink metadata and separate whitespace
attributes need explicit equivalence tests or documented corrected contracts.
They remain pending. A generic Style reexport alone is not implementation proof.

## Next implementation boundary

Keep terminal cells/styles RGB8 and avoid enlarging every Color/Cell merely to
accept high-depth stops. Add explicit channel-depth conversion at the input
boundary, and preserve precision only while computing gradient stops. Review
premultiplied RGBA16, zero-alpha handling and prefix/single-stop behavior with
actual source fixtures before choosing public constructors. Inputs, color math
and terminal quantization need separate types/contracts; no image processing,
codec or CUDA work is involved. Record the selected API in an ADR before its
implementation and retain existing RGB8 distributions as the control.
