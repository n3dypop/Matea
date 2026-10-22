# Signed Kitty placement controls

The pinned x/ansi implementation reproduces [x #926](https://github.com/charmbracelet/x/issues/926):
Options.Options serializes Z only when it is positive. Matea's existing
kitty_place accepts i32 and serializes every value, including negative values.
Its persistent kitty_byte_snapshots_and_lifecycle unit test already checks the
exact z=-1 placement bytes; no production correction was necessary.

The [official Kitty specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/#controlling-displayed-image-layout),
checked 2026-10-21, defines signed 32-bit placement z values. Negative values
place content beneath text, with another boundary at -1,073,741,824. This audit
checks control serialization. It does not exercise physical terminal layering.

Two original callers invoke the actual pinned Go Options.String and the actual
Matea kitty_place. All non-test Go kitty source files used from the module cache
match Git faa4adf95555331dfb439b716cc83767ecdadeb4. The private Rust caller's
39 dependency identities are a subset of the native root graph; the caller is
the only extra package. Source files, binary hashes, compiler information and
the 198 root execution inputs are bound in the
[collection context](evidence/kitty-z-index/context.json). No reference source
implementation is copied into this repository.

The independently generated domain covers every i16 value plus 97 signed-i32
boundaries: 65,633 cases. It includes i32 minimum/maximum and neighbors, the two
negative layering regions, zero, and signed power-of-two boundaries. The
[verifier](evidence/kitty-z-index/verify.py.txt) parses compressed raw hex output,
rejects duplicate keys and invalid framing, and checks all unrelated placement
controls. Go and Rust may order controls differently; omitted z means the
specified default zero, so comparisons use parsed control semantics.

| Result | Cases |
| --- | ---: |
| Matea preserves requested z and other controls | 65,633 |
| Source omits a requested negative value | 32,816 |
| Source nonnegative values are semantically equivalent | 32,817 |

[Verification and raw-output bindings](evidence/kitty-z-index/verification.json)
retain all observations. A producer omitting a negative z or changing quiet
mode is rejected by the independent oracle. The original callers are archived
reproductions, separate from maintained application programs.

The issue assessment closes only negative placement-control omission. It does
not close all Kitty options, animation, capability negotiation, physical emulator
support, the whole protocol module or its performance assessment. No image
payload, image engine, codec, decoder, CUDA or image-engine benchmark is added.
