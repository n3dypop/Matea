# Typed Kitty controls and prepared payloads

Status: implemented slice qualified at 0c95c37 with actual-source native,
three-OS/MSRV, matched performance and targeted Miri evidence. Full protocol
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
suite. Miri seed 233 executes these eight actual-source pure tests, including the
unchanged legacy packet/lifecycle test. No real allocator OOM or native FFI
soundness is inferred. [Source and log binding](evidence/kitty-controls/miri/context.json).

Executable examples through the terminal crate and facade validate basic author
usage. These examples only produce bytes and establish no terminal support.
Header parsing/serialization is a capability slice, not a claim that the entire
source Kitty encoder, PNG codec, file writer, animation lifecycle or compiler
export graph is implemented.

## Matched performance of the implemented slice

The [completed collector](evidence/kitty-controls/performance/context.json) retains
105 records, 3,255 calibrated positive intervals and 62 distinct RSS children.
Twenty Rust cases cover five header shapes, five shared valid parses, five
prepared-payload packet shapes, three strict rejection paths and two preflight
errors. Fifteen Go cases use the actual pinned public Options methods; packet
cases compose Options, standard-library base64 and public ansi.KittyGraphics.
This packet comparator is an explicit source-primitive composition, not execution
of upstream EncodeGraphics or its image/OS resource pipeline.

Fourteen cases have identical output bytes or normalized shared parse scalars,
operation results and consumption boundaries. Empty-payload packets differ by
the optional semicolon; the initial preflight rejected their exact-byte assertion
before timing. Both outputs are retained and this case receives no source ratio.
The five Rust-only strict checks likewise receive no source ratio. See the
[output boundary](evidence/kitty-controls/performance/output-boundary.json).

Three orders retain all cases and tails. All 42 eligible case/cohort pairs have
lower Rust p50 and p95 than the comparator on this native macOS host. First-cohort
medians are 654 ns versus 1,469 ns for the full header; 214 ns versus 983 ns for
its valid parse; 2,010 ns versus 3,534 ns for a 3,072-byte direct payload; and
3,902 ns versus 7,338 ns for two exact frame chunks. The full-header parser makes
zero measured heap allocation requests, and complete direct packet assembly
reserves its owned output once. Strict Rust checks absent in the source parser
are part of these costs.

Whole-fixture child RSS median/maximum is 2,113,536/2,129,920 bytes for Rust and
6,127,616/6,324,224 bytes for Go. These are process high-water measurements, not
per-operation resident memory or directly comparable allocation accounting.
Builds, tests, Miri and coverage did not overlap ordinary timings. No affinity,
frequency or ASLR controls, other-host speed, full-module parity, physical
terminal behavior or universal optimum is claimed. The independent
[verifier](evidence/kitty-controls/performance/verification.json) checks all raw
cohorts, oracle outputs, 3,255 intervals and 62 RSS children. The protocol unit's
performance/reference scores remain 50 because its full public scope is partial.

## Exact-source qualification and remaining scope

[All seven required CI jobs](https://github.com/n3dypop/Matea/actions/runs/39581387396)
succeed. The [artifact binding](evidence/kitty-controls/ci-source-bindings.json)
matches 200 inputs to source Git blobs. Every host completes 47 native checks;
each release log includes seven new regressions and the unchanged legacy Kitty
regression. LLVM records 97 paths/104 instances on Linux/macOS and 96/102 on
Windows. Omitted uploaded logs remain explicit. Native device tests pass twelve
cases per Unix host and nine in disposable Windows consoles, without implying
graphics protocol support in those consoles.

The [final assessment-input suite](evidence/kitty-controls/final-native/metrics.json)
also passes all 47 checks: Rust 285 plus 14 doctests, Python 146 with two
conditional skips, Miri eight pure tests and LLVM 97 paths/104 instances. Three
transitive hashbrown zero-hash profile warnings remain diagnosed and retained;
these results do not prove all branches, dependencies or physical terminals.

The [declaration review](evidence/kitty-controls/declaration-review.json) records
all 82 raw Kitty declarations: 64 bounded implemented roles, two partial roles
and sixteen pending roles. Existing raw declarations were already linked to a
partial capability, so this does not artificially reduce unlinked counts.
Full writer/media lifecycle, placeholder/diacritic and codec roles remain
explicit. Source C loss is fixed in Matea's parser. The old #926 negative-z
assessment is revalidated by a byte-identical legacy body, new CI and Miri; its
65,633-case source comparison remains historical and was not repeated.

Four portability dimensions return to 75 after qualification. Whole protocol
reference/performance stay 50; 48 global axes remain below 75 (21 reference,
27 performance), with 822 issue reports pending. Stage A continues; Stage B,
F5, image engine and CUDA remain gated.
