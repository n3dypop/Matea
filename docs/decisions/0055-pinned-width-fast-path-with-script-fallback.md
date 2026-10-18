# ADR 0055: Pinned width fast path with script fallback

Status: accepted for RichText; measurements in progress.

The first matched Unicode owned-construction/read workload takes 4.49–4.65 us
versus the reference's 1.43 us. Retain these adverse raw observations. Most of
Matea's work is normalizing/joining text and eagerly traversing all graphemes to
cache its per-grapheme width; cloning and repeated width reads have lower costs.

Avoid rescanning the joined, already sanitized buffer with a second normalization
check. A crate-private Text constructor accepts only the shared sanitizer's
output and checks that display invariant in debug builds. No unsafe conversion
or unchecked UTF-8 is introduced. Use iterator length hints for bounded initial
capacity only (64 chunks, 64 KiB text speculation); allocate transition capacity
only on the first actual style change. Unknown hints retain ordinary growth.
Empty/uniform documents do not allocate an unused transition array.

For normalized lines, unicode-width 0.2.2's optimized whole-string width equals
the sum of EGC widths outside its script-specific ligatures. Audit the pinned
crate's lib.rs width rules and tables.rs state machine: Arabic, Hebrew, Khmer,
Buginese, Tifinagh, Lisu, Old Turkic and Kirat Rai. Conservatively use the existing
per-EGC path for their entire blocks. ASCII uses byte length. Other text uses the
dependency's whole-string path. CR is already normalized and LF is structural.
The source also joins keycap/flag/modifier ZWJ combinations that may cross EGCs;
query GraphemeCursor at every boundary after ZWJ and fall back if any is a break
(or cannot be resolved). Ordinary pictographic ZWJ chains retain the fast path.

This is an internal implementation of the existing compositor width policy,
not an opt-in shaping policy or a generic Unicode-width replacement. Dependency
upgrades require reviewing the rule list. Add literal script/emoji regressions,
all Tifinagh consonant pairs with both joiners, and an explicitly executed native
release test of every non-control Unicode scalar in three mixed/ZWJ contexts. These
finite tests do not prove every possible Unicode sequence or future dependency.
Rerun the same complete-output/reference distributions before claiming a gain.
