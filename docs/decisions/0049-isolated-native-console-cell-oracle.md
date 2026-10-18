# ADR 0049: Isolated native Windows console cell oracle

Status: planned; no native console result or score promotion yet.

Synthetic cell geometry and three-OS output fixtures do not exercise Windows'
console interpretation of UTF-8 block glyphs and cursor/erase commands. Progress
therefore retains a portability assessment of 50 despite its regular CI success.
Add an independent native oracle rather than changing the evidence threshold.

Generate frames through the existing Progress, View/Surface and FrameEncoder
APIs. A validation-only child process receives a new console and opens its own
CONOUT$ handle, enabling processed VT output and UTF-8 temporarily. Read actual
cells through ReadConsoleOutputW after each frame, including narrower/wider
model sizes and both ASCII and BMP block glyphs. Compare the full padded row;
no ANSI interpreter owned by Matea produces the observation.

Use safe managed handles and explicit checked marshaling only in the test
adapter. Keep native APIs outside Rust production libraries. Validate structure
sizes, clipped read rectangles and all API return values. Save and restore
console modes, output code page, text attributes and cursor state. The console
belongs only to the child, so termination cannot alter a user's existing console.
An error or timeout is failed evidence, never an unsupported-but-passing result.

Microsoft documents VT output as the modern cross-platform path. This test uses
the [classic readback API](https://learn.microsoft.com/en-us/windows/console/readconsoleoutput)
as an independent observation point, not a new production backend. Its Unicode
cell layout follows [CHAR_INFO](https://learn.microsoft.com/en-us/windows/console/char-info-str);
[VT processing](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences)
and [output modes](https://learn.microsoft.com/en-us/windows/console/setconsolemode)
define the written stream. No modern Windows Terminal, SSH/ConPTY, font appearance,
IME, supplementary glyph or historical 2022 reference environment equivalence
follows. This is correctness/portability work, not Stage B performance work.
