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
    @unittest.skipUnless(os.name == "posix", "backslash is a filename character on POSIX")
    def test_rust_backslash_filename_cannot_alias_a_nested_path(self):
        with tempfile.TemporaryDirectory(prefix="matea-audit-identity-") as folder:
            root = Path(folder)
            (root / "a").mkdir()
            for name in ["a/b.rs", "a\\b.rs"]:
                (root / name).write_text("pub const VALUE: u8 = 0;\n", encoding="utf-8")
            result = subprocess.run([str(self.auditor()), folder], capture_output=True,
                                    check=True, timeout=15)
        self.assertEqual([json.loads(row)["file"] for row in result.stdout.splitlines()],
                         ["a/b.rs", "a\\b.rs"])

    @unittest.skipUnless(sys.platform.startswith("linux"), "non-UTF8 root names require a supporting filesystem")
    def test_rust_root_argument_preserves_os_bytes_without_decoding_panics(self):
        with tempfile.TemporaryDirectory(prefix="matea-audit-root-") as folder:
            root = os.fsencode(folder) + b"/\xff-root"
            os.mkdir(root)
            with open(root + b"/lib.rs", "wb") as file:
                file.write(b"pub const LOCAL: u8 = 0;\n")
            result = subprocess.run([os.fsencode(self.auditor()), root], capture_output=True,
                                    check=True, timeout=15)
        self.assertEqual([(r["file"], r["symbol"]) for r in map(json.loads, result.stdout.splitlines())],
                         [("lib.rs", "LOCAL")])

    def test_rust_cached_file_types_preserve_depth_first_filename_order(self):
        files = {"z.rs": "Z", "a/second.rs": "SECOND", "抹茶.rs": "UNICODE",
                 "a/inner/deep.rs": "DEEP", "A.rs": "UPPER", "a/first.rs": "FIRST"}
        with tempfile.TemporaryDirectory(prefix="matea-audit-order-") as folder:
            for name, symbol in files.items():
                path = Path(folder, name)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"pub const {symbol}: u8 = 0;\n", encoding="utf-8")
            result = subprocess.run([str(self.auditor()), folder], capture_output=True,
                                    text=True, encoding="utf-8", check=True, timeout=15)
        self.assertEqual([(row["file"], row["symbol"]) for row in map(json.loads, result.stdout.splitlines())],
                         [("A.rs", "UPPER"), ("a/first.rs", "FIRST"), ("a/inner/deep.rs", "DEEP"),
                          ("a/second.rs", "SECOND"), ("z.rs", "Z"), ("抹茶.rs", "UNICODE")])

    def test_rust_cfg_spelling_preserves_inner_nested_multiple_path_and_value_forms(self):
        text='''#![cfg(any(unix, windows))]
#[cfg(all(feature="tea",not(windows)))]
#[cfg_attr(feature="other", repr(C))]
pub struct Tea { #[cfg(target_pointer_width="64")] pub cups: u64 }
#[cfg]
pub const PATH: u8=0;
#[cfg="literal"]
pub const VALUE:u8=0;
pub mod nested { #![cfg_attr(unix, allow(unused))] pub const C:u8=0; }
#[cfg[unix]] pub const BRACKET:u8=0;
#[cfg{unix}] pub const BRACE:u8=0;
'''
        with tempfile.TemporaryDirectory() as folder:
            Path(folder,'fixture.rs').write_text(text,encoding='utf-8')
            r=subprocess.run([str(self.auditor()),folder],capture_output=True,check=True,timeout=15)
        actual={row['symbol']:row['cfg'] for row in map(json.loads,r.stdout.splitlines())}
        base='# ! [cfg (any (unix , windows))]'
        tea=base+' # [cfg (all (feature = "tea" , not (windows)))] # [cfg_attr (feature = "other" , repr (C))]'
        self.assertEqual(actual,{'Tea':tea,'Tea.cups':tea+' # [cfg (target_pointer_width = "64")]','PATH':base+' # [cfg]','VALUE':base+' # [cfg = "literal"]','nested':base+' # ! [cfg_attr (unix , allow (unused))]','nested::C':base+' # ! [cfg_attr (unix , allow (unused))]','BRACKET':base+' # [cfg [unix]]','BRACE':base+' # [cfg { unix }]'})

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
        environment = go_environment()
        environment["MATEA_GO_AUDIT_PHASE"] = "outputs"
        environment.pop("MATEA_AUDIT_GO_MANIFEST", None)
        result = subprocess.run(["go", "test", "-v", "tools/audit-go/main.go", "tools/audit-go/main_test.go",
                                 "tools/audit-go/measurement_test.go"],
                                cwd=ROOT, env=environment, capture_output=True, text=True,
                                encoding="utf-8", timeout=90)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        actual = {row["case"]: row for row in
                  (json.loads(line) for line in result.stdout.splitlines() if line.startswith("{"))}
        expected = {}
        for count in [1000, 10000, 100000]:
            for structured in [False, True]:
                def encoded_rows():
                    if structured:
                        yield {"file": "fixture.go", "line": 2, "package": "fixture", "kind": "type", "symbol": "Group"}
                    for index in range(count):
                        row = {"file": "fixture.go", "line": index + (3 if structured else 2),
                               "package": "fixture", "kind": "field" if structured else "const",
                               "symbol": f"Group.F_{index}" if structured else f"ITEM_{index}"}
                        if structured:
                            row["signature"] = "int"
                        yield row
                digest, length = 0xcbf29ce484222325, 0
                for row in encoded_rows():
                    data = (json.dumps(row, separators=(",", ":")) + "\n").encode("utf-8")
                    length += len(data)
                    for byte in data:
                        digest = ((digest ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
                name = f"go-audit-parser/{'struct' if structured else 'flat'}_{count}"
                expected[name] = f"rows={count + int(structured)};bytes={length};fnv64={digest:016x}"
                self.assertEqual(actual[name]["record_kind"], "probe-output")
                self.assertEqual(actual[name]["first_return_value"], digest)
        self.assertEqual({name: row["output"] for name, row in actual.items()}, expected)

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
