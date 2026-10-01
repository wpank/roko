#!/usr/bin/env python3
"""Tests for tools/work_backfill.py: executors mapped from [closed].by and merge trailers, lane starts from the
reflog, and every row labelled as a backfill.

Run: python3 tools/test_work_backfill.py
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402
import work_backfill  # noqa: E402

ORIGINAL_REPO = work.REPO

# (item, [closed].by, expected (executor, assist, reason))
BY_FORMS = [
    ("gap-a00001", "plan:portal-programme/03c-backend-local-access#T10", ("roko-plan", None, None)),
    ("gap-a00002", "session roko-b6", ("claude-session", None, None)),
    ("gap-a00003", "triage check 2026-09-28", ("verification-only", None, None)),
    ("gap-a00004", "Will (licence decision); files added by Claude", ("human", "claude", None)),
    ("gap-a00005", "commit trailer", ("unknown", None, "commit-trailer")),
    ("gap-a00006", None, ("unknown", None, "no-by")),
    ("gap-a00007", "work sweep 2026-09-29 (static check against HEAD)", ("verification-only", None, None)),
    ("gap-a00008", "work enrichment 2026-09-29 (static check)", ("verification-only", None, None)),
    ("gap-a00009", "wk-bench-fix1", ("claude-agent", None, None)),
    ("gap-a0000a", "rs-release (roko-b6)", ("claude-session", None, None)),
]


def item(iid, *, status="done", by=None, evidence="done in test"):
    lines = ["+++", f'id = "{iid}"', 'kind = "gap"', f'title = "Item {iid}"', f'status = "{status}"', 'triage = "verified"',
             'severity = "p2"', 'subsystem = ["x"]', "created = 2026-09-01", "updated = 2026-09-01",
             "last_verified = 2026-09-01", 'source = "test"', "anchors = []",
             'links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }']
    if status != "open":
        lines += ["", "[closed]", "at = 2026-09-20"] + ([f"by = {work.tomlstr(by)}"] if by else []) + [f"evidence = {work.tomlstr(evidence)}"]
    return "\n".join(lines) + "\n+++\n\n## Problem\n\nSomething.\n"


class BackfillTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.com")
        self.git("config", "user.name", "t")
        (self.root / "work" / "items").mkdir(parents=True)
        for iid, by, _ in BY_FORMS:
            self.write(iid, item(iid, by=by))
        self.write("gap-b00001", item("gap-b00001", status="open"))
        self.commit("items")
        # A worker's branch for gap-c00001: its commit names the item and its executor, and the coordinator added a
        # fixup of its own before the merge.
        self.git("switch", "-q", "-c", "work/gap-c00001")
        (self.root / "f.txt").write_text("work\n")
        self.commit("do the work\n\nWork-Item: gap-c00001\nExecutor: claude-agent")
        (self.root / "f.txt").write_text("work, fixed\n")
        self.commit("fix it up\n\nWork-Item: gap-c00001\nExecutor: claude-session")
        self.git("switch", "-q", "main")
        self.git("merge", "-q", "--no-ff", "work/gap-c00001", "-m", "merge: gap-c00001 (work/gap-c00001)")
        merge = self.git("rev-parse", "HEAD").strip()
        self.write("gap-c00001", item("gap-c00001", by="coordinator (session 7622b882)", evidence=f"Merged in {merge[:9]}."))
        self.commit("close gap-c00001")
        self.merge = merge
        work.set_repo(self.root)

    def tearDown(self):
        work.set_repo(ORIGINAL_REPO)
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True, text=True).stdout

    def write(self, iid, text):
        (self.root / "work" / "items" / f"{iid}-x.md").write_text(text)

    def commit(self, msg):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", msg)

    def test_backfill_labels_every_row_and_maps_executors(self):
        rows = work_backfill.build(work.load("work")[0])
        self.assertTrue(rows)
        for r in rows:
            self.assertEqual(work.check_event(r), [], r)
            self.assertEqual((r["source"], r["session"]), ("backfill", "backfill"), r)
        closed = {r["item"]: r for r in rows if r["event"] == "closed"}
        self.assertNotIn("gap-b00001", closed)  # an open item has no closed row
        for iid, by, expected in BY_FORMS:
            r = closed[iid]
            self.assertEqual((r["executor"], r.get("assist"), r.get("reason")), expected, by)
            self.assertEqual((r["executor_from"], r["ts_from"], r.get("by")), ("by", "commit", by))
        self.assertEqual(closed["gap-a00001"]["via"], "roko-plan")
        # The coordinator recorded the close; the merged commits name who did the work.
        c = closed["gap-c00001"]
        self.assertEqual((c["executor"], c["assist"], c["executor_from"], c["branch"]),
                         ("claude-agent", "claude-session", "merge-trailers", "work/gap-c00001"))
        self.assertTrue(self.merge.startswith(c["merge_sha"]))
        lanes = [r for r in rows if r["event"] == "lane-start"]
        self.assertEqual([(r["branch"], r["item"], r["ts_from"]) for r in lanes],
                         [("work/gap-c00001", "gap-c00001", "branch-reflog")])
        self.assertEqual(self.git("status", "--porcelain"), "")  # no item file was edited

    def test_the_cli_writes_one_row_per_line(self):
        out = self.root / "backfill.jsonl"
        env = {**os.environ, "WORK_REPO": str(self.root)}
        subprocess.run([sys.executable, str(Path(work_backfill.__file__)), "--out", str(out)], cwd=self.root, env=env,
                       check=True, capture_output=True, text=True)
        rows = [json.loads(ln) for ln in out.read_text().splitlines()]
        self.assertEqual(len([r for r in rows if r["event"] == "closed"]), len(BY_FORMS) + 1)
        self.assertEqual([r["ts"] for r in rows], sorted(r["ts"] for r in rows))

    def test_lane_item_tolerates_a_suffixed_branch(self):
        ids = {"gap-3506f1", "bug-7e1b6b"}
        self.assertEqual(work_backfill.lane_item("work/gap-3506f1b", ids), "gap-3506f1")
        self.assertEqual(work_backfill.lane_item("work/bug-7e1b6b", ids), "bug-7e1b6b")
        self.assertIsNone(work_backfill.lane_item("work/bench-integrate-1", ids))
        self.assertIsNone(work_backfill.lane_item("fix/serve-polish", ids))


if __name__ == "__main__":
    unittest.main(verbosity=1)
