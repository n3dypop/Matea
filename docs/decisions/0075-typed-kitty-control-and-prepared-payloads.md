# ADR 0075: Typed Kitty controls and prepared payloads

Status: implementation decision; API provisional; full protocol parity pending.

The existing encoder only exposes a fixed direct-transfer header and a few
placement/delete helpers. Pinned x/ansi `faa4adf95555331dfb439b716cc83767ecdadeb4`,
`ansi/kitty/options.go` exposes additional control, format, transmission, display,
deletion and text conversion capabilities. Its source field names are not a Rust
API prescription. Preserve the existing helpers and add
`matea_terminal::protocol::kitty` for advanced pure protocol composition.

Use enums for action, format, transmission, responses, compression and deletion;
use protocol-width integers and optional nonzero identities. A Copy control value
contains no file handle, borrowed payload, closure, background task or shared
mutable defaults. `Display`/`FromStr` and a byte parser provide text conversion;
serialization does not mutate the caller to install implicit defaults. Do not
carry forward the deprecated misspelled quiet field.

Keep transport policy separate from a packet's explicit `m=0/1` value. A bounded
packet iterator lets callers compose per-packet envelopes using ordinary Rust
iterator operations rather than storing an opaque formatting callback in state.
An owning encoder returns a complete packet sequence or an error; incremental
iteration must document possible earlier successful packets before an error.
Both paths share the existing checked base64/byte-budget primitives.

Minimal advanced usage, with the terminal session retaining ownership of output:

```rust,ignore
use matea::terminal::protocol::{Limits, kitty::{Action, Control}};
let control = Control {
    action: Action::Put,
    image_id: std::num::NonZeroU32::new(7),
    columns: 40,
    rows: 12,
    ..Control::default()
};
let bytes = control.encode(&[], Limits::default())?;
// The application's terminal transport serializes bytes under its session lease.
```

The [primary specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/),
read 2026-10-21, defines 32-bit controls, signed stacking/relative offsets,
base64 payloads and 4,096-byte direct chunks. Subsequent frame chunks retain
their action and response policy. File/shared-memory names are protocol payloads;
encoding a name establishes neither its existence nor a shared filesystem.
PNG/compression flags describe already prepared bytes. No codec, decoder,
compression algorithm, conversion/quantization engine, file/shared-memory
creation or CUDA implementation is introduced. These remain outside this slice.

Parse into a new value and reject malformed, duplicate, unknown, out-of-range or
control-containing input rather than silently accepting a partially updated
receiver. In particular, preserve signed z values and the cursor policy `C`.
Pinned source inspection finds that `UnmarshalText` omits `C`; reproduce this
before calling it a confirmed source observation. Strict parsing, explicit chunk
state and immutable defaults are intentional Rust contract differences.

The advanced controls describe wire parameters. Some keys have action-dependent
meanings; full image/animation semantic validation belongs to higher-level
constructors and the terminal's acknowledgement. Existing validated raw-pixel
helpers remain the simple entry point. Protocol encoding never detects support,
opens a terminal, writes during Drop or bypasses capability/fallback ownership.

Validate canonical source outputs, parser limits and errors, all byte enum
choices, numeric boundaries, duplicate/unknown keys, signed coordinates,
chunk boundaries, fused error iteration and unchanged legacy packets. Measure
header/packet construction separately from parsing and output copying. Native
bytes and CI do not prove emulator, filesystem, animation or shared-memory support.
