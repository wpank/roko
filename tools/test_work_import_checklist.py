#!/usr/bin/env python3
"""Tests for tools/work_import_checklist.py: which checklist rows are imported or skipped, and what gets written.

Run: python3 tools/test_work_import_checklist.py
"""

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402
import work_import_checklist as imp  # noqa: E402

ORIGINAL_REPO, ORIGINAL_INDEX = work.REPO, work.index
GOALS = ["cybernetic", "golden-path", "visibility", "release", "proof", "truth", "whitepaper"]
EPICS = ["spec-6ac537", "spec-e57870", "spec-567e52", "spec-b7303f", "spec-f8d196"]


def item(iid, kind, title):
    return "\n".join([
        "+++", f'id = "{iid}"', f'kind = "{kind}"', f'title = "{title}"', 'status = "open"', 'triage = "verified"',
        'severity = "p2"', 'subsystem = ["x"]', "created = 2026-09-01", "updated = 2026-09-01",
        "last_verified = 2026-09-01", 'source = "test"', "anchors = []",
        'links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }', "+++", "",
        "## Problem", "", "Something.", ""])


def row(rid, title, *, status="todo", deps=(), lane="L2-rust-lib", files=("src/a.rs",), verify=""):
    spec = rid.split(".")[0]
    return {"id": rid, "spec": spec, "title": title, "status": status, "deps": list(deps), "deps_raw": "", "lane": lane,
            "milestone": "MS1", "effort": "S", "hot": "no", "files": "src/a.rs", "files_owned": list(files),
            "acceptance": f"{title} works.", "verify": verify, "notes": "", "parallel_notes": "", "risk": ""}


class ImportTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        (self.root / "work" / "items").mkdir(parents=True)
        (self.root / "work" / "goals.toml").write_text("".join(f'[[goal]]\nkey = "{g}"\ntitle = "{g}"\n\n' for g in GOALS))
        lanes = "".join(f'[lane.{lane}]\npaths = ["src/**"]\n\n' for lane in ("rust-cold", "rust-hot", "bench", "frontend", "paper"))
        (self.root / "work" / "lanes.toml").write_text('milestones = ["MS0", "MS1"]\n\n' + lanes)
        (self.root / "src").mkdir()
        (self.root / "src" / "a.rs").write_text("fn a() {}\n")
        for epic in EPICS:
            self.write(epic, item(epic, "spec", f"Epic {epic}"))
        self.write("gap-111111", item("gap-111111", "gap", "Covers part of the work"))
        self.write("gap-222222", item("gap-222222", "gap", "Already does the second row (S02.T2)"))
        rows = [row("S02.T1", "First row"), row("S02.T2", "Second row"), row("S02.T3", "Third row"),
                row("S02.T4", "Fourth row"), row("S02.T5", "Fifth row", deps=["S02.T1"], files=("src/new/**",)),
                row("S01.P0-0", "Out of scope"), row("S10.T1", "Showcase contracts", lane="L4-frontend"),
                row("S03.T9", "Already verified", status="verified")]
        self.checklist = self.root / "checklist.json"
        self.checklist.write_text(json.dumps({"items": rows}))
        self.crosswalk = self.root / "crosswalk.md"
        self.crosswalk.write_text("| Row | Items | Status | Lane |\n|---|---|---|---|\n"
                                  "| S02.T3 third | gap-111111 | TRACKED | x |\n"
                                  "| S02.T4 fourth | gap-111111 (part) | PARTIAL | x |\n")
        work.set_repo(self.root)

    def tearDown(self):
        work.set_repo(ORIGINAL_REPO)
        work.index = ORIGINAL_INDEX
        self.tmp.cleanup()

    def write(self, iid, text):
        (self.root / "work" / "items" / f"{iid}-x.md").write_text(text)

    def run_import(self, *extra):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = imp.main(["--checklist", str(self.checklist), "--manifest", str(self.root / "none.json"),
                             "--crosswalk", str(self.crosswalk), *extra])
        work.index = ORIGINAL_INDEX
        return code, out.getvalue()

    def files(self):
        return sorted(p.name for p in (self.root / "work" / "items").iterdir())

    def test_import_skips_items_already_covered(self):
        before = self.files()
        code, out = self.run_import("--dry-run")
        self.assertEqual(code, 0, out)
        lines = {ln.split()[1]: ln for ln in out.splitlines() if ln.startswith(("import ", "skip "))}
        self.assertIn("named by gap-222222", lines["S02.T2"])
        self.assertIn("crosswalk TRACKED: gap-111111", lines["S02.T3"])
        self.assertTrue(lines["S02.T1"].startswith("import"))
        self.assertIn("related gap-111111", lines["S02.T4"])  # PARTIAL: imported, related to the partial cover
        self.assertTrue(lines["S10.T1"].startswith("import") and "visibility/-" in lines["S10.T1"])
        self.assertNotIn("S01.P0-0", lines)  # out of scope
        self.assertNotIn("S03.T9", lines)  # not open
        self.assertEqual(self.files(), before, "a dry run writes nothing")

    def test_import_writes_unverified_items_that_check_accepts(self):
        code, out = self.run_import()
        self.assertEqual(code, 0, out)
        items, errs = work.load("work")
        self.assertEqual(errs, [])
        self.assertEqual(work.validate(items), [])
        by_source = {it["source"].split("#")[-1]: it for it in items if it["source"].startswith(imp.CHECKLIST)}
        self.assertEqual(sorted(by_source), ["S02.T1", "S02.T4", "S02.T5", "S10.T1"])
        first, fifth = by_source["S02.T1"], by_source["S02.T5"]
        self.assertEqual((first["triage"], first["goal"], first["parent"], first["lane"], first["milestone"], first["size"]),
                         ("unverified", "cybernetic", "spec-6ac537", "rust-cold", "MS1", "S"))
        self.assertEqual(fifth["links"]["depends_on"], [first["id"]])  # a dependency imported in the same run
        self.assertEqual(fifth["anchors"], ["src"])  # src/new/** does not exist yet: its nearest existing parent
        self.assertEqual(by_source["S10.T1"]["lane"], "frontend")
        self.assertIn("First row works.", first["_body"])
        # A second run finds everything already imported (named by its own source) and writes nothing new.
        before = self.files()
        code, out = self.run_import()
        self.assertEqual((code, self.files()), (0, before), out)
        self.assertIn("would import 0" if "would" in out else "imported 0", out)


if __name__ == "__main__":
    unittest.main(verbosity=1)
