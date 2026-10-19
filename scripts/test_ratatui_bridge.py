"""Malformed or truncated cell observations cannot become successful evidence."""
import struct
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from check_ratatui_bridge import check, decode, validate


def cell(symbol="a", width=1):
    text=symbol.encode("utf-8")
    return struct.pack("<HI",width,len(text))+text+b"\0"*14


class BridgeObservationTests(unittest.TestCase):
    def test_literal_decoder_owns_all_bytes_and_wide_continuations(self):
        raw=struct.pack("<HH",3,1)+cell("茶",2)+b"\0\0"+cell()
        rows,digest=decode(raw.hex(),3,1)
        self.assertEqual([(c[0],c[1],c[2]) for c in rows[0]],[(0,2,"茶"),(2,1,"a")])
        self.assertEqual(digest,9190113407378877255)

    def test_truncation_geometry_controls_and_partial_wide_cells_fail_closed(self):
        raw=struct.pack("<HH",1,1)+cell()
        for bad in [raw[:-1],raw+b"\0",struct.pack("<HH",2,1)+cell(),
                    struct.pack("<HH",1,1)+cell("\x1b"),
                    struct.pack("<HH",1,1)+cell("茶",2),
                    struct.pack("<HH",1,1)+cell("",0),
                    raw[:-1]+b"\x80"]:
            with self.subTest(raw=bad),self.assertRaises(ValueError):decode(bad.hex(),1,1)
        for bad in ["AA","0","gg",None,"a"*1_000_002]:
            with self.subTest(text=str(bad)[:10]),self.assertRaises(ValueError):decode(bad,1,1)

    def test_empty_or_unexpected_case_sets_are_rejected(self):
        for records in [{},{"unexpected":{}}]:
            with self.assertRaises(ValueError):validate(records)

    def test_failed_launch_supersedes_old_success(self):
        with tempfile.TemporaryDirectory() as folder:
            dest=Path(folder)
            journal=dest/"outputs-check.json"
            journal.write_text('{"status":"complete"}',encoding="utf-8")
            with patch('check_ratatui_bridge.subprocess.run',side_effect=subprocess.TimeoutExpired('fixture',60)):
                with self.assertRaises(subprocess.TimeoutExpired):check(Path('fixture'),dest)
            self.assertEqual(json.loads(journal.read_text())['status'],'failed')


if __name__ == "__main__":unittest.main()
