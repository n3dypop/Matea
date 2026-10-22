# ADR 0015: Typed shortcuts and borrowed custom tokens

Status: implementation decision; provisional generic API, unpublished crates.

Bubbles key's 17 public declarations provide constructor options, mutable keys,
help, enable/disable, unbind and fmt.Stringer matching. Matea's public fields and
constructor supply the same configuration capabilities without a closure per
option. Extend Binding to Binding<K = Key>: existing keyboard callers keep typed
code/modifier matching and down-event policy; custom strings or enums use borrowed
Eq through matches_value and its multi-binding helper. String-owned keys
can be queried by &str through Borrow, with no query allocation or implicit
formatting. A caller needing formatter-defined identity can explicitly format
before matching. Associated keyboard text extraction remains a transport concern.

The generic API has one shared enable/help/unbind contract. An empty key set is
disabled even when its allocation is retained. The pinned Go source treats a
non-nil empty slice as enabled and can advertise help for an unusable shortcut.
Matea tests that clearing keys also hides actionable help, while preserving raw
metadata for editing. Unbind clears both keys and metadata, preserving capacity.

Measure constructors as well as hot matching. The retained baseline allocates
three times for Vec and two short String metadata values. Store metadata in the
already pinned CompactString primitive so short labels/descriptions stay inline;
long UTF-8 strings remain intact on the heap. No new package/version is selected.
The public metadata representation changes before API review/publishing; Into
construction and borrowed help remain the normal application interface. Small
matching methods carry inline hints; no unsafe or query-specific specialization.
Eq's reflexivity permits a pointer-identity shortcut before equality. Measure
interned and distinct-owned equal text separately so a literal shortcut cannot
hide the content comparison cost. Floating point identities can be normalized to
explicit tokens; partial non-reflexive equality is not a binding identity contract.

Bubbles #262 reports repeated key formatting. Our keyboard fixture measures that
cost separately from custom-token matching whose source String returns an existing
string without allocation. Do not generalize either result to every formatter or
application, or omit regressions. The full key module verdict uses fixed-SHA
source review, mutation regressions and finite matched workloads.

Sources: Bubbles c3bde0d166c2ca607d9864493453fd77bdcae7e4 key/key.go:43–140,
key/key_test.go and https://github.com/charmbracelet/bubbles/issues/262.
