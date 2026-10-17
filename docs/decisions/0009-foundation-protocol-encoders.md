# ADR 0009: Foundation protocol encoding, separate from image processing

Status: owner-authorized scope; implementation contracts provisional.

On 2026-10-16 the owner explicitly included image protocol encoders in Foundation.
This supersedes the earlier encoder prohibition. Image conversion, decoding,
quantization, codecs, image crates and CUDA remain after Foundation acceptance.

Place pure, bounded protocol writers under `matea-terminal::protocol`. They accept
borrowed, already prepared RGB/RGBA or indexed pixels. They neither open a terminal
nor decode image files. The runtime owns the terminal and future placement/resource
reconciliation; a byte encoder never writes during Drop. Separate protocol support
from OS support. Unknown support must fail explicitly; overrides are deliberate.

Start with Kitty direct uncompressed RGB/RGBA transfer, bounded chunking, query,
placement and scoped deletion; Sixel palette transfer with optional transparent
index and repeated-run encoding. A transparent Sixel index preserves existing
pixels; arbitrary alpha blending requires the future conversion engine. Sixel has
no Kitty-style resource deletion, and its cursor/scrolling effects belong to the
transport. No environment variable establishes protocol support.

Primary specifications read before implementation on 2026-10-16:

- [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/),
  source revision `535c7502e83816ef65c8e6624b41ff5ef1203a83`,
  `docs/graphics-protocol.rst` (direct transfer, 4096-byte base64 chunks, queries,
  placement IDs, C=1 and deletion).
- [DEC VT330/VT340 Programmer Reference Manual, chapter 14](https://vt100.net/docs/vt3xx-gp/chapter14.html),
  retrieved HTML SHA-256 `97be785d3f9997e33f0f3ee7c2d8cf59b76f4e6b85ea82bbddf9470e9689de48`.
  Raster attributes, P2 transparency, RGB percentages, sixel bit order, repeat,
  carriage return and six-pixel newline were read from this primary manual.

These are initial capability slices, not full x/ansi equivalence. PNG passthrough,
compression, file/shared-memory transfer, advanced Kitty animation/placeholders,
iTerm2, reply routing and runtime resource reconciliation remain pending in the
coverage ledger. Byte-level tests do not establish real-emulator compatibility.
