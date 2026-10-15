# Charm ecosystem audit — F0

Observed source behavior is separated from proposed Matea scope. Full origins/revisions: [upstream-lock.json](upstream-lock.json). No Go source, fonts or screenshots were copied into the implementation. Local source audits and examples, rather than README claims alone, informed the design.

| Library and observed source | Desired Rust authoring model | User behavior / boundary | Risk and acceptance evidence |
| --- | --- | --- | --- |
| Lip Gloss: value Style, Inherit and Render; Render applies dimensions, alignment, padding, margins and borders. [style.go:137](https://github.com/charmbracelet/lipgloss/blob/19f011f1cc573e5d2bf9d7a9c4f14d429647759e/style.go#L137), [style.go:238](https://github.com/charmbracelet/lipgloss/blob/19f011f1cc573e5d2bf9d7a9c4f14d429647759e/style.go#L238), [style.go:268](https://github.com/charmbracelet/lipgloss/blob/19f011f1cc573e5d2bf9d7a9c4f14d429647759e/style.go#L268); horizontal/vertical styled-string joins in join.go | `View::styled(..., theme.accent).panel(..., theme)` | Portable Style values; renderer owns geometry, transport owns ANSI | String ANSI composition is intentionally avoided; golden and fallback tests |
| Bubbles: focused textinput ignores events while blurred; supports keymaps, validation, cursor editing. [textinput/textinput.go:586](https://github.com/charmbracelet/bubbles/blob/c3bde0d166c2ca607d9864493453fd77bdcae7e4/textinput/textinput.go#L586) | `input.event(&event, focused)` | One focused target; grapheme editing, safe paste, explicit cursor highlight | Unicode/IME width mismatch remains possible; editing/focus tests |
| Bubbles list has Item/ItemDelegate, filtering and selection. [list/list.go:34](https://github.com/charmbracelet/bubbles/blob/c3bde0d166c2ca607d9864493453fd77bdcae7e4/list/list.go#L34) | `List::new(...); list.filter(query)` | List state owns selection/filter, parent maps selected index | Initial Matea filtering is case-insensitive substring, not fuzzy; empty/filter tests |
| Bubbles table and viewport own sizing/selection/scroll state in table/table.go and viewport/viewport.go | `Table`, `Viewport` implement Component | Arrow/Page navigation and shared Theme | No rich markdown yet; table/viewport tests and dashboard |
| Bubbles spinner uses ID and tag to reject unrelated/stale ticks. [spinner/spinner.go:124](https://github.com/charmbracelet/bubbles/blob/c3bde0d166c2ca607d9864493453fd77bdcae7e4/spinner/spinner.go#L124) | Parent subscription + `spinner.tick(&theme)` | Timer lifecycle belongs to Runtime; no component threads | Stable subscription IDs and cancellation generation; reduced-motion tests |
| Huh Field includes focus, blur, validation, accessibility, standalone Run and theming. [form.go:143](https://github.com/charmbracelet/huh/blob/ffb6a97195b4a031925a31cc70c62fc1d309e69d/form.go#L143); RunWithContext chooses accessible or interactive execution. [form.go:706](https://github.com/charmbracelet/huh/blob/ffb6a97195b4a031925a31cc70c62fc1d309e69d/form.go#L706) | `Form::new(vec![Field::new(label, validator)])` | Form owns inputs/validation/traversal; errors have textual labels | Matea omits per-field terminal lifecycle. Huh-style linear accessible prompts are deferred, not claimed |
| Glamour constructs Goldmark with extensions and an ANSI renderer, supports styles/word wrap. [glamour.go:68](https://github.com/charmbracelet/glamour/blob/010543cac1a07c9bfd9969e58d683f2ddef4c8f7/glamour.go#L68) | Future independent document renderer → structured views | Foundation viewport can host resulting lines | Full markdown parser, syntax highlighting and link policy remain later optional work |
| Harmonica caches spring coefficients and updates position/velocity from explicit deltas. [spring.go:104](https://github.com/charmbracelet/harmonica/blob/0ff76e405449b47bd60f1df4820d880081692d3c/spring.go#L104), [spring.go:216](https://github.com/charmbracelet/harmonica/blob/0ff76e405449b47bd60f1df4820d880081692d3c/spring.go#L216) | Future pure animation library + runtime clock | Motion values independent of terminal I/O | Foundation spinner only; no copied spring algorithm or spring parity claim |

## Dependency and license facts

All six root LICENSE files say MIT. **Harmonica spring.go additionally embeds the Ryan Juckett source notice and alteration/origin restrictions**; any future reuse must preserve that notice. This inspection therefore does not describe the entire Harmonica implementation as only a root MIT notice.

Manifests inspected: Bubble Tea v2 uses Ultraviolet/colorprofile/x/ansi/x/term/cancelreader; Lip Gloss uses Unicode width/grapheme and color libraries; Bubbles imports Bubble Tea/Lip Gloss/Harmonica, clipboard and fuzzy matching; Huh adds themes, hashing and PTY support; Glamour uses Goldmark/Chroma/Bluemonday; Harmonica's go.mod has no required external modules. The local versions of these modules differ (for example Bubbles' Bubble Tea v2.0.10 vs the audited v2.1.0), so cross-version Go compatibility is not inferred. We did not run Go tests or reproduce every dependency's source audit.

## DX/UX rubric and representative apps

| App | Audit insight | Matea executable | Scope |
| --- | --- | --- | --- |
| Counter | Small state/update/view onboarding | `cargo run -p matea --example counter` | No escape strings or manual cleanup |
| List/search | Parent composition of focused input/list | `--example list` | Tab, arrows, Enter; substring filter |
| Form | Declarative fields, validation and traversal | `--example form` | Required validation, textual errors, BackTab |
| Themed dashboard | Shared tokens across components | `--example dashboard` | Table, viewport, timer-driven spinner |
| Independent component | Public composition contract | `cargo run -p matea-example-external-widget` | Badge in a separate workspace package |
| Markdown viewer | Independent Glamour-style boundary | Foundation viewport only | Parser/render extension deferred |
| Spring animation | Pure Harmonica-style math boundary | Foundation spinner only | Spring library deferred |
| Image viewer | Separate conversion/encoding/transport | Design document only | Explicitly prohibited before owner gate |

Review the concrete running apps and [design system](design-system.md), not a numerical assertion of DX superiority. Foundation intentionally narrows features to tested contracts. Public APIs remain provisional until owner review.
