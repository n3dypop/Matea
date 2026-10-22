"""Independent fault/identity fixtures for issue research state preservation."""
import copy
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import contextlib
import io
import unittest
from unittest.mock import patch
from fetch_upstream_issues import atomic_json, merge_repository, refresh
import fetch_upstream_issues as fetcher


def issue(number, **changes):
    return {"number": number, "title": "報告", "html_url": f"https://github.com/owner/project/issues/{number}",
            "updated_at": "old", "labels": [{"name": "bug"}], **changes}


class RefreshTests(unittest.TestCase):
    def test_reviews_survive_metadata_changes_disappearance_and_reopening(self):
        first = merge_repository("owner/project", [[issue(2), issue(1)]], {})
        first["issues"][0].update(disposition="fixed", matea_reproduction="confirmed", evidence=["regression"])
        saved = copy.deepcopy(first)
        second = merge_repository("owner/project", [[issue(2, updated_at="new")]], first)
        self.assertEqual(first, saved)
        self.assertEqual(second["issues"][0]["number"], 2)
        self.assertTrue(second["issues"][0]["upstream_updated_since_previous_snapshot"])
        self.assertEqual(second["no_longer_open"][0]["disposition"], "fixed")
        third = merge_repository("owner/project", [[issue(1)]], second)
        self.assertEqual(third["issues"][0]["evidence"], ["regression"])
        self.assertEqual(third["issues"][0]["matea_reproduction"], "confirmed")
        self.assertNotIn("listing_status", third["issues"][0])
        self.assertEqual(third["no_longer_open"][0]["number"], 2)

    def test_pagination_excludes_prs_but_rejects_duplicate_or_foreign_identity(self):
        result = merge_repository("owner/project", [[{**issue(7), "pull_request": {}}], [issue(3)]], {})
        self.assertEqual(result["open_issue_count"], 1)
        self.assertEqual(result["page_count"], 2)
        for pages in [[[issue(3)], [issue(3)]], [[issue(3, html_url="https://github.com/other/repo/issues/3")]],
                      [[issue(True)]], {}, [None]]:
            with self.subTest(pages=pages), self.assertRaises((AssertionError, TypeError)):
                merge_repository("owner/project", pages, {})

    def test_failed_later_fetch_keeps_input_and_published_evidence(self):
        previous = {"schema": 1, "repositories": []}
        saved = copy.deepcopy(previous)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "issues.json"
            atomic_json(path, previous)
            before = path.read_bytes()
            def fetch(repo):
                if repo == "second":
                    raise TimeoutError("offline fixture")
                return []
            with patch("fetch_upstream_issues.REPOSITORIES", ["first", "second"]):
                with self.assertRaises(TimeoutError):
                    atomic_json(path, refresh(previous, fetch))
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(previous, saved)
            self.assertEqual(list(path.parent.iterdir()), [path])

    def test_failed_replace_keeps_old_file_and_cleans_temporary(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "issues.json"
            atomic_json(path, {"old": "抹茶"})
            before = path.read_bytes()
            with patch("fetch_upstream_issues.os.replace", side_effect=PermissionError("fixture")):
                with self.assertRaises(PermissionError):
                    atomic_json(path, {"new": True})
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(json.loads(before), {"old": "抹茶"})
            self.assertEqual(list(path.parent.iterdir()), [path])

    def test_schema_previous_identity_and_metadata_are_validated_before_merge(self):
        for schema in [True,"1",2,None]:
            with self.subTest(schema=schema),self.assertRaises(AssertionError): refresh({"schema":schema},lambda _:[])
        for previous in [{"repository":"other/repo"}, {"issues":[{"number":True}]},
                         {"issues":[{"number":1},{"number":1}]},
                         {"issues":[{"number":1}],"no_longer_open":[{"number":1}]}]:
            with self.subTest(previous=previous),self.assertRaises(AssertionError):merge_repository("owner/project",[],previous)
        for change in [{"title":None},{"updated_at":""},{"labels":{}},{"labels":[{"name":7}]}]:
            with self.subTest(change=change),self.assertRaises(AssertionError):merge_repository("owner/project",[[issue(1,**change)]],{})

    def test_nonfinite_serialization_never_overwrites_or_leaves_a_temporary(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"issues.json";atomic_json(path,{"preserve":True});before=path.read_bytes()
            with self.assertRaises(ValueError):atomic_json(path,{"invalid":float("nan")})
            self.assertEqual(path.read_bytes(),before)
            self.assertEqual(list(path.parent.iterdir()),[path])

    def test_cli_late_network_or_json_failure_preserves_report_and_cache(self):
        for failure in [subprocess.TimeoutExpired("gh",60), '{"duplicate":0,"duplicate":1}', 'NaN']:
            with self.subTest(failure=failure),tempfile.TemporaryDirectory() as directory:
                path=Path(directory)/"issues.json";cache=Path(directory)/"private-cache"
                atomic_json(path,{"schema":1,"repositories":[]});before=path.read_bytes()
                calls=0
                def run(command,**kwargs):
                    nonlocal calls
                    calls+=1
                    if calls==1:return subprocess.CompletedProcess(command,0,"[[]]","")
                    if isinstance(failure,Exception):raise failure
                    return subprocess.CompletedProcess(command,0,failure,"")
                with patch.object(fetcher,"REPOSITORIES",["owner/first","owner/last"]),\
                     patch.object(fetcher.subprocess,"run",side_effect=run),\
                     patch.object(sys,"argv",["issues","--destination",str(path),"--cache",str(cache)]),\
                     contextlib.redirect_stdout(io.StringIO()),self.assertRaises((subprocess.TimeoutExpired,AssertionError,ValueError)):
                    fetcher.main()
                self.assertEqual(calls,2)
                self.assertEqual(path.read_bytes(),before)
                self.assertFalse(cache.exists())

    def test_cli_rejects_corrupted_prior_json_before_fetching(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"issues.json"
            for raw in ['{"schema":1,"schema":1}', '{"schema":1,"bad":NaN}']:
                path.write_text(raw,encoding="utf-8")
                with patch.object(sys,"argv",["issues","--destination",str(path)]),\
                     patch.object(fetcher.subprocess,"run") as run,self.assertRaises((AssertionError,ValueError)):
                    fetcher.main()
                run.assert_not_called()
                self.assertEqual(path.read_text(),raw)


if __name__ == "__main__":
    unittest.main()
