//! Existing UTF-8 -> AST -> complete NDJSON bytes, separate from filesystem/CLI.
#[path = "../../perf-probe/src/measurement.rs"]
mod measurement;
use std::{fmt::Write, hint::black_box};

fn source(count: usize, nested: bool) -> String {
    let mut text = String::new();
    if nested {
        text.push_str("#[cfg(feature=\"parent\")] pub mod nested {\n");
    }
    for i in 0..count {
        writeln!(text, "#[cfg(unix)] pub const ITEM_{i}: usize = {i};").unwrap();
    }
    if nested {
        text.push_str("}\n");
    }
    text
}
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn main() {
    if !std::env::args().any(|s| s == "--audit-paths") {
        return;
    }
    let n = measurement::iterations(1);
    for count in [1000, 10000, 100000] {
        for nested in [false, true] {
            let input = source(count, nested);
            let mut expected = Vec::new();
            matea_api_audit::audit_source(&mut expected, "fixture.rs", &input).unwrap();
            // The emitted bytes are checked independently by the measurement
            // orchestrator before timing. Keep whole output ownership and hash
            // observable without printing 100,000 JSON rows per sample.
            let output = format!(
                "rows={};bytes={};fnv64={:016x}",
                count + usize::from(nested),
                expected.len(),
                digest(&expected)
            );
            measurement::measure(
                &format!(
                    "audit-parser/{}_{}",
                    if nested { "nested" } else { "flat" },
                    count
                ),
                n,
                || {
                    let mut out = Vec::with_capacity(expected.len());
                    matea_api_audit::audit_source(&mut out, "fixture.rs", black_box(&input))
                        .unwrap();
                    digest(black_box(&out))
                },
                &output,
                "existing fixed UTF8 string -> syn AST -> lexical declaration walk -> complete owned NDJSON -> FNV64; output allocation included; excludes source traversal, file I/O and process spawn; no compiler/cfg/macro resolution",
            );
        }
    }
    if let Some(folder) = std::env::var_os("MATEA_AUDIT_MANIFEST_DIR") {
        for name in ["crossterm", "ratatui"] {
            let manifest =
                std::fs::read_to_string(std::path::Path::new(&folder).join(format!("{name}.tsv")))
                    .unwrap();
            let sources: Vec<_> = manifest
                .lines()
                .map(|line| {
                    let (file, path) = line.split_once('\t').unwrap();
                    (file.to_owned(), std::fs::read_to_string(path).unwrap())
                })
                .collect();
            let mut expected = Vec::new();
            for (file, source) in &sources {
                matea_api_audit::audit_source(&mut expected, file, source).unwrap();
            }
            let output = format!(
                "rows={};bytes={};fnv64={:016x}",
                expected.iter().filter(|b| **b == b'\n').count(),
                expected.len(),
                digest(&expected)
            );
            measurement::measure(
                &format!("audit-parser/{name}"),
                n,
                || {
                    let mut out = Vec::with_capacity(expected.len());
                    for (file, source) in black_box(&sources) {
                        matea_api_audit::audit_source(&mut out, file, source).unwrap();
                    }
                    digest(black_box(&out))
                },
                &output,
                "existing pinned source strings in sorted manifest order -> AST -> complete lexical declaration NDJSON -> FNV64; output allocation included; excludes source traversal, file I/O and process spawn; no compiler/cfg/macro resolution",
            );
        }
    }
}
