"""Fetch fixed public reference snapshots outside the Matea checkout for CI."""
from evidence_checks import require
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", choices=["crossterm", "ratatui"], required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    destination = args.destination.resolve()
    if destination.is_relative_to(ROOT):
        parser.error("references must stay outside Matea")
    pin = next(p for p in json.loads((ROOT / "docs/upstream-lock.json").read_text(encoding="utf-8")) if p["name"] == args.reference)
    checkout = destination / args.reference
    if checkout.exists():
        parser.error("choose an empty reference destination; existing checkouts are never reset")
    checkout.mkdir(parents=True)
    for command in [["git", "init"], ["git", "remote", "add", "origin", pin["remote"]],
                    ["git", "fetch", "--depth", "1", "origin", pin["sha"]],
                    ["git", "checkout", "--detach", "FETCH_HEAD"]]:
        subprocess.run(command, cwd=checkout, check=True)
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=checkout, text=True).strip()
    require(actual == pin["sha"])
    print(f"{args.reference} at {actual}")


if __name__ == "__main__":
    main()
