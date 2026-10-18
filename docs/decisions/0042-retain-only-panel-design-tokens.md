# ADR 0042: Retain only the design tokens consumed by a panel

Status: proposed for measured implementation.

Attribute expansion preserves the 40-byte Cell stride but grows the complete
Theme. The 4d1a397 controls show short panel construction still makes two
allocations, while requesting 160 bytes instead of the previous 140. Node::Panel
retains a boxed clone of all seven Theme styles, gap and motion fields. Actual
painting/natural height use only surface, border, accent, padding and border kind.

Keep View::panel(title, &theme) unchanged and snapshot only those five tokens into
a private boxed PanelTheme. Copy the three resolved styles at construction. The
view remains owned after the passed theme is mutated or dropped; no theme/global
cache, Arc, unsafe storage or lifetime parameter is introduced. Keep the snapshot
boxed so enlarging the Panel variant does not grow every View in every stack.

An inline full/partial theme would enlarge Node's largest variant; retaining an
Arc<Theme> introduces shared allocation/ownership into a value-based public API.
Neither is justified for three cheap Copy values. Removing the snapshot entirely
would lose the existing owned-scene semantics. Do not change those semantics to
reduce a benchmark's work.

Validate all three border kinds, title/border/surface styles including the new
attributes, padding/natural height, cloned scenes and mutation/drop of the source
theme. Reuse the committed 4d1a397 20-case scene baseline without repeating its
completed run, then retain all distributions/allocation counters and balanced
repeats. The literal full-cell/row tests, existing goldens, third-party widget and
interactive resize/cleanup tests protect observable behavior. Run targeted Miri
and native/three-OS checks before calling this optimization validated. No new
public API, reference capability completion or Stage B claim follows.

The first measured token-only version (db865c6) reduces short panel requested
bytes from 160 to 102 with two allocations, without a consistent speed gain.
Retain it as an intermediate. Evaluate storing the child View and three-token
snapshot in one boxed PanelContent, while retaining the inline title in Node.
This avoids a second allocation and keeps the enum/stack stride bounded; actual
padding can request a few more bytes than the two separate payloads. Prefer the
single owner only if measured allocations, distributions and all behavior remain
acceptable. Preserve the two-allocation intermediate and mixed results.
