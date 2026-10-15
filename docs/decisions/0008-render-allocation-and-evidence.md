# ADR 0008: Inline cells and independently checked performance evidence

Status: accepted for the reopened Foundation implementation; APIs remain provisional.

The source inventory exposed substantial coverage gaps. A benchmark must identify
one workload, its source revisions, output oracle and resource metric. Six tiny
workloads cannot establish parity for 94 reference modules.

Use `CompactString` for cell graphemes, borrowing sanitized input when possible,
an ASCII path alongside grapheme segmentation, borrowed damage iteration and a
reused terminal output buffer. Common glyphs need no individual heap allocation;
long combining sequences retain the full grapheme rather than being truncated.
Keep unsafe allocation instrumentation confined to the research executable.

Compare fresh 80×24 surfaces and separately compare existing-frame transport to
a reused memory sink. Replay the ASCII transport with an independent, deliberately
limited cursor/erase interpreter. Record output bytes as well as timings: the
first comparison exposed 37 versus 2 bytes for one-cell updates. Add a stateful
FrameEncoder that caches cursor and adapted style, uses CR/BS where exact, and
invalidates after external output, capability changes or writer errors. Native
transport owns this encoder; the convenience write_frame helper starts fresh.
Re-test byte equality after the optimization. The borrowed identical-surface case is an optimization, not a general
claim about newly allocated equal frames. Do not extrapolate microbenchmarks to
terminal latency, Unicode completeness, scheduling or the full upstream suite.

Compact strings add a dependency and do not reduce the fixed cell array much.
Hash/interned glyph stores and packed structure-of-arrays remain alternatives;
they would add lifetime or indirection costs without current evidence.
