"""Execute both source-inventory CLIs against independent finite fixtures."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def go_environment():
    environment = dict(os.environ)
    environment.setdefault("GOCACHE", str(Path(tempfile.gettempdir()) / "matea-quality-go-cache"))
    return environment


class SourceAuditorTests(unittest.TestCase):
    def auditor(self):
        target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
        return target / "debug" / ("matea-api-audit.exe" if os.name == "nt" else "matea-api-audit")

    def test_rust_tree_skips_external_links_and_cycles_and_rejects_linked_root(self):
        def link(source, destination):
            if os.name == "nt":
                subprocess.run(["cmd", "/c", "mklink", "/J", str(destination), str(source)],
                               check=True, capture_output=True, timeout=15)
            else:
                destination.symlink_to(source, target_is_directory=True)
        with tempfile.TemporaryDirectory(prefix="matea-audit-links-") as folder:
            base=Path(folder);source=base/"source";source.mkdir();outside=base/"outside";outside.mkdir()
            (source/"lib.rs").write_text("pub const LOCAL: usize = 1;\n",encoding="utf-8")
            (outside/"outside.rs").write_text("pub const OUTSIDE: usize = 2;\n",encoding="utf-8")
            link(outside,source/"escape");link(source,source/"cycle");link(source,base/"alias")
            result=subprocess.run([str(self.auditor()),str(source)],capture_output=True,text=True,encoding="utf-8",timeout=15,check=True)
            self.assertEqual([json.loads(l)['symbol'] for l in result.stdout.splitlines()],["LOCAL"])
            for path in [base/"alias",source/"lib.rs",base/"missing"]:
                bad=subprocess.run([str(self.auditor()),str(path)],capture_output=True,timeout=15)
                self.assertNotEqual(bad.returncode,0)
                self.assertEqual(bad.stdout,b"")

    @unittest.skipUnless(sys.platform.startswith("linux"), "arbitrary filename bytes require a supporting Linux filesystem; APFS rejects this fixture")
    def test_rust_non_utf8_paths_fail_instead_of_colliding_after_lossy_conversion(self):
        with tempfile.TemporaryDirectory(prefix="matea-audit-path-") as folder:
            for byte in [b"\xff",b"\xfe"]:
                with open(os.fsencode(folder)+b"/"+byte+b".rs","wb") as file:
                    file.write(b"pub const VALUE: usize = 0;\n")
            result=subprocess.run([str(self.auditor()),folder],capture_output=True,timeout=15)
            self.assertNotEqual(result.returncode,0)
            self.assertIn(b"not UTF-8",result.stderr)
            self.assertEqual(result.stdout,b"")

    @unittest.skipUnless(os.name == "posix", "FIFO source entries are a Unix filesystem contract")
    def test_rust_fifo_with_source_suffix_is_not_opened(self):
        with tempfile.TemporaryDirectory(prefix="matea-audit-fifo-") as folder:
            source=Path(folder)
            (source/"lib.rs").write_text("pub const LOCAL: usize = 1;\n",encoding="utf-8")
            os.mkfifo(source/"blocked.rs")
            result=subprocess.run([str(self.auditor()),folder],capture_output=True,text=True,encoding="utf-8",timeout=15,check=True)
            self.assertEqual([json.loads(l)['symbol'] for l in result.stdout.splitlines()],["LOCAL"])

    def test_go_parser_and_writer_failures(self):
        result = subprocess.run(["go", "test", "tools/audit-go/main.go", "tools/audit-go/main_test.go"],
                                cwd=ROOT, env=go_environment(), capture_output=True, text=True,
                                encoding="utf-8", timeout=90)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_rust_nested_platform_and_feature_conditions_are_never_dropped(self):
        target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
        binary = target / "debug" / ("matea-api-audit.exe" if os.name == "nt" else "matea-api-audit")
        self.assertTrue(binary.exists(), "build matea-api-audit before running this suite")
        source = '''#[cfg(feature="parent")]
pub mod nested {
pub struct S { #[cfg(unix)] pub field: u8, private: u8 }
pub enum E { #[cfg(windows)] Win { #[cfg(feature="payload")] value: u8 } }
impl S { #[cfg(feature="method")] pub fn call(&self) {} }
pub trait T { #[cfg(feature="member")] fn run(); }
#[cfg(feature="constant")] pub const C: u8 = 0;
#[cfg(feature="static")] pub static V: u8 = 0;
#[cfg(feature="alias")] pub type A = u8;
#[cfg(feature="export")] pub use self::S as Other;
}
'''
        with tempfile.TemporaryDirectory() as folder:
            Path(folder, "抹茶.rs").write_text(source, encoding="utf-8")
            result = subprocess.run([str(binary), folder], capture_output=True, text=True, encoding="utf-8", check=True, timeout=30)
        entries = [json.loads(line) for line in result.stdout.splitlines()]
        expected = {"nested::S.field": "unix", "nested::E::Win": "windows", "nested::E::Win.value": "payload", "nested::<S>::call": "method", "nested::T::run": "member", "nested::C": "constant", "nested::V": "static", "nested::A": "alias", "self :: S as Other": "export"}
        for symbol, condition in expected.items():
            entry = next(item for item in entries if item["symbol"] == symbol)
            self.assertIn("parent", entry["cfg"], symbol)
            self.assertIn(condition, entry["cfg"], symbol)
            self.assertEqual(entry["file"], "抹茶.rs")
            if symbol == "nested::E::Win.value":
                self.assertIn("windows", entry["cfg"])
        self.assertNotIn("nested::S.private", [e["symbol"] for e in entries])

    def test_go_platform_exports_embedded_types_and_benchmarks(self):
        self.assertIsNotNone(shutil.which("go"), "Go is required to execute the source-auditor fixture")
        with tempfile.TemporaryDirectory() as folder:
            Path(folder, "module_windows.go").write_text('''//go:build windows && amd64
package fixture
type Public struct { Field int; hidden int; Embedded }
type Reader interface { Read() error; Embedded }
func (p *Public) Exported() {}
func hidden() {}
const Answer = 42
''', encoding="utf-8")
            Path(folder, "module_test.go").write_text('''package fixture
func BenchmarkVisible() {}
func TestHidden() {}
''', encoding="utf-8")
            result = subprocess.run(["go", "run", str(ROOT / "tools/audit-go/main.go"), folder], env=go_environment(), capture_output=True, text=True, encoding="utf-8", check=True, timeout=90)
        entries = [json.loads(line) for line in result.stdout.splitlines()]
        names = {e["symbol"] for e in entries}
        self.assertTrue({"Public", "Public.Field", "Public.Embedded", "Reader.Read", "Reader.Embedded", "*Public.Exported", "Answer", "BenchmarkVisible"}.issubset(names))
        self.assertFalse({"hidden", "Public.hidden", "TestHidden"} & names)
        self.assertTrue(all(e.get("build") == ["windows && amd64"] for e in entries if e["file"] == "module_windows.go"))


if __name__ == "__main__":
    unittest.main()
