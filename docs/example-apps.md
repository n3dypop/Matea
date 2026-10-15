# Running and reviewing examples

From the repository root:

```sh
cargo run -p matea --example counter
cargo run -p matea --example list
cargo run -p matea --example form
cargo run -p matea --example dashboard
cargo run -p matea --example effects
cargo run -p matea --example motion
cargo run -p matea --example input
cargo run -p matea --example pagination
cargo run -p matea-example-external-widget
```

All nine accept `-- --snapshot` for plain non-TTY output. `NO_COLOR=1` disables colors; `MATEA_REDUCED_MOTION=1` disables dashboard motion. Full-screen interactive runs require TTY stdin and stdout. Windows PowerShell users can set environment variables through `$env:NO_COLOR='1'`.

Counter: +/- or arrows, q/Esc exit. List: typing filters, Tab changes search/list focus, arrows select, Enter chooses, Esc exits. Form: Tab/Shift-Tab traverse, Enter advances/validates/submits, Esc exits. Dashboard: Tab changes table/notes focus, arrows/PageUp/PageDown navigate, q exits. Badge: Space toggles, q exits. Effects: a typed completion updates the view; r reloads, c cancels, q exits. Ctrl-C exits all live examples through cleanup.

The lifecycle fixture is for validation, not an application showcase: q exits after cooperative worker cancellation, p triggers an update panic, and `--panic-view` exercises first-view restoration. render_probe prints local cell-render timing and output counts. It does not benchmark images or compare against other frameworks.

Input: inspect typed press/repeat/release, modifiers, mouse, paste size and focus.
Add `-- --mouse` to request mouse capture. In a compatible Unix terminal,
`-- --mouse --kitty --deliver-ctrl-c` requests enhanced keyboard reporting and
delivers Ctrl-C to the app; q exits. `--kitty` is an explicit support assertion,
not automatic detection, and is rejected before setup by the current Windows
command backend. The inspector requests alternate keys for layout-provided shifted
text; backend normalization cannot retain physical identity. See [limits](input-audit.md).

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --all-targets
cargo build --workspace --examples
python3 scripts/example_smoke.py
python3 scripts/pty_smoke.py  # Unix only
cargo run -p matea --example render_probe --release
```

The Python smoke suite compares the dashboard against its reviewed plain golden and starts all public examples in snapshot mode. The PTY suite checks input, resize, termios/cursor/screen/paste/focus restoration and cancellation under exit and panic on Unix. PTYs are not terminal emulator visual tests. CI repeats portable tests on Linux, Windows and macOS, with PTYs on Unix. Actual recorded results are in foundation-gate-report.md.

Pagination: left/h or right/l changes the tea page, d switches Arabic/dots, and q
exits. Custom formatting, keys, empty data and bounded active-page dots are covered
by the component tests; the example also has a Unix PTY navigation test.
