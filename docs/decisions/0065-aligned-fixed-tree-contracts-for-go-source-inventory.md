# ADR 0065: Align the Go inventory's fixed-tree contract

Status: implemented; native and three-OS qualification pending.

The Go source auditor opened every non-directory path ending in `.go`. A local
fixture reproduced an external file-link declaration and a FIFO read that did
not complete within two seconds. A linked root also returned zero with no output.
Preserve those known-negative observations in `go-auditor-containment/before.json`.

Require a real, non-linked directory using `os.Lstat` before allocating the output
buffer. Skip links and non-regular entries before opening source. The selected
root is scanned even when its own basename is an otherwise ignored descendant
name. Keep the existing descendant exclusions and deterministic `WalkDir` order.

Reject invalid UTF-8 relative names before parsing/JSON encoding: Go's JSON
encoder would otherwise replace invalid bytes and could merge distinct file IDs.
Root arguments remain OS-native strings; only emitted relative IDs require UTF-8.
Use `filepath.ToSlash`, preserving legitimate POSIX filename backslashes. These
are fixed-tree contracts, not protection against adversarial concurrent mutation.

Regression tests cover external file links (Unix), cycle/external directory links
and root links (Windows junctions included), a FIFO (Unix), a selected root named
`examples`, Linux invalid-name/root-byte cases and existing parse/short-write/flush
failures. Preserve platform skips explicitly. Native race/vet checks supplement
three-OS CI; no Windows file-symlink creation or FIFO execution is implied.

Correctness changes to invalid inputs do not create a matched performance ratio.
Compare ordinary fixed pinned trees and independent synthetic output bytes before
and after. Parser-only Go timing/allocations/RSS remain an independent unfinished
measurement scope; this change does not automatically raise that performance score.
