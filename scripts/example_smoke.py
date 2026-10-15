"""Portable examples and reviewed dashboard golden. Run from any directory."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT
env = dict(os.environ, NO_COLOR="1", MATEA_REDUCED_MOTION="1")
for name, expected in [("counter", "Count: 0"), ("list", "Matcha"),
                       ("form", "Favorite tea"), ("dashboard", "Tea studio"),
                       ("effects", "Loading tea"), ("motion", "Matea motion"), ("input", "Matea input"),
                       ("pagination", "Page 1 of 4")]:
    result = subprocess.run(
        ["cargo", "run", "--locked", "--quiet", "-p", "matea", "--example", name,
         "--", "--snapshot"], cwd=WORKSPACE, env=env, check=True,
        capture_output=True, text=True, encoding="utf-8", timeout=60)
    assert expected in result.stdout, (name, result.stdout)
    assert "\x1b" not in result.stdout, name
    if name == "dashboard":
        golden = (WORKSPACE / "tests/golden/dashboard.txt").read_text(encoding="utf-8")
        assert result.stdout == golden, "dashboard golden mismatch"
    print(f"PASS {name} snapshot")
result = subprocess.run(
    ["cargo", "run", "--locked", "--quiet", "-p", "matea-example-external-widget",
     "--", "--snapshot"], cwd=WORKSPACE, env=env, check=True,
    capture_output=True, text=True, encoding="utf-8", timeout=60)
assert "Third-party badge" in result.stdout
assert "\x1b" not in result.stdout
print("PASS external widget snapshot")
