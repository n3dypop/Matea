"""Prepare a disposable pinned Go module without modifying/vendoring references."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument("--references",type=Path,required=True)
parser.add_argument("--destination",type=Path,required=True)
args=parser.parse_args()
lock={r["name"]:r for r in json.loads((ROOT/"docs/upstream-lock.json").read_text())}
source=args.references.resolve()/"ultraviolet"
motion=args.references.resolve()/"harmonica"
bindings=args.references.resolve()/"bubbles"
tea=args.references.resolve()/"bubbletea"
lip=args.references.resolve()/"lipgloss"
assert subprocess.check_output(["git","-C",str(lip),"rev-parse","HEAD"],text=True).strip()==lock["lipgloss"]["sha"]
assert not subprocess.check_output(["git","-C",str(lip),"status","--porcelain"],text=True)
assert subprocess.check_output(["git","-C",str(tea),"rev-parse","HEAD"],text=True).strip()==lock["bubbletea"]["sha"]
assert not subprocess.check_output(["git","-C",str(tea),"status","--porcelain"],text=True)
assert subprocess.check_output(["git","-C",str(bindings),"rev-parse","HEAD"],text=True).strip()==lock["bubbles"]["sha"]
assert not subprocess.check_output(["git","-C",str(bindings),"status","--porcelain"],text=True)
assert subprocess.check_output(["git","-C",str(motion),"rev-parse","HEAD"],text=True).strip()==lock["harmonica"]["sha"]
assert not subprocess.check_output(["git","-C",str(motion),"status","--porcelain"],text=True)
actual=subprocess.check_output(["git","-C",str(source),"rev-parse","HEAD"],text=True).strip()
assert actual==lock["ultraviolet"]["sha"]
assert not subprocess.check_output(["git","-C",str(source),"status","--porcelain"],text=True)
destination=args.destination.resolve();destination.mkdir(parents=True,exist_ok=True)
shutil.copy2(ROOT/"tools/perf-upstream/main.go",destination/"main.go")
(destination/"go.mod").write_text(f'''module matea-foundation-comparison

go 1.26.0

require github.com/charmbracelet/ultraviolet v0.0.0-20261008173134-6b8d4baf91b4
require github.com/charmbracelet/harmonica v0.2.0
require charm.land/bubbles/v2 v2.0.0

replace github.com/charmbracelet/ultraviolet => {json.dumps(str(source))}
replace github.com/charmbracelet/harmonica => {json.dumps(str(motion))}
replace charm.land/bubbles/v2 => {json.dumps(str(bindings))}
replace charm.land/bubbletea/v2 => {json.dumps(str(tea))}
replace charm.land/lipgloss/v2 => {json.dumps(str(lip))}
''')
(destination/"source-pin.json").write_text(json.dumps({"ultraviolet_sha":actual,"harmonica_sha":lock["harmonica"]["sha"],"bubbles_sha":lock["bubbles"]["sha"],"bubbletea_sha":lock["bubbletea"]["sha"],"lipgloss_sha":lock["lipgloss"]["sha"],"harness":"tools/perf-upstream/main.go"},indent=2)+"\n")
print(destination)
