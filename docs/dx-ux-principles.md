# Developer and user experience rubric

1. A counter must fit in a small Model and one run call. No setup/cleanup, ANSI literals, dynamic casts or mandatory async runtime in app code.
2. Every interactive component uses Component/Input/Response/View/Theme. Parent routing consumes Tab once and forwards input only to the selected target. Component actions map into domain messages at the parent.
3. All app state is inspectable without a real terminal. Runtime::dispatch, advance and frame make examples reproducible. Effects alone use threads; cancellation and failure behavior are documented.
4. Use the same explicit Theme for all components. Focus uses a marker and reverse/bold, validation uses an Error label and underline. Color alone never communicates selection or failure.
5. Keyboard help remains visible. Tab/Shift-Tab switch focus; arrows navigate; Enter activates/submits; Esc exits the sample form/list; Ctrl-C always leaves the live driver.
6. Reduce motion with MATEA_REDUCED_MOTION=1 and stop unnecessary timer subscriptions in the app. NO_COLOR preserves non-color focus cues. Snapshot examples render plain text without a TTY.
7. At narrow sizes, clip safely and do not split graphemes or write outside the viewport. Responsive reflow/wrapping and a complete screen-reader prompt mode are open work. Never equate plain snapshots with full accessibility certification.
8. Review public API and publication licensing before naming/semver lock-in. Root package names are provisional and unpublished.

Evidence: runnable examples, widget/core/render/style tests, dashboard golden and Unix PTY suite. No user study, real-emulator screenshot suite or comparative DX superiority claim has been made.
