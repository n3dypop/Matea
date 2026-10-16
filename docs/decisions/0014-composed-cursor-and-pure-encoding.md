# ADR 0014: Composed cursor position and pure frame encoding

Status: implementation decision; provisional position-only cursor contract.

Bubble Tea's View.Cursor uses frame coordinates and its text input asks callers
to add widget offsets manually. Bubbles #906/#1001 identify logical-rune/scroll
coordinates leaking into real cursor placement. Ratatui's pinned Frame
set_cursor_position also uses frame coordinates and applies the cursor after the
buffer diff. Reuse that ordering, while keeping Matea's local composition contract.

Add a Copy Cursor position to render and optional cursor metadata to Surface. A
View can annotate its child with a local cursor. Row/column/panel/layer layout
translates it during painting, checks inherited clips and normalizes continuation
cells to a grapheme's leading cell. Later opaque paint over the cursor suppresses
it; a later focused cursor annotation can replace it. TextInput's native mode uses
its already cell-based scrolled viewport, including prompt width, so app code has
no offset arithmetic. Hidden input keeps the cursor fixed.

Cursor metadata changes independently of cell damage. Transport must apply it
even when no row changed. Hide a visible cursor while painting cells, position it
after the batch, and show/hide according to the frame. Repaint after external
output treats visibility as unknown and reestablishes the requested state. The
session guard retains visibility restoration ownership across errors and panic.

Unknown or wrap-pending physical cursor positions reanchor absolutely. Known
same-axis moves may use relative ANSI when shorter than CUP, without temporary
String allocation. Retained measurements and independent cell/position/visibility
replay reduce the near-move fixture from 6 to 4 bytes; this does not justify
relative moves across unknown-width graphemes or unverified terminal modes.

Keep this increment to position/visibility. Shape, color, virtual blinking and
their capability/restoration contracts remain explicit pending work; do not expose
nonfunctional appearance knobs. This extends the narrow compositor with metadata,
not a paragraph/layout or large-widget replacement. The Ratatui adapter decision
is still required before that broader scope (ADR 0004).

The released Crossterm 0.29 Command::queue can execute WinAPI directly when ANSI
is unsupported. Frame encoding into a Vec must never choose OS operations. Call
Command::write_ansi through a small fmt-to-io adapter; preserve original I/O errors
and invalidate encoder state on failure. Native mode acquisition/restoration may
still use Crossterm's actual transport commands. Pure encoding does not establish
support in a legacy Windows console; native-console validation remains separate.

Sources: pinned Bubble Tea tea.go:379, Bubbles textinput.go:916, Ratatui
ratatui-core/src/terminal/frame.rs:151–168, released Crossterm src/command.rs:120–134
and pinned cursor.rs. Appearance protocol research does not authorize image-engine
or CUDA implementation.
