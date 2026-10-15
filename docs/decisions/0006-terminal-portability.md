# ADR 0006 — Terminal ownership and portable degradation

Status: Implementation decision; three-OS CI verified, owner review pending (2026-10-16).

## Decision
Use released Crossterm 0.29 for platform input and command encoding. The audited Git checkout is newer and is not the dependency version. One exclusive session owns raw mode, alternate screen, cursor, focus/paste reporting and output. Reject nesting and non-TTY interactive runs. Restore on normal return, initialization failure, I/O failure and unwinding panic. Catch application unwind at the transport boundary, restore, then resume it. Unix SIGINT/SIGTERM use a small sigaction guard that saves/restores exact prior handlers and sets only an atomic flag in the handler; raw Ctrl-C is handled as input. SIGKILL, process abort, power loss and terminal I/O failures cannot have guaranteed restoration.

Environment detection is conservative and overrideable: NO_COLOR/TERM=dumb disable colors, COLORTERM selects truecolor, 256color selects indexed color, otherwise ANSI16. These are hints, not active protocol negotiation. No graphics capabilities are advertised. Explicit plain snapshots work without a terminal, with ASCII borders and motion disabled. Full-screen is the interactive Foundation mode; true inline interactive layout and a screen-reader form prompt mode are unresolved work, not supported claims.

CI runs fmt/clippy/test/doc and example smoke on Linux, Windows and macOS. Unix PTY checks exercise resize, input, cancellation and restoration; real Windows console behavior requires Windows validation. A workflow file alone is not evidence of green CI. Three-OS and Linux MSRV runs succeeded on 2026-10-16; evidence is in the Foundation gate report. Explicit owner approval remains required.

## Signal implementation follow-up

Source review of signal-hook-registry 1.4.8 `src/lib.rs:694–702` found that unregister does not restore the prior/default handler and may leave SIGTERM ignored after run returns. Therefore the transport uses a narrowly scoped unsafe Unix FFI module around libc sigaction/sigemptyset, with documented safety arguments and partial-install cleanup. All other crates forbid unsafe; the terminal crate denies it except this module. Terminal raw mode/input parsing remain delegated to Crossterm. EXCLUSIVE prevents overlapping Matea sessions; applications must not concurrently replace SIGINT/SIGTERM handlers during a session. The PTY suite also verifies post-run SIGTERM default behavior. This is an intentional and reviewable exception, not a claim of an entirely safe-Rust transport.
