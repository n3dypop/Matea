# Key binding capability and performance audit

Reference: Bubbles `c3bde0d166c2ca607d9864493453fd77bdcae7e4`,
[key/key.go](https://github.com/charmbracelet/bubbles/blob/c3bde0d166c2ca607d9864493453fd77bdcae7e4/key/key.go),
key/key_test.go. All 17 declaration candidates are production, unconditional Go
APIs. No source issue markers occur in this directory. Its only dependency is fmt.
Design: [ADR 0015](decisions/0015-generic-shortcut-bindings.md).

| Source capability | Matea implementation |
| --- | --- |
| Binding, BindingOpt, NewBinding | `Binding<K = Key>::new`, struct configuration; options become ordinary Rust values |
| WithKeys, SetKeys, Keys | generic owned `keys: Vec<K>`; read, replace, clear or extend with capacity reuse |
| WithHelp, SetHelp, Help, Help.Key, Help.Desc | public label/description, borrowed `help()` for actionable display; raw metadata remains readable |
| WithDisabled, SetEnabled, Enabled | public enabled flag plus `is_enabled`, which also checks for actual shortcuts |
| Unbind | removes keys and metadata together, retaining storage capacity |
| Matches | typed keyboard `matches`/multi-binding helper; custom tokens use borrowed `matches_value`/helper |

The Rust default compares key code/modifiers and rejects Release while allowing
Repeat. Custom identities can be strings, enums or any Borrow/Eq pair;
owned String keys accept an &str query without allocation. Source fmt.Stringer
can perform arbitrary formatting; a Rust caller can explicitly format into a
string token when that behavior is wanted. Matching does not interpret or normalize
tokens. Associated keyboard text still depends on the terminal event API and is
not inferred from these generic binding capabilities.

```rust
use matea::prelude::*;
let cancel = Binding::new([Key::new(KeyCode::Esc)], "esc", "cancel");
assert!(cancel.matches(Key::new(KeyCode::Esc)));
let custom = Binding::new([String::from("save draft")], "save", "save draft");
assert!(custom.matches_value("save draft"));
```

The fixed source's Enabled checks slice non-nil rather than non-empty. A non-nil
empty key slice advertises enabled/help although nothing can match. Executed
[observations](evidence/key-source-edges.json) retain this distinction. Matea treats
empty vectors as disabled, preventing help for an unusable shortcut. Unit tests
cover retained empty storage, replace/edit, disabled and unbound states, help,
borrowed strings, enums, case sensitivity, multi-binding and key-kind policy.

[Bubbles #262](https://github.com/charmbracelet/bubbles/issues/262) reports repeated
key string rendering/allocation. Source Matches calls String once per invocation
before checking whether bindings are disabled. The fixed keyboard formatter's
allocation cost is measured in six key-event fixtures. Three separate custom-token
fixtures plus a distinct-owned equal-string query use existing strings in both
languages and may allocate nothing even in Go; the issue does not imply every
Stringer allocates. Constructors and metadata getters add two fixtures. Inline
short help storage reduces constructor allocations from 3 to 1 (49 to 32 bytes),
with measured p50 76.47 to 29.43ns. Borrowed token matching tests both shared and
distinct storage; Eq permits the shared-identity shortcut without unsafe code.
all samples, source hashes, retained pre-change data, p50/p95 and deviations are in
[performance.md](performance.md). Heap bytes and timings depend on representation,
compiler escape analysis and the chosen fixture. This is a finite module verdict,
not a claim of universal speed superiority or full keyboard-parser coverage.

[Bubbles #151](https://github.com/charmbracelet/bubbles/issues/151) requests timed
vim/leader key sequences. That is an extension request, absent from this pinned
module's public single-event capability set. Matea applications can track message
state, but no built-in longest-prefix/timeout sequence resolver is claimed here.
List-specific #1045 remains pending. The [help audit](key-help-audit.md) now assesses
#571 and implements separate key-description gap styles; rich decorators remain
pending with the broader style system.
