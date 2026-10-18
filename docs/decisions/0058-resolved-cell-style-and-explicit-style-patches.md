# ADR 0058: Resolved cell style and explicit style patches

Status: implemented for a narrow cell-style slice; qualification/performance pending. Public API is provisional.

The resolved Style used by every Cell cannot express inheritance versus explicit
terminal default, or inherited versus explicitly disabled attributes. Pinned
Ratatui style.rs models Option colors and additive/subtractive modifier sets;
Lip Gloss separately tracks unset properties. These missing capabilities also
block composition of viewport line/highlight styles.

Preserve the compact resolved Style/Cell and introduce StylePatch as a separate
Copy value. Optional foreground/background/underline colors distinguish inherit
from explicit Color::Default. Private u16 attribute mask/value fields distinguish
inherit/set/clear without allocations. Underline, blink and font-frame choices
always occupy complete groups; safe constructors cannot create invalid numeric
combinations. Construction/inspection must not expose mutable raw mask bits.

resolve(base) produces a fully resolved Style. then(overlay) composes the two
intent values with the last explicit setting winning, including false/default.
reset explicitly resets all colors/attributes. from_style creates a complete
override. Inherit methods remove only the requested intent. Resolving a combined
patch must equal resolving its inputs sequentially for every legal base style.
This keeps per-cell storage unchanged and avoids interpreting default values as
inheritance. Terminal capability/color adaptation still runs on resolved styles.

Use an attributes child module to access the private packed representation;
there is no unchecked public constructor or unsafe memory operation. Audit the
composition algebra, finite boolean/enum domains and terminal encoder results.
Compare the overlapping color/modifier subset directly with pinned ratatui-core
in the isolated std-only reference workspace. Underline-color is conditional in
that source and is not silently enabled in the existing frozen text profile.
Extra Matea decoration groups and conflicting raw source modifier combinations
need explicit independent tests/limitations, not a whole-style parity claim.

The initial API does not add arbitrary padding/border/layout decorators or
mutate shared RichText snapshots. Callers may resolve patches against theme
tokens before constructing rich spans. RichText mutation, style decorators,
Stylize conversions, full platform/feature graphs and complete module performance
remain separate work. Require native/Miri/three-OS tests and complete output /
allocation distributions before assigning a high performance verdict.

Native tests validate 1,679,616 Boolean base/ternary-patch combinations and
579,933 overlapping std-only source observations. Literal SGR tests include
explicit reset and disabled bold. Six raw overlapping source add/sub controls
show consistent removal priority in both has_modifier and Cell::set_style; an
initial contrary assumption was disproved and withdrawn, not recorded as a bug.
Raw overlapping source state is projected by its defined removal priority; Matea
setters retain last-setter intent. Reference underline-color and serialization
profiles remain pending.
