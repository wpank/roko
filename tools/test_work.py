#!/usr/bin/env python3
"""Tests for tools/work.py: verify-command parsing and lint, picking non-conflicting work, claims, drift, sync and the
event log.

Run: python3 tools/test_work.py
"""

import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import work  # noqa: E402

ORIGINAL_REPO = work.REPO


def item(iid, title, *, severity="p2", goal="core", anchors=(), verify=(), extra="", status="open"):
    kind = {"bug": "bug", "gap": "gap"}[iid.split("-")[0]]
    lines = [
        "+++", f'id = "{iid}"', f'kind = "{kind}"', f'title = "{title}"', f'status = "{status}"', 'triage = "verified"',
        f'severity = "{severity}"', f'goal = "{goal}"', 'subsystem = ["x"]', "created = 2026-09-01", "updated = 2026-09-01",
        "last_verified = 2026-09-01", 'source = "test"', "anchors = [" + ", ".join(f'"{a}"' for a in anchors) + "]",
        'links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }',
    ]
    if extra:
        lines.append(extra)
    for v in verify:
        lines += ["", "[[verify]]", f"command = {work.tomlstr(v)}"]
    return "\n".join(lines) + "\n+++\n\n## Problem\n\nSomething.\n"


class TestVerifyCommands(unittest.TestCase):
    def test_split_and_respects_quotes(self):
        self.assertEqual(work.split_and("grep -q 'a && b' f && cargo test -p x t"), ["grep -q 'a && b' f", "cargo test -p x t"])

    def test_static_prefix_stops_at_cargo_and_cd(self):
        self.assertEqual(work.static_prefix("grep -q x f && ! grep -q y g && cargo test -p c t"), "grep -q x f && ! grep -q y g")
        self.assertEqual(work.static_prefix("cd apps/portal && npx vitest run a.test.ts"), "")
        self.assertEqual(work.static_prefix("cargo test -p c t"), "")

    def test_lint_flags_head_pipeline_and_whole_suite(self):
        self.assertTrue(any("head" in w for w in work.lint_verify("grep -n 'x' f | head -5")))
        self.assertTrue(any("whole" in w for w in work.lint_verify("cargo test -p roko-acp")))
        self.assertEqual(work.lint_verify("test -f crates/c/tests/t.rs && cargo test -p c --test t"), [])
        self.assertEqual(work.lint_verify("grep -rqw 'fn t' crates/c/ && cargo test -p c --lib t"), [])

    def test_close_keywords(self):
        found = lambda msg: [i for m in work.CLOSE_RE.finditer(msg) for i in work.ID_ANY.findall(m.group(1))]  # noqa: E731
        self.assertEqual(found("fix the thing\n\nCloses: bug-1a2b3c, gap-4d5e6f"), ["bug-1a2b3c", "gap-4d5e6f"])
        self.assertEqual(found("fixes bug-1a2b3c and gap-4d5e6f"), ["bug-1a2b3c", "gap-4d5e6f"])
        self.assertEqual(found("fix(engine): settle verify failures"), [])
        self.assertEqual(found("ci: hold deploys until bug-7eef96 is fixed"), [])

    def test_anchor_parts(self):
        self.assertEqual(work.anchor_parts("crates/x/src/a.rs::Foo::bar"), ("crates/x/src/a.rs", "bar"))
        self.assertEqual(work.anchor_parts("crates/x/src/a.rs:12"), ("crates/x/src/a.rs", None))
        self.assertEqual(work.anchor_parts("roko learn router"), (None, None))

    def test_strip_note_and_cargo_guard(self):
        self.assertEqual(work.strip_note("grep -q x f  (the fix must add a test)"), "grep -q x f")
        self.assertEqual(work.cargo_guard("cargo test -p roko-cli --lib mod::tests::it_works"), "grep -rqw 'fn it_works' crates/roko-cli/")
        self.assertEqual(work.cargo_guard("cargo test -p roko-cli --test parity"), "test -f crates/roko-cli/tests/parity.rs")


class RepoTest(unittest.TestCase):
    """A throwaway git repo with a work/ tree."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.com")
        self.git("config", "user.name", "t")
        (self.root / "work" / "items").mkdir(parents=True)
        (self.root / "work" / "goals.toml").write_text('[[goal]]\nkey = "core"\ntitle = "Core"\nnow = 3\n')
        (self.root / "src").mkdir()
        for f in ("a.rs", "c.rs"):
            (self.root / "src" / f).write_text("fn foo() {}\n")
        self.write("bug-aaaaaa", item("bug-aaaaaa", "A", severity="p1", anchors=["src/a.rs::foo"]))
        self.write("bug-bbbbbb", item("bug-bbbbbb", "B", severity="p2", anchors=["src/a.rs"]))
        self.write("gap-cccccc", item("gap-cccccc", "C", severity="p3", anchors=["src/c.rs"]))
        self.commit("initial")
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

    def items(self):
        items, errs = work.load("work")
        self.assertEqual(errs, [])
        return items

    def run_work(self, *args):
        """Run tools/work.py in the repo as a separate process, the way the skills do."""
        env = {k: v for k, v in os.environ.items() if k != "WORK_SESSION"}
        return subprocess.run([sys.executable, str(Path(work.__file__)), *args], cwd=self.root,
                              env={**env, "WORK_REPO": str(self.root)}, check=True, capture_output=True, text=True)

    def events(self, session):
        f = self.root / "work" / "telemetry" / "events" / f"{session}.jsonl"
        return [json.loads(ln) for ln in f.read_text().splitlines()]

    def add_worktree(self, branch):
        """A linked worktree of the repo on a new branch, removed after the test."""
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        path = Path(tmp.name) / "wt"
        self.git("worktree", "add", "-q", "-b", branch, str(path))
        return path


class TestPicking(RepoTest):
    def test_next_skips_items_that_touch_the_same_files(self):
        picked, skipped = work.pick_next(self.items(), n=3, claims={})
        self.assertEqual([i["id"] for i in picked], ["bug-aaaaaa", "gap-cccccc"])
        self.assertEqual(sum(skipped.values()), 1)

    def test_claimed_items_and_their_files_are_avoided(self):
        subprocess.run([sys.executable, str(Path(work.__file__)), "claim", "bug-aaaaaa", "--by", "t"], cwd=self.root,
                       env={**os.environ, "WORK_REPO": str(self.root)}, check=True, capture_output=True)
        self.assertIn("bug-aaaaaa", work.load_claims())
        picked, _ = work.pick_next(self.items(), n=3)
        self.assertEqual([i["id"] for i in picked], ["gap-cccccc"])

    def test_hold_keeps_an_item_out_of_next(self):
        self.write("bug-aaaaaa", item("bug-aaaaaa", "A", severity="p1", anchors=["src/a.rs::foo"], extra='hold = "not now"'))
        picked, _ = work.pick_next(self.items(), n=3, claims={})
        self.assertNotIn("bug-aaaaaa", [i["id"] for i in picked])
        self.assertEqual(work.validate(self.items()), [])


class TestDriftAndSync(RepoTest):
    def test_drift_sees_changed_and_missing_anchors(self):
        (self.root / "src" / "c.rs").write_text("fn bar() {}\n")
        self.commit("change c")
        (self.root / "src" / "a.rs").write_text("fn renamed() {}\n")
        self.commit("rename foo")
        drift = work.compute_drift(self.items())
        self.assertTrue(drift["gap-cccccc"]["touched"])
        self.assertEqual(drift["bug-aaaaaa"]["gone"], ["src/a.rs::foo"])

    def test_sync_closes_items_named_by_a_commit_trailer(self):
        (self.root / "src" / "c.rs").write_text("fn fixed() {}\n")
        self.commit("make c work\n\nCloses: gap-cccccc")
        closed, conflicts = work.cmd_sync(type("A", (), {"dry_run": False})(), quiet=True)
        self.assertEqual(len(closed), 1)
        self.assertEqual(conflicts, [])
        c = {i["id"]: i for i in self.items()}["gap-cccccc"]
        self.assertEqual(c["status"], "done")
        self.assertTrue(c["closed"]["commit"])
        self.assertIn("## Problem", c["_body"])
        self.assertEqual(work.validate(self.items()), [])

    def test_sync_refuses_when_the_verify_command_fails(self):
        self.write("gap-cccccc", item("gap-cccccc", "C", severity="p3", anchors=["src/c.rs"], verify=["grep -q fixed src/c.rs"]))
        self.commit("add verify")
        self.commit_empty = self.git("commit", "-q", "--allow-empty", "-m", "claims too much\n\nCloses: gap-cccccc")
        closed, conflicts = work.cmd_sync(type("A", (), {"dry_run": False})(), quiet=True)
        self.assertEqual(closed, [])
        self.assertEqual(len(conflicts), 1)

    def test_closing_in_the_main_checkout_prunes_the_claim(self):
        d = work.claims_dir()
        d.mkdir(parents=True)
        (d / "gap-cccccc.json").write_text('{"id": "gap-cccccc", "by": "t", "at": "2026-09-29T10:00:00+02:00"}')
        it = {i["id"]: i for i in self.items()}["gap-cccccc"]
        work.close_item(it, evidence="done in test", commit="abc1234")
        self.assertFalse((d / "gap-cccccc.json").exists())


class TestEvents(RepoTest):
    def test_claim_and_release_append_events_to_the_session_file(self):
        self.run_work("claim", "gap-cccccc", "--by", "t", "--session", "s1", "--branch", "work/gap-cccccc")
        self.run_work("release", "gap-cccccc", "--session", "s1")
        rows = self.events("s1")
        self.assertEqual([r["event"] for r in rows], ["claim", "release"])
        for r in rows:
            self.assertEqual(work.check_event(r), [])
            self.assertEqual((r["item"], r["session"], r["branch"], r["source"]), ("gap-cccccc", "s1", "work/gap-cccccc", "live"))
        # The release is logged while its claim is still live, and then the claim is gone.
        self.assertEqual([r["concurrency"] for r in rows], [1, 1])
        self.assertFalse((work.claims_dir() / "gap-cccccc.json").exists())

    def test_the_session_defaults_to_the_claimant(self):
        self.run_work("claim", "gap-cccccc", "--by", "Batch Orchestrator")
        self.run_work("release", "gap-cccccc")
        self.assertEqual([r["event"] for r in self.events("batch-orchestrator")], ["claim", "release"])

    def test_event_merged_appends_a_row_with_the_merge_sha(self):
        self.run_work("claim", "gap-cccccc", "--by", "t", "--session", "s1", "--branch", "work/gap-cccccc")
        sha = self.git("rev-parse", "HEAD").strip()
        self.run_work("event", "merged", "gap-cccccc", "--merge-sha", sha, "--conflicts", "2", "--session", "s1")
        self.run_work("event", "post-verify", "gap-cccccc", "--rc", "0", "--session", "s1")
        merged, verify = self.events("s1")[1:]
        for r in (merged, verify):
            self.assertEqual(work.check_event(r), [])
        self.assertEqual((merged["event"], merged["conflicts"], merged["branch"]), ("merged", 2, "work/gap-cccccc"))
        self.assertTrue(sha.startswith(merged["merge_sha"]))
        self.assertEqual((verify["event"], verify["rc"]), ("post-verify", 0))

    def test_a_worker_in_a_linked_worktree_logs_no_events(self):
        work.set_repo(self.add_worktree("work/gap-cccccc"))
        self.assertIsNone(work.log_event("claim", "gap-cccccc", session="s1"))
        self.assertFalse((self.root / "work" / "telemetry").exists())


class TestExecutorFields(RepoTest):
    def test_close_copies_claim_fields_into_closed(self):
        self.run_work("claim", "gap-cccccc", "--by", "t", "--session", "s1", "--branch", "work/gap-cccccc",
                      "--executor", "claude-agent", "--via", "work-batch", "--size", "S")
        claim = work.item_claim("gap-cccccc")
        self.assertEqual((claim["executor"], claim["via"], claim["size"]), ("claude-agent", "work-batch", "S"))
        self.assertRegex(claim["claimed_at"], work.TS_RE)
        # A worker closes the item in its own worktree: the claim is read from the main checkout.
        wt = self.add_worktree("work/gap-cccccc")
        env = {k: v for k, v in os.environ.items() if k != "WORK_SESSION"}
        subprocess.run([sys.executable, str(Path(work.__file__)), "close", "gap-cccccc", "--evidence", "done in test",
                        "--commit", "HEAD", "--model", "claude-opus-5-5"], cwd=wt, env={**env, "WORK_REPO": str(wt)},
                       check=True, capture_output=True, text=True)
        work.set_repo(wt)
        closed = {i["id"]: i for i in self.items()}["gap-cccccc"]["closed"]
        self.assertEqual({k: closed.get(k) for k in ("executor", "via", "size", "claimed_at", "model", "forced")},
                         {"executor": "claude-agent", "via": "work-batch", "size": "S", "claimed_at": claim["claimed_at"],
                          "model": "claude-opus-5-5", "forced": False})
        self.assertRegex(closed["at_ts"], work.TS_RE)
        self.assertEqual(work.validate(self.items()), [])
        # The worker's close leaves the claim for the merge, and logs nothing.
        self.assertTrue((self.root / ".roko" / "work-claims" / "gap-cccccc.json").exists())
        self.assertEqual([r["event"] for r in self.events("s1")], ["claim"])

    def test_release_records_its_reason(self):
        self.run_work("claim", "gap-cccccc", "--by", "t", "--session", "s1", "--executor", "claude-agent",
                      "--via", "work-next")
        self.run_work("release", "gap-cccccc", "--reason", "blocked")
        claim, release = self.events("s1")
        self.assertEqual(work.check_event(release), [])
        self.assertEqual((release["event"], release["reason"], release["executor"], release["via"]),
                         ("release", "blocked", "claude-agent", "work-next"))
        self.assertNotIn("size", release)  # the item has no size, and a field nobody gave is left out
        self.assertEqual(release["claimed_at"], claim["claimed_at"])

    def test_the_claim_size_defaults_to_the_items(self):
        self.write("gap-cccccc", item("gap-cccccc", "C", severity="p3", anchors=["src/c.rs"], extra='size = "M"'))
        self.commit("size C")
        self.run_work("claim", "gap-cccccc", "--by", "t", "--session", "s1")
        self.assertEqual(self.events("s1")[0]["size"], "M")

    def test_sync_records_who_closed_an_item(self):
        # A commit trailer closes C, which nobody claimed; a passed plan task closes B.
        (self.root / "src" / "c.rs").write_text("fn fixed() {}\n")
        self.commit("make c work\n\nCloses: gap-cccccc")
        plan = self.root / "plans" / "p1"
        plan.mkdir(parents=True)
        (plan / "tasks.toml").write_text('[meta]\nplan = "p1"\n\n[[task]]\nid = "T1"\ncloses = ["bug-bbbbbb"]\n')
        cp = self.root / ".roko" / "state" / "graph" / "p1"
        cp.mkdir(parents=True)
        (cp / "checkpoint.json").write_text(json.dumps(
            {"run_id": "r1", "extensions": {"roko.gate.verdict@1": {"value": {"verdicts": {"T1": "passed"}}}}}))
        closed, conflicts = work.cmd_sync(type("A", (), {"dry_run": False})(), quiet=True)
        self.assertEqual((len(closed), conflicts), (2, []))
        by_id = {i["id"]: i for i in self.items()}
        self.assertEqual({k: by_id["gap-cccccc"]["closed"].get(k) for k in ("executor", "forced")},
                         {"executor": "unknown", "forced": False})
        self.assertEqual({k: by_id["bug-bbbbbb"]["closed"].get(k) for k in ("executor", "via", "run_id")},
                         {"executor": "roko-plan", "via": "roko-plan", "run_id": "r1"})
        rows = self.events("sync")
        self.assertEqual(sorted((r["item"], r["event"], r["source"], r["executor"]) for r in rows),
                         [("bug-bbbbbb", "closed", "reconciled", "roko-plan"), ("gap-cccccc", "closed", "reconciled", "unknown")])
        self.assertEqual([e for r in rows for e in work.check_event(r)], [])


if __name__ == "__main__":
    unittest.main(verbosity=1)
