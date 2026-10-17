# ADR 0026: Bounded protocol header formatting

Status: accepted for the owner-authorized pure encoder slice (ADR 0009).

## Observation

The new 41-case protocol probe consumes already prepared borrowed RGB/RGBA or
indexed bytes. It includes validation, complete packet allocation/encoding/drop,
chunk boundaries, Sixel band boundaries, transparency, lifecycle controls and
bounded query-response classification. It performs no conversion, quantization,
decoding, file access, terminal I/O or image-engine work.

Temporary `format!` strings were allocated for every Kitty transfer header and
Sixel palette/color/run header. An 80 by 24 varied 16-color Sixel fixture required
711 allocation calls and 9,568 cumulative requested bytes, including its returned
packet and masks. Median encoding time was 29,785.00 ns on the recorded host.

## Decision

Format internal numeric headers into a private 128-byte stack buffer using safe
`fmt::Write`, checked addition and checked mutable slices. Every complete header
is copied into the limited packet once. This removes temporary heap strings
without making packet capacity follow each small formatter fragment. The
buffer's lifetime ends within the call; it introduces no borrowed public state,
global caches, unsafe code, dependencies or public API changes.

The existing fixed numeric headers fit within this bound, including maximum
u32 identifiers/dimensions and usize run counts. Formatting overflow or formatting
failure returns `EncodeError::Limit` before appending any partial header. Packet
limits and the existing all-or-error return contract remain in force. Future
headers with arbitrary text must revisit the bound rather than truncate data.

## Evidence and limitations

All 41 before/after operation contracts, outputs, iteration counts and return
values match. The varied Sixel case changes from 711 to 9 allocation calls,
9,568 to 5,870 requested bytes, and median 29,785.00 to 14,820.84 ns. The
transparent variant changes from 673 to 9 calls and 28,411.68 to 13,622.50 ns.
A one-pixel RGB Kitty transfer changes from 3 to 2 calls and 517.50 to 297.50 ns.
Unchanged lifecycle/reply paths and adverse samples remain in the report.

The buffer-overflow/exact-fit/maximum-number test checks bounds and no partial
append. Existing independent subset interpreters check 20 Kitty inputs and 54
Sixel inputs, chunk and band edges, transparency and exact output limits.
Targeted Miri covers the new buffer test. This is not a general image decoder,
emulator interoperability result, matched Go performance ratio, resident-memory
measurement, or proof that every upstream graphics feature is implemented.
