# Typed Kitty controls: implementation candidate

Status: native correctness candidate; matched distributions, targeted Miri and
the changed-source three-OS/MSRV qualification are pending. Full protocol
reference/performance assessments remain partial. No image engine, codec,
decoder, compression algorithm, filesystem/shared-memory creation or CUDA is
implemented by this slice. [ADR 0075](decisions/0075-typed-kitty-control-and-prepared-payloads.md).

Pinned reference: x/ansi `faa4adf95555331dfb439b716cc83767ecdadeb4`,
`ansi/kitty/options.go`, `graphics.go` and the packet policy in `writer.go`.
The pure Rust module is `matea_terminal::protocol::kitty`, also reachable through
the public Matea facade. Existing legacy encoders retain their implementations.

| Source capability | Rust contract and remaining scope |
| --- | --- |
| Options control fields, action/format/transmission/delete constants | Copy Control with typed enums, optional nonzero identities, protocol-width unsigned fields and signed z/relative offsets |
| Quiet and deprecated Quite alias | One Responses enum; no ambiguous duplicate/misspelled state |
| Options/String/MarshalText | Deterministic nonmutating Display; std formatting can target text/byte writers without per-field owned strings |
| Stringish/UnmarshalText | Bounded parse of str/String/byte storage and FromStr; complete new value or typed error; strict duplicates/unknown keys/enum/range errors are deliberate differences |
| Chunk and ChunkFormatter | Explicit per-packet more state, automatic direct chunking and composable owned packet iterator; partial allocation failure semantics are documented |
| File and transmission policy | Prepared names are payloads, separate from header state. Pure serialization neither opens nor creates their OS objects; full source writer/lifecycle equivalence remains pending |
| Format/compression metadata | Prepared RGBA/RGB/PNG/compressed data only; pixel conversion and codec algorithms remain gated |
| New primary-spec N/H/V controls | Retained usage mask and signed relative offsets; source Options does not expose these fields |

The [primary specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
was read 2026-10-21. Chunked data is base64 framed, and frame continuations retain
their action/response policy. Encoding does not negotiate support. Declared
data-action dimensions and total output bytes are bounded; animation-control
s/v values keep their different meanings. Undeclared geometry and prepared-format
coherence remain the producer/terminal's responsibility. A packet sequence's
budget is checked before output allocation; ownership of terminal writes stays
outside the encoder. No physical-emulator or shared-memory interoperability
claim follows from the pure packet tests.

## Source observation: cursor policy lost on text round trip

An actual public Options value with image 7, C=1, 40 columns, 12 rows and action p
marshals to `i=7,C=1,c=40,r=12,a=p`. UnmarshalText into a fresh value returns nil
but produces `i=7,c=40,r=12,a=p`, with DoNotMoveCursor false. The pinned numeric-key
dispatch omits C. The [retained original caller and observation](evidence/kitty-controls/source-cursor-policy/context.json)
bind all five production Kitty source files to their prior fixed-Git snapshot.
No upstream issue was posted or modified, and this is not presented as a
maintainer-confirmed defect. Rust explicitly parses both C values and retains
the policy in the literal placement packet.

## Native candidate checks

Six integration tests cover every scalar role against a literal header; all 256
byte choices for action, transmission, deletion and compression; unsigned/signed
boundaries; Boolean/format/response domains; malformed/duplicate/unknown controls;
safe error display; full-header bounds; byte budgets; direct chunk boundaries;
frame continuations; fused exhaustion; explicit last-packet state; prepared names;
and declared geometry limits. A separate internal reservation-failure injection
test exercises owning failure before publication, budget rejection before
reservation, and the iterator's one error after a successful prefix. The real
allocator is not forced into OOM. Independent bit-accumulator decoding reconstructs
the original payload. Legacy encoder/terminal tests pass in the native selected
suite. Matched performance distributions, Miri and
other-host execution still need qualification; missing evidence is pending.

Executable examples through the terminal crate and facade validate basic author
usage. These examples only produce bytes and establish no terminal support.
Header parsing/serialization is a capability slice, not a claim that the entire
source Kitty encoder, PNG codec, file writer, animation lifecycle or compiler
export graph is implemented.
