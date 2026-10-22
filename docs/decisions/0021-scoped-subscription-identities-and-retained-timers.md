# ADR 0021: Scoped subscription identities and retained timer tables

Status: implementation decision; public API provisional.

The Rust-only baseline at d774b9f9a9a4a3cd4b1b53f56af34d24b3d82bdf measures
clock adapter construction and unchanged headless reconciliation. One adapter
allocates twice; 64 retained clock subscriptions allocate 203 times / 20,992
requested bytes per ignored focus event. This is not Go parity or timer latency.
Keep raw distributions, executable/source hashes and that exact baseline commit.

Add an opaque SubscriptionId with disjoint named and scoped representations.
Preserve every(impl Into<String>, ...) and named-string ordering. The opt-in scoped
key contains a private caller namespace TypeId plus diagnostic type_name, instance,
revision and variant. The full tuple is compared; do not replace identities with
a collision-prone short hash. Scoped keys clone inline without formatting or heap
allocation. Type identity is in-process, not a wire/persistence format. Ord groups
by diagnostic namespace, then TypeId, instance, revision and variant; TypeId's tie
order is not promised stable across builds. Same-ID behavior still retains the old
callback/interval/deadline. Changing behavior requires changing the identity.

ClockSchedule uses a private namespace and the same clock/generation/effective
period tuple as before. This keeps shortened countdown deadlines distinct and
keeps widget state out of callbacks. Third-party components can define their own
private tag type without coordinating string prefixes with the Runtime.

Validate the entire new configuration before modifying timers: reject the size,
zero-period and duplicate-key errors first. Then retain existing map entries whose
keys remain present and insert only new entries. Sort the at-most-64 incoming
descriptors and use binary search for retention; reuse BTreeMap nodes and keep
key-sorted due-event dispatch. Move the descriptor ID into the map key and store
only interval/callback/deadline in Timer, avoiding a duplicate owned ID and named
string clones even on retained entries. Preserve callbacks on unchanged IDs.

This also fixes a correctness defect: the old rebuild removed an existing entry
before discovering a duplicate and returning an error. Restoring valid model
configuration could then postpone that subscription's original deadline. A known
negative regression records the lost `a` callback, then must pass with both `a`
and `b` after the fix. The timer table is transactional on configuration errors;
model Update/effects are not rolled back. Callback construction and the Model's
Vec still allocate. Bigger inline keys trade bounded descriptor/node size for
fewer allocations; measure counts, bytes, p50/p95 and the zero-subscription path.
