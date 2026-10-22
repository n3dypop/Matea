# ADR 0016: Compiler API coverage before renderer integration

Status: audit tooling implemented; full target/feature closure and renderer bridge
selection remain pending. This does not alter the F5 or image-engine gates.

The source inventories deliberately leave Rust macro/cfg/export resolution open.
Source-level public visibility is insufficient: public methods on private types
can be unreachable, bitflags/derives generate callable APIs, glob/alias exports
change names, and a public type in a private module can be usable through a public
return value. An experiment with the actual nightly compiler shows rustdoc omits
that inferred type's public methods unless private items are included in the raw
graph. Collect hidden/private metadata, then traverse only public module/reexport
edges and public types referenced by reachable signatures. Private unrelated
subtrees stay excluded. Annotate paths that can be used only through inferred types
or trait-call notation rather than promising every representative path is importable.

Compile exact Git snapshots with their tracked lockfiles in temporary copies
outside Matea. Verify reference SHA/cleanliness and input hashes; never substitute
the published package just because its version label matches. The registry's
ratatui-core 0.1.2 identifies commit e665c36cb14752a61cd777fbd06dbef8474f2add,
while the approved reference is d1ef782b48ddf0cb825c111d4d8b9adef572c0e1.
That difference requires review, not an assumption of API or behavior equality.

Use a recorded nightly compiler and checked rustdoc JSON format. Keep raw rustdoc,
sources and source docs outside the repository; import normalized names, public
headers, source locations, reexport/signature/trait edges, hashes and build logs.
Keep locally declared blanket implementations; classify compiler-synthetic and
external blanket implementations separately rather than treating all std blanket
methods as newly implemented TUI features. Capture dependency feature trees.

Each compiled profile remains partial evidence. An initial three-OS matrix covers
Crossterm minimal/default/all, Ratatui Core/Widgets all, facade default and facade
all on Unix. Alternative Crossterm versions, no_std, other feature combinations,
individual Windows-supported backends and external signature dependencies still
need review. Windows facade all enables Unix Termion; its omission is recorded as
pending platform review rather than excluding every optional backend on Windows.
Green jobs cannot promote unresolved coverage. The strict gate separately checks
compiler export/impl keys, source capability links and closure flags.

This audit informs ADR 0004's required Ratatui reevaluation. The pinned Buffer
supports CellDiffOption::Skip/AlwaysUpdate/ForcedWidth; arbitrary symbols can
contain escape sequences. A blind copy into Matea cells loses these contracts or
bypasses sanitization. Style modifiers/underline color and StatefulWidget render
state also need explicit translation. A future managed buffer/widget bridge must
preserve or explicitly reject unsupported cases and keep terminal restoration in
Matea's transport; reexporting Ratatui or nesting another terminal session is not
integration evidence. No renderer bridge or broader widget catalog is claimed in
this increment. The narrow compositor remains in use while that review proceeds.

Sources: fixed Ratatui ratatui-core/src/buffer/cell.rs:13–72 and widgets.rs;
Crossterm src/lib.rs, command.rs and event.rs; normalized compiler snapshots and
regressions in scripts/test_rustdoc_exports.py. Pure protocol encoding does not
authorize image processing, decoding or CUDA.
