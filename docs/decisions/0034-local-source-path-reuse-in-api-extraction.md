# ADR 0034: Reuse source path normalization within one compiler graph

Status: accepted for Foundation evidence tooling.

The compiler importer was measured, but extraction from the original compiler JSON was still missing. Add two optional workloads using existing external collector directories: Crossterm/all and Ratatui Core/all on the native target. Bind each input to the recorded pin, package, profile, successful run and original rustdoc SHA256. Read and parse the complete JSON, extract the graph and serialize it canonically. Compare every graph field with the prior local compiler record before accepting any sample. Raw documentation/source bodies stay outside Matea.

A separate diagnostic profile of Ratatui Core showed 3,926 location calls, 989 path resolutions and 9,389 readlink calls. Repeated source filenames were resolved again for declarations and implementations. Cache only normalized filename/outside-reference pairs within one `exports` invocation. Construct each line/column record independently. Do not share state across invocations or source roots; the compiler snapshot is fixed during extraction.

Before/after runs retain 31 sequential samples each. Complete Crossterm extraction changes from p50/p95 103.73/116.40 to 95.68/108.13 ms; Ratatui Core changes from 233.30/253.77 to 192.73/216.28 ms. Complete outputs and input hashes match. Separate Python traced peaks remain 27,972,171 and 51,532,174 bytes because parsing dominates those peaks. No RSS reduction, compiler/network speedup or whole-unit maximum-performance verdict is inferred.

[Raw distributions, compiler records, hashes and separate profiles](../evidence/optimization/compiler-extraction/report.json). The independent location fixture verifies distinct coordinates, mutation independence and switching the same filename between inside/outside roots. Existing alias/private-signature/external-trait fixtures remain required. All three OSes must execute the changed extractor in the compiler collection matrix before updating its portability assessment.
