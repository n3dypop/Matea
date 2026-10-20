# Native terminal handles

Use a borrowed Rust IO owner, rather than a numeric file descriptor. For a visible
terminal size, stdout is the usual output handle on Unix and Windows:

```no_run
use matea_terminal::device::terminal_size;
fn main() -> std::io::Result<()> {
    let output = std::io::stdout();
    let size = terminal_size(&output)?;
    println!("{} columns × {} rows", size.width, size.height);
    Ok(())
}
```

If stdout is redirected, this query returns an OS error. On Unix, an application
can explicitly open `/dev/tty` when it needs its controlling terminal. On Windows,
open `CONOUT$` for viewport queries; `CONIN$` is the input console and is used for
raw/password input. An application without a console can still get an error.

```no_run
#[cfg(windows)]
fn main() -> std::io::Result<()> {
    let output = std::fs::OpenOptions::new().read(true).write(true).open("CONOUT$")?;
    let size = matea_terminal::device::terminal_size(&output)?;
    println!("{} × {}", size.width, size.height);
    Ok(())
}
#[cfg(not(windows))]
fn main() {}
```

Numeric descriptors are deliberately rejected; the borrowed owner must stay open:

```compile_fail
let _ = matea_terminal::device::terminal_size(&1usize);
```

`TerminalFile` composes `AsFd` on Unix and `AsHandle` on Windows. Regular files,
pipes and invalid console roles can return errors; they do not receive a guessed
size. Read-only queries do not acquire the mutation lease. `TerminalDevice::open`
and `open_raw` own original-state restoration and share the interactive runtime's
exclusive lease. The caller keeps the file/handle's close policy. Do not run a
device mutation guard concurrently with Matea's interactive runtime.

The direct `read_password` primitive blocks. Its byte limit, partial-error bytes,
current-mode restoration and byte-backspace semantics are explicit. Limit errors
consume the triggering byte; allocation failures can consume additional bytes
from the same canonical line. This primitive installs no process signal policy;
abrupt process termination does not run Drop. See [the audit](terminal-device-audit.md)
for exact tested conditions and pending cancellation/platform work.

This typed API and the common-handle examples address the documentation request
in [charmbracelet/x #76](https://github.com/charmbracelet/x/issues/76), assessed
against fixed x/term 4542a189d0fbeb156bb1df8730d253f2ef443c00. That issue concerns
descriptor documentation; no historical functional defect is asserted.
