# Explicit cell-style patches

Implementation/qualification in progress. No high performance or whole-style
module verdict is assigned. This is a separate intent value; the resolved Style
stored in every Cell remains 14 bytes, with a 16-byte Copy/Eq/Hash StylePatch.
Foreground/background/underline-color options distinguish inheritance from an
explicit terminal default. Attribute mask/value pairs preserve explicit false.
Underline/blink/font-frame groups have typed setters and no invalid raw numeric
constructor. `then` composes intent; `resolve` applies it to a theme/style.

Six ordinary integration tests cover inheritance/default/false, full reset,
complete overrides, Boolean/enum composition, associativity, const use and
concurrent copies. A separately executed finite test checks all 256 Boolean base
states against all 6,561 ternary patches (1,679,616 pairs). A literal terminal
encoder regression verifies explicit reset and disabled bold without introducing
manual escape emission into application APIs. See ADR 0058.

The independent reference workspace still uses pinned ratatui-core
d1ef782b48ddf0cb825c111d4d8b9adef572c0e1, its frozen 43 package versions and the
std-only profile. Native public Style/Cell observations compare 46,656 Boolean
resolutions, 531,441 intent compositions, 1,764 foreground/background resolutions
and 72 valid underline/blink choices: 579,933 observations. The conditional
underline-color profile is not silently enabled. Extra Matea decoration groups,
raw conflicting flags and serialization are not claimed equal by this corpus.

Six overlapping raw source add/sub controls confirm **removal** priority in both
`has_modifier` and `Cell::set_style`. An initial contrary assumption failed its
native assertion and was corrected; it is not an upstream defect. Retain the
correction record and complete control observations under `evidence/style-patches`.
Matea exposes deterministic last-setter intent and cannot construct overlapping
private mask/value states through safe APIs.

The measurement caller has six matched operations (build, resolve, compose,
inherit, complete override and runtime-selected reset), with complete canonical
16-byte foreground/background/six-Boolean records consumed via FNV. Two separate
25-byte expanded-decoration controls have no reference ratio. Source resolution
includes a Cell clone; representations and extra properties differ. Measurement,
Miri, full native/LLVM and three-OS qualification are pending.

Style decorators, arbitrary padding/borders/layout, all Stylize conversions,
conditional underline-color/serde/anstyle/palette profiles, complete raw modifier
composition and full module performance remain open. Stage A and F5 stay closed
to advancement; the image engine and CUDA remain gated.
