"""Finite independent source/IO fault fixtures; no network or upstream compiler."""
import contextlib
import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import audit_references as audit
from check_coverage import verify_inventory_summary


class ReferenceAuditTests(unittest.TestCase):
    def test_git_windows_checkout_preserves_hash_bound_inventory_bytes(self):
        attributes=Path(__file__).resolve().parents[1]/".gitattributes"
        folder=self.refs/"first"
        (folder/".gitattributes").write_bytes(attributes.read_bytes())
        target=folder/"docs/coverage/fixture.json";target.parent.mkdir(parents=True)
        expected=b'{\n  "finite": true\n}\n';target.write_bytes(expected)
        self.commit(folder)
        self.git(folder,"-c","core.autocrlf=true","checkout-index","--all","--force")
        self.assertEqual(target.read_bytes(),expected)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="matea-cohort-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.refs = self.root / "references"
        self.out = self.root / "output"
        self.pins = [self.repository(name, {"api.go":"package fixture\nconst Tea = 1 // TODO review\n", "go.mod":"module fixture\n"})
                     for name in ["first", "last"]]

    def git(self, folder, *arguments):
        return subprocess.check_output(["git", *arguments], cwd=folder, stderr=subprocess.PIPE)

    def commit(self, folder):
        self.git(folder, "add", "--all")
        self.git(folder, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "-c", "commit.gpgsign=false", "-c", "core.hooksPath="+str(self.root/"empty-hooks"),
                 "commit", "--quiet", "-m", "fixture")
        return self.git(folder, "rev-parse", "HEAD").decode().strip()

    def repository(self, name, files):
        folder = self.refs / name
        folder.mkdir(parents=True)
        for relative, text in files.items():
            path = folder/relative; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(text, encoding="utf-8")
        self.git(folder, "init", "--quiet")
        return {"name":name, "sha":self.commit(folder), "remote":"https://example.invalid/"+name+".git"}

    def auditor(self, tool, source):
        self.assertEqual(str(tool), "go" if (source/"go.mod").exists() else "rust")
        suffix = "*.go" if str(tool) == "go" else "*.rs"
        return "".join(json.dumps({"file":p.relative_to(source).as_posix(), "line":2,
            "kind":"const", "symbol":p.stem})+"\n" for p in sorted(source.rglob(suffix)))

    def plan(self, pins=None, auditor=None):
        return audit.plan(self.refs, self.pins if pins is None else pins, self.out, "go", "rust", auditor=auditor or self.auditor)

    def snapshot(self):
        return {p.relative_to(self.out).as_posix():p.read_bytes() for p in self.out.rglob("*") if p.is_file()} if self.out.exists() else {}

    def test_late_pin_dirty_parser_and_launch_failures_publish_nothing(self):
        audit.publish(self.out, self.plan())
        before = self.snapshot()
        wrong = copy.deepcopy(self.pins); wrong[-1]["sha"] = "a"*40
        with self.assertRaises(AssertionError): self.plan(pins=wrong)
        self.assertEqual(self.snapshot(), before)
        dirty = self.refs/"last/api.go"; original = dirty.read_bytes(); dirty.write_bytes(original+b"// dirty\n")
        with self.assertRaises(AssertionError): self.plan()
        dirty.write_bytes(original)
        self.assertEqual(self.snapshot(), before)
        for failure in [OSError("launch"), subprocess.CalledProcessError(7, ["fixture"]), subprocess.TimeoutExpired("fixture", 120)]:
            def fail_last(tool, source):
                if source.name == "last": raise failure
                return self.auditor(tool, source)
            with self.subTest(failure=type(failure).__name__), self.assertRaises(type(failure)): self.plan(auditor=fail_last)
            self.assertEqual(self.snapshot(), before)
        self.assertEqual(len(before), 3)

    def test_malformed_duplicate_or_escaping_declarations_cannot_publish(self):
        audit.publish(self.out, self.plan()); before = self.snapshot()
        valid = {"file":"api.go", "line":2, "kind":"const", "symbol":"Tea"}
        outputs = ['{', '[]', '{"file":"api.go","file":"other.go"}',
                   json.dumps(valid)+"\n"+json.dumps(valid), json.dumps({**valid,"line":True}),
                   json.dumps({**valid,"line":0}), json.dumps({**valid,"symbol":""}),
                   json.dumps({**valid,"file":"../outside.go"}), json.dumps({**valid,"file":"missing.go"}),
                   json.dumps(valid)[:-1]+',"number":NaN}']
        for output in outputs:
            def malformed_last(tool, source): return output if source.name == "last" else self.auditor(tool, source)
            with self.subTest(output=output), self.assertRaises((ValueError, AssertionError, KeyError, TypeError)):
                self.plan(auditor=malformed_last)
            self.assertEqual(self.snapshot(), before)

    def test_review_fields_survive_only_the_same_pin_and_plan_is_read_only(self):
        audit.publish(self.out, self.plan())
        path=self.out/"first.json"; old=json.loads(path.read_text()); marker=old["issue_markers"][0]
        marker.update(assessment="bounded", analysis="prior review", matea_evidence=["proof.rs"])
        path.write_bytes(audit.serialized(old)); before=self.snapshot()
        planned=self.plan(); current=json.loads(planned["first.json"])
        self.assertEqual(current["issue_markers"][0], marker)
        self.assertEqual(self.snapshot(), before)
        (self.refs/"first/api.go").write_text("package fixture\nconst Tea = 2 // TODO review\n", encoding="utf-8")
        pins=copy.deepcopy(self.pins); pins[0]["sha"]=self.commit(self.refs/"first")
        current=json.loads(self.plan(pins=pins)["first.json"])
        self.assertNotIn("assessment", current["issue_markers"][0])
        self.assertEqual(self.snapshot(), before)

    def test_x_module_uses_new_exact_pinned_tree_after_deleted_files(self):
        pin=self.repository("charm-x", {"ansi/api.go":"package ansi\nconst Tea = 1\n", "ansi/go.mod":"module fixture\n",
                                      "term/other.go":"package term\nconst Other = 1\n"})
        pin["name"]="charm-x-ansi"
        seen=[]
        def inspect(tool, source):
            seen.append(source)
            self.assertFalse((source/"term").exists())
            return self.auditor(tool, source)
        first=json.loads(self.plan(pins=[pin],auditor=inspect)["charm-x-ansi.json"])
        folder=self.refs/"charm-x"; (folder/"ansi/api.go").unlink()
        (folder/"ansi/fresh.go").write_text("package ansi\nconst Fresh = 1\n",encoding="utf-8")
        pin["sha"]=self.commit(folder)
        second=json.loads(self.plan(pins=[pin],auditor=inspect)["charm-x-ansi.json"])
        self.assertEqual([d["file"] for d in first["declarations"]],["ansi/api.go"])
        self.assertEqual([d["file"] for d in second["declarations"]],["ansi/fresh.go"])
        self.assertNotEqual(seen[0],seen[1])
        self.assertTrue(all(not source.exists() for source in seen))

    def test_interrupted_publication_is_detected_and_temporaries_are_removed(self):
        old=self.plan(); audit.publish(self.out,old); changed=dict(old)
        record=json.loads(changed["first.json"]);record["inventory_kind"]="different validated cohort"
        changed["first.json"]=audit.serialized(record)
        summary=json.loads(changed["summary.json"]);summary["inventory_sha256"]["first.json"]=hashlib.sha256(changed["first.json"]).hexdigest()
        changed["summary.json"]=audit.serialized(summary)
        real_replace=audit.os.replace
        def fail_second(source,target):
            if target.name=="last.json":raise OSError("publication interrupted")
            return real_replace(source,target)
        with patch.object(audit.os,"replace",side_effect=fail_second),self.assertRaises(OSError): audit.publish(self.out,changed)
        inventories=[json.loads((self.out/(p["name"]+".json")).read_text()) for p in self.pins]
        with self.assertRaisesRegex(AssertionError,"incomplete/stale"):verify_inventory_summary(self.out,inventories)
        self.assertEqual(set(self.snapshot()),set(old))
        audit.publish(self.out,changed)
        verify_inventory_summary(self.out,[json.loads(changed[p["name"]+".json"]) for p in self.pins])

    def test_cli_reports_success_only_after_complete_publication(self):
        docs=self.root/"docs";docs.mkdir();(docs/"upstream-lock.json").write_text(json.dumps(self.pins),encoding="utf-8")
        arguments=["audit","--references",str(self.refs),"--destination",str(self.out),"--go-tool","go","--rust-tool","rust"]
        with patch.object(audit,"ROOT",self.root),patch.object(sys,"argv",arguments),\
             patch.object(audit,"run_auditor",side_effect=self.auditor),contextlib.redirect_stdout(io.StringIO()) as output:
            audit.main()
        self.assertIn("first: 1 modules, 1 declarations, 1 issue markers",output.getvalue())
        self.assertIn("last:",output.getvalue())
        verify_inventory_summary(self.out,[json.loads((self.out/(p["name"]+".json")).read_text()) for p in self.pins])

    def test_bad_pin_set_or_destination_never_writes_into_references(self):
        for pins in [[],self.pins+self.pins,[{**self.pins[0],"name":"../escape"}],[{**self.pins[0],"sha":"HEAD"}]]:
            with self.subTest(pins=pins),self.assertRaises(AssertionError):self.plan(pins=pins)
            self.assertFalse(self.out.exists())
        with self.assertRaises(AssertionError): audit.plan(self.refs,self.pins,self.refs/"output","go","rust",auditor=self.auditor)
        self.assertFalse((self.refs/"output").exists())

    def test_rust_conditions_and_unicode_source_paths_are_preserved(self):
        pin=self.repository("rust",{"抹茶.rs":"// fixture\npub struct Tea;\n"})
        def rust(tool,source):
            self.assertEqual(tool,"rust")
            return json.dumps({"file":"抹茶.rs","line":2,"kind":"struct","symbol":"Tea","cfg":"feature=tea"})
        record=json.loads(self.plan(pins=[pin],auditor=rust)["rust.json"])
        self.assertEqual(record["macro_cfg_review"],"pending")
        self.assertEqual(record["declarations"][0]["cfg"],"feature=tea")
        self.assertEqual(record["declarations"][0]["file"],"抹茶.rs")
        self.assertIn("%E6%8A%B9%E8%8C%B6.rs",record["declarations"][0]["source_url"])


if __name__=="__main__":unittest.main()
