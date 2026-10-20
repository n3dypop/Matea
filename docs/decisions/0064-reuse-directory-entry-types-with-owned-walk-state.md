# ADR 0064: Reuse directory-entry types with owned walk state

Status: implemented; measured and cross-platform qualification pending.

After incremental source parsing reduced large-file residency, small-file
traversal still needed improvement. The 8 KiB complete-AST experiment did not
provide a stable tiny-file gain and increased requested allocations on both
pinned corpora. Revert that experiment and keep its four cohorts and Miri test as
historical evidence. Do not adjust the acceptance threshold to retain it.

Use `DirEntry::file_type()` while enumerating a directory and carry owned
`(PathBuf, FileType)` pairs in the DFS stack. Reuse the already-validated root
file type. Exclude ignored names before requesting their type. This avoids an
unconditional `symlink_metadata` for every popped entry. The
[Rust API contract](https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_type)
states that this query does not follow links and commonly needs no extra system
call on Windows and Unix; some filesystems/platforms still require one. This is
not a measured syscall-count claim.

Drop each `DirEntry` after obtaining its path and type: on Unix it can retain an
open directory handle. The stack owns no such handle or external borrow.
Sibling entry names are unique, so unstable sorting by full path has the same
deterministic DFS order without stable-sort scratch storage. Preserve public
schema, relative UTF-8 IDs, failure status, ignored trees, linked-root rejection,
cycle/external-link skipping and non-regular-file handling.

The fixed-tree contract remains mandatory. Cached metadata does not protect
against hostile concurrent replacement. Errors during enumeration may precede
child processing; no exact partial-tree output on failure was promised. A zero
exit status alone qualifies a complete tree inventory.

Keep parser/serializer source bytes identical to the qualified incremental
checkpoint. Reuse its separately scoped parser measurements, verify eight
complete parser projections again, and run fresh balanced complete-CLI/RSS
measurements for 16 valid/boundary/error/tree cases. Native Linux, Windows and
macOS must run junction, FIFO, invalid-byte and explicit DFS-order regressions
where supported before portability is raised.
