"""Acceptance tests: `work.py close --spend-usd` records what a closed item cost."""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402
from test_work import RepoTest  # noqa: E402


class TestCloseSpend(RepoTest):
    def show(self, iid):
        return json.loads(self.run_work("show", iid, "--json").stdout)

    def item_text(self, iid):
        files = [f for f in (self.root / "work").rglob(f"{iid}-*.md")]
        self.assertEqual(len(files), 1, files)
        return files[0].read_text()

    def test_close_writes_spend_usd_as_a_number(self):
        self.run_work("close", "gap-cccccc", "--evidence", "done in test", "--spend-usd", "1.25")
        self.assertIn("spend_usd = 1.25", self.item_text("gap-cccccc"))
        closed = self.show("gap-cccccc")["closed"]
        self.assertIsInstance(closed["spend_usd"], (int, float))
        self.assertEqual(closed["spend_usd"], 1.25)
        self.run_work("close", "bug-bbbbbb", "--evidence", "done in test", "--spend-usd", "0")
        self.assertEqual(self.show("bug-bbbbbb")["closed"]["spend_usd"], 0)

    def test_close_without_spend_usd_has_no_key(self):
        self.run_work("close", "gap-cccccc", "--evidence", "done in test")
        closed = self.show("gap-cccccc")["closed"]
        self.assertIsNotNone(closed)
        self.assertNotIn("spend_usd", closed)
        self.assertNotIn("spend_usd", self.item_text("gap-cccccc"))

    def test_negative_spend_is_rejected_and_item_stays_open(self):
        r = self.run_work("close", "gap-cccccc", "--evidence", "x", "--spend-usd", "-1", check=False)
        self.assertNotEqual(r.returncode, 0)
        shown = self.show("gap-cccccc")
        self.assertEqual(shown["status"], "open")
        self.assertIsNone(shown["closed"])

    def test_non_numeric_spend_is_rejected_and_item_stays_open(self):
        r = self.run_work("close", "gap-cccccc", "--evidence", "x", "--spend-usd", "abc", check=False)
        self.assertNotEqual(r.returncode, 0)
        shown = self.show("gap-cccccc")
        self.assertEqual(shown["status"], "open")
        self.assertIsNone(shown["closed"])

    def test_check_closed_reports_bad_spend(self):
        for bad in (-0.5, "1.0", True, math.nan):
            errs = work.check_closed({"spend_usd": bad})
            self.assertTrue(errs, bad)
            self.assertTrue(any("spend_usd" in e for e in errs), (bad, errs))
        for good in (0, 2.5, 3):
            self.assertEqual(work.check_closed({"spend_usd": good}), [], good)
