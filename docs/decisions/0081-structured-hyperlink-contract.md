# ADR0081: Hyperlink values and sparse scene metadata

Status: design proposal for the remaining active style reference axis;not
implemented or accepted as reference parity. Corner qualification is in progress.

Lip Gloss Style.Hyperlink(link,params...) wraps core text before padding/borders.
x/ansi SetHyperlink and ResetHyperlink construct OSC8 sequences. Hyperlinks must
remain structured intent in Matea;applications should not embed escape strings
in normalized Text. Neither terminal support nor safe restoration is universal.

Propose a validated,owned,immutable Hyperlink value with shared URL/parameter
storage. A disabled/default value provides explicit removal;an absent BlockStyle
property still inherits. Reject C0/C1 controls and parameter delimiters that can
escape the structured parameter fields. Preserve caller ordering for deterministic
encoding. Determine empty URI/reset and unknown parameter semantics from fixed
source fixtures before locking this API. Do not restrict URI schemes merely to
work around unrelated concerns or open links from the framework.

Keep Style,Cell and RichText storage unchanged. Put optional sparse intervals in
Surface and annotate the actual body rows in StyledBlock,excluding padding and
borders. Expose whole-grapheme-safe annotation for independent components.
Snapshot copying,overpainting,wide-glyph erasure,clipping,resizing and equality/
damage must include link ownership. An ordered interval map is a candidate:
O(log n) lookup/update without per-cell link pointers or a full metadata grid.
A no-link scene must avoid allocation;benchmark the real complete path before
claiming allocation or speed wins. Review fragmentation/merge behavior and
bounded worst cases before choosing the representation.

FrameEncoder should emit OSC8 only under an explicit supported/declared policy,
with plain text fallback. Treat link-only changes as damage when enabled;close
active links before positional controls and at every frame boundary. Keep policy
independent from color depth and reduced motion. Native owns cleanup and its
retryable lease;FrameEncoder performs no Drop I/O. Existing Native::restore must
attempt hyperlink cleanup even if earlier output fails. Research cancellation of
partially written OSC packets and test every byte failure before asserting recovery;
encoding a complete Vec does not make the physical stdout write atomic.

Qualification requires actual fixed Lip Gloss/x/ansi output and UV link metadata,
independent interval/whole-grapheme models,inherit/reset/control errors,body/frame
scope,composed snapshots,link-only changes,policy changes,fallback and all write/
flush/panic paths. Check source declaration/target conditions and current3OS/MSRV
behavior. Physical emulator interoperability remains explicitly separate.
No score promotion,other active axis,image engine or CUDA work follows this proposal.
