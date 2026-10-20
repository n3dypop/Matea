# ADR 0072: Owned Unix attribute customization after compiler export inspection

Status: selected; native qualification in progress.

The bounded compiler experiment found additional public selectors beyond Windows
Mode: Darwin State exposes Termios plus seven fields; Linux amd64 exposes Termios
plus eight, including Line. The source embeds unix.Termios through private state,
so exported descendants remain selectable. The raw declaration inventory does
not resolve this chain. Its candidate count must not be labeled all public APIs.

Keep portable TerminalState opaque and add explicit device::unix attributes and
snapshot operations. Reuse the existing target libc::termios type as Attributes;
copy its owned fields, edit them outside the device, then apply through the same
guard's OS call. No mutable borrow of the original restoration state escapes.
No new unsafe block, descriptor ownership, codec or engine is introduced.

Use the actual target libc ABI and field names rather than pretending the Go
kernel record has the same layout. For example Linux amd64 source Cc has 19
entries while libc's array can have 32. Speed fields may be normalized by libc
and the OS rather than exposing zero-initialized/ignored kernel-record tail data.
No raw-byte/padding equality or literal Go layout is promised; compare named
behavior and record unverified source ABI/OS conditions. Native normalization and
invalid flags still return OS results at apply time.

Test all control-byte values at each target array index, maximum flag words and
copy independence in pure Rust. Native disposable PTYs change echo and EOF bytes,
query them independently, then restore the original capture before existing raw,
input, Drop/unwind and queued-line checks. The altered snapshot must not mutate
the guard's original. Requalify Linux/macOS/Windows/MSRV and targeted Miri after
the change. Windows uses its separate ConsoleMode API from ADR 0071.

The Go target/export closure stays false. This bounded experiment omits pointer,
returned/private-interface, generic and broader package/profile reachability;
it is evidence of omissions, not completion of a Go export audit.
