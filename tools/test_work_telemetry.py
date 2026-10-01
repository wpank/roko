#!/usr/bin/env python3
"""Tests for tools/work_telemetry.py: the rollup's metrics and coverage on a fixture, and the manifest.

Run: python3 tools/test_work_telemetry.py
"""

import contextlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402
import work_telemetry  # noqa: E402

ORIGINAL_REPO = work.REPO
REAL = Path(__file__).resolve().parents[1]


def item(iid, title, size):
    return "\n".join([
        "+++", f'id = "{iid}"', 'kind = "gap"', f'title = "{title}"', 'status = "done"', 'triage = "verified"',
        'severity = "p2"', f'size = "{size}"', 'subsystem = ["x"]', "created = 2026-09-01", "updated = 2026-09-30",
        "last_verified = 2026-09-30", 'source = "test"', 'lane = "tracker"', "anchors = []",
        'links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }', "",
        "[closed]", "at = 2026-09-30", 'at_ts = "2026-09-30T12:30:00Z"', 'executor = "claude-agent"', 'via = "work-batch"',
        f'size = "{size}"', "forced = false", 'evidence = "done in test"', "+++", "", "## Problem", "", "Something.", ""])


def calls(item, model, first_ts, **tok):
    return {"schema": "roko.work_harvest/1", "row": "calls", "date": first_ts[:10], "session": "s", "agent": None,
            "branch": f"work/{item}" if item else "chore/x", "item": item, "join": "branch" if item else "none",
            "model": model, "speed": None, "entrypoint": "cli", "origin": "operator", "sidechain": False, "calls": 1,
            "input_tokens": tok.get("input", 0), "output_tokens": tok.get("output", 0), "cache_read_tokens": 0,
            "cache_write_5m_tokens": 0, "cache_write_1h_tokens": 0, "web_search_requests": 0, "web_fetch_requests": 0,
            "first_ts": first_ts, "last_ts": first_ts}


class TelemetryTest(unittest.TestCase):
    """Two items: gap-aaaaaa was claimed, released over a conflict, claimed again and merged with one conflicted
    file; gap-bbbbbb was claimed once and merged clean. Both have harvested calls."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.com")
        self.git("config", "user.name", "t")
        (self.root / "work" / "items").mkdir(parents=True)
        (self.root / "work" / "telemetry").mkdir()
        shutil.copy(REAL / "work" / "telemetry" / "DEFINITIONS.md", self.root / "work" / "telemetry")
        (self.root / "config" / "prices").mkdir(parents=True)
        shutil.copy(REAL / "config" / "prices" / "2026-09-28.toml", self.root / "config" / "prices")
        (self.root / "work" / "items" / "gap-aaaaaa-x.md").write_text(item("gap-aaaaaa", "A", "S"))
        (self.root / "work" / "items" / "gap-bbbbbb-x.md").write_text(item("gap-bbbbbb", "B", "M"))
        self.commit("items")
        self.merges = {iid: self.merge_branch(iid, conflicts) for iid, conflicts in (("gap-aaaaaa", 1), ("gap-bbbbbb", 0))}
        ev = lambda event, iid, ts, **f: work.event_row(  # noqa: E731
            event, iid, session="s1", ts=ts, executor="claude-agent", via="work-batch", branch=f"work/{iid}", concurrency=1, **f)
        rows = [
            ev("claim", "gap-aaaaaa", "2026-09-30T08:00:00Z", size="S"),
            ev("release", "gap-aaaaaa", "2026-09-30T09:00:00Z", reason="conflict"),
            ev("claim", "gap-aaaaaa", "2026-09-30T10:00:00Z", size="S"),
            ev("merged", "gap-aaaaaa", "2026-09-30T12:00:00Z", merge_sha=self.merges["gap-aaaaaa"][:9], conflicts=1),
            ev("post-verify", "gap-aaaaaa", "2026-09-30T12:05:00Z", rc=0),
            ev("claim", "gap-bbbbbb", "2026-09-30T08:00:00Z", size="M"),
            ev("merged", "gap-bbbbbb", "2026-09-30T14:00:00Z", merge_sha=self.merges["gap-bbbbbb"][:9], conflicts=0),
            ev("post-verify", "gap-bbbbbb", "2026-09-30T14:05:00Z", rc=0),
        ]
        self.events = self.root / "work" / "telemetry" / "events" / "s1.jsonl"
        self.events.parent.mkdir()
        self.events.write_text("".join(json.dumps(r) + "\n" for r in rows))
        # claude-opus-5-5: $4 in, $20 out per 1M; claude-sonnet-5: $2 in; claude-haiku-4-5: $1 in.
        harvest = [
            calls("gap-aaaaaa", "claude-opus-5-5", "2026-09-30T08:30:00.250Z", input=1_000_000, output=100_000),  # $6.00
            calls("gap-aaaaaa", "claude-opus-5-5", "2026-09-30T13:00:00Z", input=1_000_000),  # after the merge: left out
            calls("gap-bbbbbb", "claude-sonnet-5", "2026-09-30T09:00:00Z", input=2_000_000),  # $4.00
            calls("gap-bbbbbb", "mystery-model", "2026-09-30T09:30:00Z", input=500),  # not in the snapshot: unpriced
            calls(None, "claude-haiku-4-5-20251001", "2026-09-30T07:00:00Z", input=1_000_000),  # overhead, $1.00
        ]
        self.harvest = self.root / "work" / "telemetry" / "harvest" / "2026-09-30.jsonl"
        self.harvest.parent.mkdir()
        self.harvest.write_text("".join(json.dumps(r) + "\n" for r in harvest))
        work.set_repo(self.root)

    def tearDown(self):
        work.set_repo(ORIGINAL_REPO)
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True, text=True).stdout

    def commit(self, msg):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", msg)

    def merge_branch(self, iid, conflicts):
        """A worker's branch with one commit, merged the way the work skills merge it."""
        self.git("switch", "-q", "-c", f"work/{iid}")
        (self.root / f"{iid}.txt").write_text("work\n")
        self.commit(f"do {iid}\n\nWork-Item: {iid}\nExecutor: claude-agent")
        self.git("switch", "-q", "main")
        msg = self.root / ".git" / f"work-merge-{iid}.msg"
        msg.write_text(f"merge: {iid}\n\nWork-Item: {iid}\nExecutor: claude-agent\nConflicts: {conflicts}\n")
        self.git("merge", "-q", "--no-ff", f"work/{iid}", "-F", str(msg))
        return self.git("rev-parse", "HEAD").strip()

    def test_rollup_reports_cost_per_merged_item_with_coverage(self):
        r = work_telemetry.build(work_telemetry.dt.date(2026, 10, 1))
        a = r["all"]
        self.assertEqual(a["merged_items"], 2)
        self.assertEqual((a["attempts"]["distribution"], a["attempts"]["mean"]), ({"1": 1, "2": 1, "3+": 0}, 1.5))
        # gap-aaaaaa took two attempts, so only gap-bbbbbb merged first try.
        self.assertEqual(a["first_try_merge"]["num"], 1)
        self.assertEqual(a["first_try_merge"]["den"], 2)
        self.assertEqual((a["conflict_rate"]["num"], a["conflict_rate"]["den"], a["conflict_rate"]["abandoned_over_conflict"]),
                         (1, 2, 1))
        self.assertEqual((a["post_merge_verify_failure"]["num"], a["post_merge_verify_failure"]["den"]), (0, 2))
        cost = a["cost_per_merged_item_usd"]
        self.assertEqual((cost["median"], cost["p90"], cost["total"]), (5.0, 6.0, 10.0))
        self.assertEqual(cost["label"], "API-equivalent (subscription)")
        self.assertGreater(cost["unpriced_token_share"], 0)
        self.assertEqual({m["item"]: m["cost_usd"] for m in r["merged"]}, {"gap-aaaaaa": 6.0, "gap-bbbbbb": 4.0})
        self.assertEqual((r["overhead"]["total_usd"], r["overhead"]["per_merged_item_usd"]), (1.0, 0.5))
        # Every input is present, so every metric covers both items; neither item's 14-day window has ended.
        for k in ("attempts", "first_try_merge", "claim_to_merge_hours", "conflict_rate", "cost_per_merged_item_usd",
                  "unassisted_merge_share"):
            self.assertEqual((a[k]["coverage"]["k"], a[k]["coverage"]["n"]), (2, 2), k)
        self.assertEqual((a["escape"]["pending"], a["escape"]["den"]), (2, 0))
        self.assertEqual((a["unassisted_merge_share"]["num"], a["unassisted_merge_share"]["den"]), (2, 2))
        # From the first claim, across every attempt: A 08:00→12:00, B 08:00→14:00.
        self.assertEqual((a["claim_to_merge_hours"]["median"], a["claim_to_merge_hours"]["p90"]), (5.0, 6.0))
        self.assertEqual(sorted(r["tables"]["kind_size"]), ["gap × M", "gap × S"])
        with contextlib.redirect_stdout(io.StringIO()):
            work_telemetry.cmd_rollup(type("A", (), {"today": "2026-10-01"})())
        text = (self.root / "work" / "telemetry" / "ROLLUP.md").read_text()
        self.assertTrue(text.startswith("# Development record: rollup\n\nNot for workers"))
        self.assertIn(work_telemetry.sha256((self.root / "work" / "telemetry" / "DEFINITIONS.md").read_bytes()), text)
        self.assertIn("| First-try merges | 1/2 (50%) | 2 of 2 merged items (100%) |", text)

    def test_backfill_rows_are_kept_apart(self):
        row = work.event_row("closed", "gap-aaaaaa", session="backfill", ts="2026-09-29T10:00:00Z", executor="human",
                             source="backfill")
        (self.events.parent / "backfill.jsonl").write_text(json.dumps(row) + "\n")
        r = work_telemetry.build(work_telemetry.dt.date(2026, 10, 1))
        self.assertEqual(r["backfill"]["closed_by_executor"], {"human": 1})
        self.assertEqual(r["all"]["merged_items"], 2)
        self.assertEqual(r["all"]["attempts"]["distribution"], {"1": 1, "2": 1, "3+": 0})

    def test_manifest_changes_when_an_input_row_changes(self):
        before = work_telemetry.manifest("2026-10-01")
        paths = {f["path"] for f in before["files"]}
        self.assertTrue({"work/telemetry/events/s1.jsonl", "work/telemetry/harvest/2026-09-30.jsonl",
                         "work/telemetry/DEFINITIONS.md", "config/prices/2026-09-28.toml"} <= paths)
        self.assertEqual({f["path"]: f["rows"] for f in before["files"]}["work/telemetry/events/s1.jsonl"], 8)
        for path, edit in ((self.events, lambda s: s.replace('"rc": 0', '"rc": 1', 1)),
                           (self.harvest, lambda s: s.replace("1000000", "1000001", 1))):
            old = path.read_text()
            path.write_text(edit(old))
            self.assertNotEqual(work_telemetry.manifest("2026-10-01"), before, path.name)
            path.write_text(old)
        self.assertEqual(work_telemetry.manifest("2026-10-01"), before)

    def test_a_dated_model_id_is_priced_as_its_base_id(self):
        prices = work_telemetry.load_prices()
        self.assertEqual(work_telemetry.base_model("claude-haiku-4-5-20251001", prices), "claude-haiku-4-5")
        self.assertEqual(work_telemetry.base_model("claude-opus-5-5[1m]", prices), "claude-opus-5-5")
        self.assertIsNone(work_telemetry.base_model("mystery-model", prices))


if __name__ == "__main__":
    unittest.main(verbosity=1)
