# ADR 0048: Source-checked measurement attempts

Status: implemented; source c5069e0 qualifies on Linux, Windows and macOS.
Evidence: ../evidence/measurement-lifecycle/ci-source-bindings.json.

A rejected invocation of the auditor and remaining-workload collectors left the
previous successful context untouched. Two independent failing tests reproduce
that stale-success state. Recording only the end of a run is insufficient.

Create an authoritative running journal before checking sources or building.
Keep every attempt in its own directory, including partial complete cases,
failures and interrupts. Publish the conventional raw file only after complete
output/comparability checks and a second source snapshot. Complete status and its
raw hash are the final atomic manifest write. Raw files without a matching
complete manifest cannot qualify the invocation. A killed process remains
running, never implicitly successful. One writer owns each output directory;
this is not a distributed lock or power-loss durability guarantee.

Use binary atomic writes so Windows EOL conversion cannot change generated
synthetic fixtures relative to their recorded hashes. Retain the previously
discarded first auditor process interval as cold_ns, followed by all 31 positive
warm intervals; reject invalid intervals without retry or outlier removal.
This changes evidence lifecycle, not the measured auditor operation.

Captured compiler extraction must validate the entire source snapshot against
its collector manifest, bind the manifest bytes to the completed run, require
the same extractor and matching native profile identities, verify raw/compiler
metadata before and after collection, and compare the whole generated graph.
A source manifest is provenance/integrity evidence from a pinned collector,
not cryptographic authentication of untrusted third-party artifacts. Native
compilation and source fetching remain outside extraction timing.

Go preparation validates all five clean pins before publication, uses explicit
preparing/failed/complete metadata and records hashes of its four generated
inputs. Paths remain outside Matea and references. Partial copy failures never
publish complete status. Attribute reporting gains the shared read-only check
mode; historical data and thresholds are preserved.

Qualify actual complete collectors, not only syntax or mocked process calls,
on Linux, Windows and macOS. Collect only the two missing native all-feature
captures; retain existing twenty-profile API evidence separately. CI timing
values qualify execution/schema and cannot establish a performance improvement.
Other reporter/prepare performance paths and resident memory remain pending.
