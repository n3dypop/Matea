"""Execute both source-inventory CLIs against independent finite fixtures."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class SourceAuditorTests(unittest.TestCase):
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

    @unittest.skipUnless(shutil.which("go"), "Go CLI unavailable; source-auditor execution remains pending")
    def test_go_platform_exports_embedded_types_and_benchmarks(self):
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
            result = subprocess.run(["go", "run", str(ROOT / "tools/audit-go/main.go"), folder], capture_output=True, text=True, encoding="utf-8", check=True, timeout=90)
        entries = [json.loads(line) for line in result.stdout.splitlines()]
        names = {e["symbol"] for e in entries}
        self.assertTrue({"Public", "Public.Field", "Public.Embedded", "Reader.Read", "Reader.Embedded", "*Public.Exported", "Answer", "BenchmarkVisible"}.issubset(names))
        self.assertFalse({"hidden", "Public.hidden", "TestHidden"} & names)
        self.assertTrue(all(e.get("build") == ["windows && amd64"] for e in entries if e["file"] == "module_windows.go"))


if __name__ == "__main__":
    unittest.main()
