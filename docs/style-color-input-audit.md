# Terminal style color input

Active axis: `matea-style/lib` / reference. Score remains **50**. This is its
first sequential implementation subtask; other 46 below-high axes are unchanged.

The source is Lip Gloss `19f011f1cc573e5d2bf9d7a9c4f14d429647759e`, `color.go`
(MIT), using the already pinned source comparison module/dependencies. All 33
raw declarations in that file are reviewed in the retained declaration map.
Reference helpers work on arbitrary Go color.Color; the currently tested
adjustment helpers operate on terminal RGB8/indexed colors. High-depth/custom
color transformations and full style geometry remain part of this open axis.

| Source contract | Matea implementation and explicit limits |
| --- | --- |
| Sixteen constants, ANSIColor, RGBColor fields | Typed Indexed(0..15/255), Rgb channels; canonical palette conversion |
| NoColor / RGBA | Default is unknown; rgb_channels/rgba16 return None, preserving absence rather than inventing black |
| Color(string) | FromStr; #RGB/#RRGGBB, signed decimal magnitude, index or low 24 packed bits; explicit parse/overflow errors |
| LightDark/LightDarkFunc | AdaptiveColor::resolve with caller-provided background state |
| Complete/CompleteFunc | ProfileColors::resolve, independent author choices for four ColorLevel values |
| Alpha | Rgba16::with_alpha, explicit conversion to validated PremultipliedRgba16; source contract correction below |
| Complementary | Color::complementary, exact integer HSV 180-degree rotation for RGB8 |
| Darken / Lighten | Color methods, finite clamp and source channel arithmetic for RGB8; unknown default/nonfinite errors |

The [frozen producer](evidence/style-reference-color/fixtures.go.txt) calls the
actual public source APIs and standard Go NRGBA64 conversion. Its unchanged raw
29,898 TSV observations are the native test fixture; no rewritten expected
outputs are substituted. They comprise 5,146 parse inputs, 1,285 complementary
colors, 11,565 darken and 11,565 lighten observations, 81 precise-depth standard
premultiplications and 256 source-alpha observations. Invalid parse observations
are tested as explicit Rust errors plus the caller's source-compatible fallback.
Another 282 observations cover all 256 RGBColor depth values, NoColor.RGBA,
both background selections, seven profile selectors and sixteen named constants.
The first profile-adapter test incorrectly assumed enum discriminants without
the source's Unknown variant. Its failure is retained; the corrected fixture
adapter uses the pinned values 3/4/5 for ANSI/ANSI256/TrueColor. Production uses
the typed Rust enum, without dependency on Go discriminants.

In the alpha observations the source keeps R*257 in color.RGBA and sets alpha to
32639 at 0.5, so 128 of 256 red values exceed alpha. Rust retains straight RGB in
Rgba16 and only constructs valid premultiplied values; these are deliberate
contract corrections, not byte-identical source output. RGBColor's source
channel<<8 also yields 65280 for white; Rust expands RGB8 to 65535 canonically.
No upstream issue report is marked resolved merely from these observations.

The six new native tests also check all 65,536 scalar values against nine alpha
boundaries (589,824 combinations), all 65,536 opacity values against an
independent opaque-background rational model, and 49 mixed-alpha combinations
against an independent floating model. Negative syntax, signed boundaries,
unknown colors, NaN/infinity, premultiplication errors and all profile branches
are covered. The existing exhaustive RGB quantization and compact Style/Color
size checks remain required. Miri uses seed239, five boundary tests; it skips
the 29,898 source rows and samples nine scalar/opacity boundaries rather than
claiming exhaustive Miri coverage. Native/Miri/3 OS source bindings are recorded
alongside this audit as qualification completes.

No timing measurements or source speed ratios are added by this increment.
Pure integer values avoid heap ownership and unsafe code, but that is a design
property, not a measured module performance verdict. The open style axis still
requires common decorators/dimensions, rich text/whitespace/hyperlink contracts,
inheritance, all features/platforms and precise/custom-color closure before 75.
