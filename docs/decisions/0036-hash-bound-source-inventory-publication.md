# ADR 0036: Validate a source-inventory cohort before publishing it

Status: accepted for Foundation evidence tooling.

The previous auditor wrote each reference immediately. A reproduced later pin
failure changed the first inventory while leaving the old summary in place.
The x/ansi and x/term snapshots also reused a persistent directory, allowing
deleted or unrelated old files to enter a later audit. These are evidence
integrity defects, not missing framework features.

Separate a read-only planning phase from publication. Validate every pin,
checkout, source identity, parser result and prior review before writing any
output. Materialize each pinned x module into a fresh invocation-owned temporary
directory, preserving bytes and only the requested tree. Retain matching marker
reviews at the same SHA; a repin resets them. Reuse validated source paths within
one inventory rather than resolving the same filename for each declaration.

Publish each file by atomic replacement, then publish summary schema 2 with the
exact inventory-file SHA256 values last. The ledger checker and source-path
checker require the complete matching cohort, including identities, pins and
integer counts. A publication I/O error or process interruption can still leave
mixed files, but readers reject them. This is a single-writer protocol, not a
global filesystem transaction, durability proof or distributed lock. Temporary
files are cleaned after ordinary errors. An unsuccessful planning phase leaves
the old output untouched.

Share strict JSON decoding between research tools: reject duplicate fields and
nonfinite values. Issue refresh also rejects boolean schema versions, invalid
prior identities and malformed metadata before publication. Nonfinite output
serialization fails without replacing old evidence. The existing issue snapshot
and its review dispositions are not refreshed or reset by this change.

Independent Git/filesystem/parser/IO fixtures cover late failures, dirty/wrong
pins, malformed and duplicate records, escaping source paths, old reviews,
deleted module files, interrupted publication, recovery and CLI completion.
Every field of all eleven real reference inventories equals the retained cohort,
including review metadata. Module/API closure and performance remain separate.
