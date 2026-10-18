"""Validate retained attribute measurements, including explicit Debug evolution."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from optimization_evidence import compare, records, summary
from performance_records import parse_json, write_report

ROOT = Path(__file__).resolve().parents[1]
COLOR = r"(?:Default|Indexed\(\d+\)|Rgb\(\d+, \d+, \d+\))"
LEGACY_STYLE = re.compile(r"Style \{ fg: ("+COLOR+r"), bg: ("+COLOR+
                          r"), bold: (true|false), underline: (true|false), reverse: (true|false) \}")
THEMES = {"style/theme_default", "style/theme_plain"}


def normalize_legacy_theme(text):
    """Add only defaults absent from the old resolved representation.

    This is never applied to raw files or timing values. Full normalized output
    must equal the new raw output, so extra attributes/fields cannot disappear.
    """
    def style(match):
        fg, bg, bold, underline, reverse = match.groups()
        flags = ", ".join(name for name, present in [("Bold", bold), ("Reverse", reverse)] if present == "true")
        return ("Style { fg: "+fg+", bg: "+bg+", underline_color: Default, attributes: TextAttributes { flags: {"+
                flags+"}, underline: "+("Single" if underline=="true" else "None")+", blink: None, frame: None } }")
    result, count = LEGACY_STYLE.subn(style, text)
    if count != 7 or not text.startswith("Theme { text: Style {"):
        raise ValueError("expected seven complete legacy theme styles")
    return result


def legacy_comparison(before, after):
    if set(before) != set(after):
        raise ValueError("legacy workload set changed")
    normalized = {}
    for name, item in before.items():
        value = dict(item)
        if name in THEMES:
            value["output"] = normalize_legacy_theme(item["output"])
        normalized[name] = value
    rows = compare(normalized, after, len(before))
    for row in rows:
        row["output_comparison"] = "explicit_legacy_theme_defaults" if row["case"] in THEMES else "exact_raw_bytes"
    return rows


def load_stage(directory, stage):
    context = parse_json((directory/(stage+"-context.json")).read_text(encoding="utf-8"))
    labels = ["style", "views", "default", "cursor"] + ([] if stage=="before" else ["attributes"])
    expected = {stage+"-"+label+".jsonl" for label in labels}
    if set(context["raw_sha256"]) != expected:
        raise ValueError("raw cohort membership changed: "+stage)
    data = {}
    for name, expected_hash in context["raw_sha256"].items():
        raw = (directory/name).read_bytes()
        if hashlib.sha256(raw).hexdigest() != expected_hash:
            raise ValueError("raw hash mismatch: "+name)
        for case, item in records(raw.decode("utf-8")).items():
            if case in data: raise ValueError("duplicate cross-cohort case: "+case)
            data[case] = item
    if not context["source_sha256"]:
        raise ValueError("empty source snapshot")
    for name, expected_hash in context["source_sha256"].items():
        path = Path(name)
        if path.is_absolute() or ".." in path.parts or path.as_posix()!=name:
            raise ValueError("noncanonical source path")
        raw = subprocess.check_output(["git", "show", context["git_head"]+":"+name],cwd=ROOT)
        if hashlib.sha256(raw).hexdigest()!=expected_hash:
            raise ValueError("source hash mismatch: "+name)
    return context, data


def compact(rows):
    for row in rows:
        row["output_sha256"] = hashlib.sha256(row.pop("output").encode("utf-8")).hexdigest()
    return rows


def generate(directory):
    stages = {name: load_stage(directory, name) for name in ["before", "initial", "after"]}
    before = stages["before"][1]
    initial = stages["initial"][1]
    after = stages["after"][1]
    if len(before)!=157 or len(initial)!=171 or len(after)!=171:
        raise ValueError("incomplete attribute/control cohorts")
    legacy = {k:v for k,v in after.items() if k in before}
    first_legacy = {k:v for k,v in initial.items() if k in before}
    reference = []
    for mask in [0,1,85,255]:
        matea = after[f"attributes/flags_{mask}_matea"]
        upstream = after[f"attributes/flags_{mask}_crossterm"]
        for field in ["semantics", "output", "first_return_value", "iterations_per_sample"]:
            if matea[field]!=upstream[field]: raise ValueError("unmatched reference fixture: "+field)
        reference.append({"mask":mask,"output_equivalent_for_this_fixture":True,
                          "matea":summary(matea),"crossterm_0_29_0":summary(upstream)})
    return {"schema":1,"stage":"individual_modules","contexts":{k:v[0] for k,v in stages.items()},
            "legacy_initial":compact(legacy_comparison(before, first_legacy)),
            "legacy_final":compact(legacy_comparison(before, legacy)),
            "cache_change_all_controls":compact(compare(initial,after,171)),
            "matched_positive_flags":reference,
            "limits":["155 legacy outputs match raw bytes; exactly two theme Debug outputs require the explicit new-default transform. Raw records remain untouched.",
                      "One native macOS arm64 host with 31 samples; separate requested allocation pass, no affinity/frequency control or resident-memory evidence.",
                      "Eight-flag construction/query equivalence is not full Crossterm/Lip Gloss behavior or rendering throughput parity.",
                      "Preserve p50/p95 and allocation regressions; no quality-score promotion or universal maximum-performance claim."]}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory",type=Path)
    parser.add_argument("--check",action="store_true",help="validate without replacing retained evidence")
    args=parser.parse_args()
    report=generate(args.directory)
    report["reporter_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    write_report(args.directory/"report.json",json.dumps(report,indent=2)+"\n")
    print("157 legacy controls and 171 cache-change controls validated; 4 matched reference flag fixtures")


if __name__=="__main__":
    main()
