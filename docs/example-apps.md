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
cargo run -p matea --example help
cargo run -p matea --example clocks
cargo run -p matea --example spinners
cargo run -p matea --example cursor
cargo run -p matea --example attributes
cargo run -p matea-example-external-widget
```

All fourteen showcase programs accept `-- --snapshot` for plain non-TTY output. `NO_COLOR=1` disables colors; `MATEA_REDUCED_MOTION=1` disables dashboard motion. Full-screen interactive runs require TTY stdin and stdout. Windows PowerShell users can set environment variables through `$env:NO_COLOR='1'`.

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

Help: `?` switches short/full display, `d` changes the action binding's enabled
state and its displayed help together, and `q` exits. Borrow metadata into HelpItem
per view call and configure Help.width for whole-item fitting. Gap colors/attributes
can be styled independently. Rich per-field decorators remain pending; see the
[help audit](key-help-audit.md).

Clocks: `s` pauses/resumes the stopwatch, `c` pauses/resumes the countdown,
`r` resets/rearms and `q` exits. Expired countdowns need rearming. Both use existing
UI-thread subscriptions, with no worker per clock. The example counts filtered
timeout outcomes, including those returned by controls. See the [clock audit](clocks-audit.md).

The form offers tea-name completion: type a prefix in Favorite tea, then Tab to
accept a suggestion before moving focus. Up/down selects a match. TextInput also
provides placeholder/prompt, typed edit keys, word deletion, live validators and
normal/password/hidden echo. Run `cargo run -p matea --example form -- --native`
to use a native cursor whose position composes through the panel. Appearance,
blink and OS clipboard remain pending; see the input editing audit for limits.

`cargo run -p matea --example spinners` displays twelve independent presets.
Use p to pause/resume, r to reset phases, n for manual stepping, m to toggle
reduced motion and q to quit. The stable `--snapshot` form acquires no terminal.
The example adds a twelfth snapshot and a sixteenth Unix PTY control/cleanup case.

The counter and independent widget use format_args! through owned Text, without a
separate String allocation for short formatted content. A lifecycle formatter
error joins the direct view/update panic checks in Unix PTY cleanup tests.

## Virtual cursor composition

`cargo run -p matea --example cursor` composes independently owned VirtualCursor
state with a clonable TextInput. Editing restarts the blink phase; terminal focus
events focus/blur it. Ctrl+B switches blinking/static mode, Ctrl+R toggles reduced
motion and Esc exits. Subscription reconciliation removes inactive timers; native
cursor mode remains a separate TextInput setting. Use `-- --snapshot` for plain
non-TTY output. This example is covered by snapshots and Unix PTY restoration
checks, without claiming physical emulator or IME validation.

## Resolved text attributes

`cargo run -p matea --example attributes` uses typed decoration builders on
passed theme tokens. +/- or arrows change the counter while focused; Tab toggles
focus, b toggles a hardware blink request and q/Esc exits. The explicit > marker
keeps focus visible without color. The panel clips on resize and the portable
plain snapshot acquires no terminal. Reduced-motion output suppresses blink.
Underline shapes, Fraktur and font framing depend on the terminal; SGR byte
correctness is separate from actual font appearance.

The provisional Style boolean fields migrate to `style.has(TextAttribute::Bold)`
and `style.attribute(TextAttribute::Bold, false)`. Existing bold(), underline()
and reverse() builders remain available. Attributes are resolved owned values,
with no ambient inheritance or backend types in component APIs.
