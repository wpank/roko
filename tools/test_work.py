#!/usr/bin/env python3
"""Tests for tools/work.py: verify-command parsing and lint, picking non-conflicting work, claims, drift and sync.

Run: python3 tools/test_work.py
"""

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


if __name__ == "__main__":
    unittest.main(verbosity=1)
