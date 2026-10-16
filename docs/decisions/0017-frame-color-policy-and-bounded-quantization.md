# ADR 0017: Frame color policy and bounded quantization

Status: implementation decision; finite styled-frame evidence, no full-module
performance or terminal compatibility claim.

FrameEncoder receives explicit capabilities. Resolve NO_COLOR/environment policy
when constructing those capabilities, then serialize colors without process-global
state. Crossterm 0.29 Colored's formatter consults a memoized global NO_COLOR/
force-color flag; using its foreground/background commands inside a pure writer
can emit empty SGR resets despite an explicit ANSI/truecolor capability. Preserve
the observed failing traces, and encode the closed Matea Color values directly.
Defaults still respect NO_COLOR through Capabilities::from_environment; an explicit
override is intentional. Never force Crossterm's global flag as a workaround.

Repeated RGB quantization per cell dominated the ANSI256 full-frame fixture.
Memoize at most eight source/adapted Style pairs in each exclusive FrameEncoder,
keyed by color level and all style values. No global growing map, lock or heap
allocation is needed. Clear the logical cache on quantization-level changes;
keep it across physical cursor/style invalidation because nominal palette mapping
is pure and coordinate-independent. Bypass memoization for already-supported
colors/levels. If adaptive/configurable palettes are introduced, their policy
generation must join the key before these entries can be reused.

Skip unchanged ASCII cells before color adaptation. Emit the four supported fixed
attribute SGR sequences directly, avoiding Attribute::sgr's temporary Strings;
differential tests compare every supported combination with Crossterm's ANSI
formatter. Cache tests exercise eviction, attributes, level changes and recovery
against the uncached Style::adapt and fresh encoders.

Keep None/TrueColor adaptation as direct inline value operations. The initial
cache change added roughly 8/9 percent p50 overhead to these controls; retain that
intermediate run rather than discarding it. The direct paths remove the observed
regression. Cache-thrashing and arbitrary palette distributions remain unmeasured;
eight entries are a bounded starting policy, not a universal optimum.

For a known cursor, the next row's column zero uses CRLF, including a delayed-wrap
position. CR cancels pending wrap before LF; requested coordinates are within the
surface, so the encoder never advances below its last row. Unknown position uses
absolute addressing; the home anchor uses CSI H with default parameters. Recovery
still resets physical pen, cursor and visibility. Regression tests include the
bottom row, and independent replay rejects scrolling below the fixture viewport.

Four matched full-frame cases cover ANSI16, ANSI256, truecolor and monochrome.
Both implementations recover unknown cursor/style/visibility, retain their color
lookup caches and write to warmed memory sinks. The Go harness invalidates cursor
position and explicitly resets pen/visibility before Redraw; it does not compare a
known-state redraw against Matea's stronger recovery operation. Independent replay
checks both initial and recovered frames, including lower blank backgrounds and
hidden cursor. Selected RGB inputs yield the same profile output; arbitrary nearest
color algorithms and configurable terminal palettes are not equated. Retain all
before/after p50/p95, allocation and output-byte evidence.

The retained ANSI256 fixture changes from p50 1,575,277.5 ns / p95 1,581,835 ns
to 28,472.5 / 31,150 ns, with measured allocations 2 to 0. All four profiles use
zero measured allocations after these combined changes. CRLF/home reduce each
full-frame fixture by 90 bytes; monochrome is 1,655 bytes in both implementations.
These are one-host warm-cache memory-sink results, not real-terminal latency or
full-module parity. See [the audit](../frame-color-audit.md) and raw distributions.

Sources: released Crossterm 0.29 style/types/colored.rs:75–101 and
style/types/attribute.rs:177; pinned Ultraviolet terminal_renderer.go:597–620 and
cell.go:434, colorprofile v0.4.3 profile.go. Go's process-wide color map caches
comparable color values without eviction; Matea's closed value Color and bounded
per-writer ownership avoid that unbounded/global-state design. This is terminal
styling, not image conversion or an image-engine benchmark.
