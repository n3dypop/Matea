"""Read every page of open issues for the scoped upstream repositories.

Only public issue metadata is published; bodies stay in a temporary research cache.
"""
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
repos = ["charmbracelet/" + name for name in ["bubbletea", "lipgloss", "bubbles", "huh", "glamour", "harmonica", "ultraviolet", "x"]]
repos += ["ratatui/ratatui", "crossterm-rs/crossterm"]
cache = Path(tempfile.gettempdir()) / "matea-upstream-issues"
cache.mkdir(exist_ok=True)
records = []
for repo in repos:
    result = subprocess.run(["gh", "api", "--paginate", "--slurp", f"repos/{repo}/issues?state=open&per_page=100"], capture_output=True, text=True, check=True)
    pages = json.loads(result.stdout)
    issues = [issue for page in pages for issue in page if "pull_request" not in issue]
    (cache / (repo.replace("/", "-") + ".json")).write_text(json.dumps(issues))
    records.append({"repository": repo, "page_count": len(pages), "open_issue_count": len(issues), "issues": [{"number": i["number"], "title": i["title"], "url": i["html_url"], "updated_at": i["updated_at"], "labels": [label["name"] for label in i["labels"]], "matea_reproduction": "pending", "disposition": "untriaged_report_not_confirmed_defect"} for i in issues]})
    print(f"{repo}: {len(issues)} open issues, {len(pages)} API pages", flush=True)
target = ROOT / "docs/coverage/upstream-issues.json"
target.parent.mkdir(exist_ok=True)
target.write_text(json.dumps({"schema": 1, "as_of_utc": datetime.now(timezone.utc).isoformat(), "scope": "open issues only; PRs excluded; closed issues not enumerated", "repositories": records}, indent=2, ensure_ascii=False) + "\n")
