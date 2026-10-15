# Bubble Tea source audit — F0

Audited on 2026-10-16 from pinned local Git objects (main reference checkouts clean; x module versions read with git show). Full revisions and origins are in [upstream-lock.json](upstream-lock.json). Observations below come from source inspection; upstream test suites were not executed. No feature-parity claim is made.

## Revisions and event flow

Bubble Tea v2.1.0: `f9df43c4f2c021d8740a2628d59cd810a862eb98`. For the v1 comparison, local tag v1.3.10 resolves to `143649d9386dcb6f0d8ad5694b6670ba163f4830`; its `tea.go` Model returns string View. The v2 upgrade guide documents the transition to declarative View fields (AltScreen, mouse, focus, paste), key press/release events, and vanity import paths. V2 is the reference because terminal state belongs to the framework and a richer view is a better extension boundary.

| Observed behavior | Pinned source evidence | Matea decision / test |
| --- | --- | --- |
| Model has Init, Update and View; Msg aliases Ultraviolet Event | [tea.go:50](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L50) | Associated typed Message; Event separates framework/user events; core transition tests |
| View includes styled string Content, cursor, terminal state and mouse handler | [tea.go:84](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L84), [UPGRADE_GUIDE_V2.md:39](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/UPGRADE_GUIDE_V2.md#L39) | Owned structured View; no ANSI in app content; clipping/control tests |
| Central loop translates input, handles framework messages, updates, schedules Cmd, renders | [tea.go:767](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L767), [input.go:8](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/input.go#L8) | Same unidirectional flow, synchronous model ownership; deterministic Runtime driver |
| Cmd handler starts goroutines; comments explicitly acknowledge uncancellable Cmd lifetime | [tea.go:724](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L724) | Eight active worker slots, cooperative token, bounded completion queue; cancellation/overload tests |
| Batch has no completion order; Sequence executes commands in order | [commands.go:15](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/commands.go#L15), [tea.go:919](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L919) | Explicit batch/sequence; child errors stop sequence; sequence test |
| Run installs handlers, initializes terminal/size/input/renderer, invokes Init and initial render | [tea.go:1018](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L1018) | Terminal Session guard encloses model initialization, first view and the loop; PTY unwind tests |
| Input reader uses cancellable reading and Ultraviolet events | [tty.go:59](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tty.go#L59) | Crossterm poll/read, one polling owner, no custom key parser; key-release test |
| Input state restoration uses saved TTY input/output states | [tty.go:34](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tty.go#L34) | RAII cleanup on success/error/unwind; exact termios PTY checks |
| Default renderer is cursedRenderer backed by Ultraviolet | [cursed_renderer.go:21](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/cursed_renderer.go#L21), [tea.go:1090](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/tea.go#L1090) | Do not assume historical standard_renderer.go exists in this checkout |
| Flush lays out styled Content into a cell buffer, adjusts inline/fullscreen dimensions, invalidates resized buffers | [cursed_renderer.go:296](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/cursed_renderer.go#L296) | Fixed fullscreen viewport initially; inline interactive mode deferred |
| Simple example owns countdown state and returns a tick Cmd from Init/Update | [examples/simple/main.go:45](https://github.com/charmbracelet/bubbletea/blob/f9df43c4f2c021d8740a2628d59cd810a862eb98/examples/simple/main.go#L45) | Runnable counter plus lifecycle effect fixture, application has no terminal setup |

Trace: terminal bytes → Ultraviolet TerminalReader → translated Msg → eventLoop → Model.Update → Cmd handler → p.Send → eventLoop → Model.View → cursedRenderer.flush → Ultraviolet buffer/renderer → terminal writer. Not all of this belongs in a Rust runtime crate.

## The actual terminal dependencies

Bubble Tea go.mod pins Ultraviolet `6b8d4baf91b4`, x/ansi v0.11.9 and x/term v0.2.2. Ultraviolet was fetched and checked out at its exact dependency SHA, not at an arbitrary latest HEAD.

| Observation | Source |
| --- | --- |
| Ultraviolet cells carry content, display width, style and link; zero-width placeholders represent continuation cells | [cell.go:15](https://github.com/charmbracelet/ultraviolet/blob/6b8d4baf91b4c933e6613b0eefc8f180f6eda002/cell.go#L15) |
| ScreenBuffer embeds RenderBuffer and selects a width method | [buffer.go:603](https://github.com/charmbracelet/ultraviolet/blob/6b8d4baf91b4c933e6613b0eefc8f180f6eda002/buffer.go#L603) |
| Render uses touched lines, maintains previous buffer state and treats terminal resize as damage | [terminal_renderer.go:1427](https://github.com/charmbracelet/ultraviolet/blob/6b8d4baf91b4c933e6613b0eefc8f180f6eda002/terminal_renderer.go#L1427) |
| Reader StreamEvents responds to context cancellation and coordinates its read worker | [terminal_reader.go:139](https://github.com/charmbracelet/ultraviolet/blob/6b8d4baf91b4c933e6613b0eefc8f180f6eda002/terminal_reader.go#L139) |
| x/ansi has separate grapheme and wcwidth string measurement and a stateful escape stripper | [ansi/width.go:60](https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/width.go#L60) |
| x/term exposes saved state, MakeRaw, Restore and size; Unix adapter uses termios | [term/term.go:1](https://github.com/charmbracelet/x/blob/4542a189d0fbeb156bb1df8730d253f2ef443c00/term/term.go#L1), [term/term_unix.go:17](https://github.com/charmbracelet/x/blob/4542a189d0fbeb156bb1df8730d253f2ef443c00/term/term_unix.go#L17) |

These are facts about the audited versions. Matea does not import their Go code or implement terminal graphics protocols. Colorprofile and other transitive Go packages were identified in go.mod but not separately source-audited; do not treat their internals as verified.

## Rust alternatives and deviations

Ratatui `d1ef782b48ddf0cb825c111d4d8b9adef572c0e1`: [ratatui-core/src/buffer/buffer.rs:336](https://github.com/ratatui/ratatui/blob/d1ef782b48ddf0cb825c111d4d8b9adef572c0e1/ratatui-core/src/buffer/buffer.rs#L336) writes graphemes; [ratatui-core/src/buffer/buffer.rs:471](https://github.com/ratatui/ratatui/blob/d1ef782b48ddf0cb825c111d4d8b9adef572c0e1/ratatui-core/src/buffer/buffer.rs#L471) provides diffing; [ratatui-core/src/buffer/diff.rs:1](https://github.com/ratatui/ratatui/blob/d1ef782b48ddf0cb825c111d4d8b9adef572c0e1/ratatui-core/src/buffer/diff.rs#L1) contains uncertain-width and trailing-cell handling. It is a strong reuse candidate, not merely a competing application framework. Matea's small custom buffer/diff is an ADR 0004 experiment: changed rows are cleared first, each grapheme has an explicit cursor position. Revisit this choice before expanding renderer scope. No claim of faster or more complete rendering.

Crossterm source audit `9a397df6dc4f5bd3ff900bb6308b728186cb914c` covers poll/read and Unix/Windows raw-mode adapters. Actual dependency is released **0.29.0**, pinned in Cargo.lock, not that newer Git revision. Its locally cached release source was also inspected when diagnosing resize initialization. Delegate platform I/O to it; keep layout, focus, timers and app state out of the adapter.

## Licenses and limits

Bubble Tea, Ultraviolet, x, Ratatui and Crossterm root LICENSE files were inspected and are MIT. Audit is architectural inspiration, not copied implementation. Root licenses do not certify every transitive file or asset. Rust dependency declarations/checksums are captured separately. Terminal-emulator behavior, SSH/tmux, Go tests and public naming availability have not been validated by this audit.
