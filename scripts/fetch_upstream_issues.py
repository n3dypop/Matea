"""Refresh open upstream issues without discarding completed Matea reviews.

Only public metadata is published. Bodies remain in a temporary research cache.
A missing open-issue entry is archived as no longer listed, not inferred closed.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import tempfile
from evidence_checks import require
from check_coverage import load_json, parse_json

ROOT = Path(__file__).resolve().parents[1]
REPOSITORIES = ["charmbracelet/" + name for name in ["bubbletea", "lipgloss", "bubbles", "huh", "glamour", "harmonica", "ultraviolet", "x"]]
REPOSITORIES += ["ratatui/ratatui", "crossterm-rs/crossterm"]


def unique_by(items, key):
    require(isinstance(items, list), "invalid previous issue collection")
    result = {}
    for item in items:
        value = item[key]
        require((type(value) is int and value > 0) if key == "number" else
                (isinstance(value, str) and bool(value)), "invalid prior " + key)
        require(value not in result, "duplicate " + key)
        result[value] = item
    return result


def merge_repository(repository, pages, previous):
    require(previous.get("repository", repository) == repository, "prior issue repository mismatch")
    require(isinstance(pages, list) and all(isinstance(p, list) for p in pages), "invalid paginated issues")
    active = unique_by(previous.get("issues", []), "number")
    archived = unique_by(previous.get("no_longer_open", []), "number")
    require(not active.keys() & archived.keys(), "overlapping active/archived issue IDs")
    known = {**archived, **active}
    issues = []
    seen = set()
    for page in pages:
        for issue in page:
            if "pull_request" in issue:
                continue
            number = issue["number"]
            require(type(number) is int and number > 0 and number not in seen, "invalid/duplicate issue number")
            seen.add(number)
            require(issue["html_url"] == f"https://github.com/{repository}/issues/{number}", "issue repository identity mismatch")
            require(isinstance(issue["title"], str) and isinstance(issue["updated_at"], str)
                    and bool(issue["updated_at"]) and isinstance(issue["labels"], list)
                    and all(isinstance(label["name"], str) for label in issue["labels"]), "invalid issue metadata")
            old = known.get(number, {})
            current = {**old, "number": number, "title": issue["title"], "url": issue["html_url"],
                       "updated_at": issue["updated_at"], "labels": [label["name"] for label in issue["labels"]]}
            current.pop("listing_status", None)
            current.setdefault("matea_reproduction", "pending")
            current.setdefault("disposition", "untriaged_report_not_confirmed_defect")
            if old and old["updated_at"] != issue["updated_at"]:
                current["upstream_updated_since_previous_snapshot"] = True
            issues.append(current)
    missing = [{**known[number], "listing_status": "not_in_current_open_snapshot"}
               for number in sorted(known.keys() - seen)]
    return {"repository": repository, "page_count": len(pages), "open_issue_count": len(issues),
            "issues": sorted(issues, key=lambda i: i["number"]), "no_longer_open": missing}


def refresh(previous, fetch_pages):
    require(type(previous.get("schema", 1)) is int and previous.get("schema", 1) == 1, "unsupported issue schema")
    known = unique_by(previous.get("repositories", []), "repository")
    records = [merge_repository(repo, fetch_pages(repo), known.get(repo, {})) for repo in REPOSITORIES]
    return {"schema": 1, "as_of_utc": datetime.now(timezone.utc).isoformat(),
            "scope": "open issues only; PRs excluded; prior review records retained when no longer listed; closed issues not enumerated",
            "repositories": records}


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as output:
            temporary = Path(output.name)
            json.dump(value, output, indent=2, ensure_ascii=False, allow_nan=False)
            output.write("\n")
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, default=ROOT / "docs/coverage/upstream-issues.json")
    parser.add_argument("--cache", type=Path, default=Path(tempfile.gettempdir()) / "matea-upstream-issues")
    args = parser.parse_args()
    previous = load_json(args.destination) if args.destination.exists() else {}
    cache = {}

    def fetch(repository):
        result = subprocess.run(["gh", "api", "--paginate", "--slurp", f"repos/{repository}/issues?state=open&per_page=100"],
                                capture_output=True, text=True, encoding="utf-8", check=True, timeout=60)
        pages = parse_json(result.stdout)
        cache[repository] = [issue for page in pages for issue in page if "pull_request" not in issue]
        print(f"{repository}: {len(cache[repository])} open issues, {len(pages)} API pages", flush=True)
        return pages

    report = refresh(previous, fetch)
    # No published evidence is changed until every repository has been validated.
    for repository, issues in cache.items():
        atomic_json(args.cache / (repository.replace("/", "-") + ".json"), issues)
    atomic_json(args.destination, report)


if __name__ == "__main__":
    main()
