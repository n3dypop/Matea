# ADR 0033: Reject ambiguous public-capability evidence

Status: accepted for Foundation evidence tooling.

The coverage checker constructed a dictionary from performance cases without checking uniqueness. A later successful case could overwrite an earlier failed case with the same ID. It also accepted truthy values such as the string `"false"` as output equivalence. Independent synthetic ledgers reproduce these failures on the preceding checker; the failure log and source hashes are retained under `docs/evidence/optimization/coverage-guard/`.

Validate JSON fields and case IDs before lookup. Equivalence and closure flags are actual booleans; successful compiler exits are integer zero, excluding boolean false. Implemented capabilities require source and correctness evidence, and performance evidence must be an existing project file or the explicit pending/owner-deferred state. Evidence paths must be canonical project-relative files, including after symlink resolution. Source inventory SHAs must match the pinned revisions.

Compiler snapshots additionally bind reference, package, profile, target and SHA across the index, run and hashed graph. Reject duplicate profiles and export keys. These checks detect inconsistent or malformed recorded evidence; they do not establish its scientific independence, review every feature combination, or substitute for executing reference behavior.

The checker is import-safe and read-only. Ten tests use a small independent ledger and corrupted variants, including an otherwise valid partial ledger whose `--gate` must remain closed. Normal and optimized Python execute the checks. The real ledger retains every pending capability, issue and module; no acceptance threshold or scope is reduced.
