"""Prepare a disposable pinned Go module without modifying/vendoring references."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from measurement_run import atomic_bytes, json_bytes

ROOT=Path(__file__).resolve().parents[1]


def prepare(references, destination, root=ROOT):
    lock={r["name"]:r for r in json.loads((root/"docs/upstream-lock.json").read_bytes())}
    references, destination=references.resolve(),destination.resolve()
    if destination.is_relative_to(root.resolve()) or destination.is_relative_to(references):
        raise ValueError("comparison workspace must stay outside Matea and reference checkouts")
    names=["ultraviolet","harmonica","bubbles","bubbletea","lipgloss"]
    for name in names:
        checkout=references/name
        actual=subprocess.check_output(["git","rev-parse","HEAD"],cwd=checkout,text=True,timeout=30).strip()
        dirty=subprocess.check_output(["git","status","--porcelain"],cwd=checkout,text=True,timeout=30)
        if actual!=lock[name]["sha"] or dirty:
            raise ValueError("reference is not the clean pinned input: "+name)
    source,motion,bindings,tea,lip=(references/name for name in names)
    module=f"""module matea-foundation-comparison

go 1.26.0

require github.com/charmbracelet/ultraviolet v0.0.0-20261008173134-6b8d4baf91b4
require github.com/charmbracelet/harmonica v0.2.0
require charm.land/bubbles/v2 v2.0.0

replace github.com/charmbracelet/ultraviolet => {json.dumps(str(source))}
replace github.com/charmbracelet/harmonica => {json.dumps(str(motion))}
replace charm.land/bubbles/v2 => {json.dumps(str(bindings))}
replace charm.land/bubbletea/v2 => {json.dumps(str(tea))}
replace charm.land/lipgloss/v2 => {json.dumps(str(lip))}
"""
    files={name:(root/"tools/perf-upstream"/name).read_bytes() for name in ["main.go","main_test.go","go.sum"]}
    files["go.mod"]=module.encode("utf-8")
    pin={name+"_sha":lock[name]["sha"] for name in names}
    pin.update(harness="tools/perf-upstream/main.go",status="preparing")
    destination.mkdir(parents=True,exist_ok=True)
    atomic_bytes(destination/"source-pin.json",json_bytes(pin))
    try:
        for name,data in files.items():
            atomic_bytes(destination/name,data)
        pin.update(status="complete",file_sha256={name:hashlib.sha256(data).hexdigest() for name,data in files.items()})
        atomic_bytes(destination/"source-pin.json",json_bytes(pin))
    except Exception as error:
        pin.update(status="failed",error_type=type(error).__name__)
        atomic_bytes(destination/"source-pin.json",json_bytes(pin))
        raise
    return destination


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--references",type=Path,required=True)
    parser.add_argument("--destination",type=Path,required=True)
    args=parser.parse_args()
    print(prepare(args.references,args.destination))


if __name__=="__main__":
    main()
