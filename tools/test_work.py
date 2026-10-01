#!/usr/bin/env python3
"""Tests for tools/work.py: verify-command parsing and lint, picking non-conflicting work, claims, drift, sync and the
event log.

Run: python3 tools/test_work.py
"""

import datetime as dt
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from collections import Counter
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

    def test_static_prefix_stops_at_a_heavy_command_anywhere_in_a_part(self):
        for cmd in ("for i in $(seq 1 20); do cargo test -p roko-cli --lib x; done",
                    "CARGO_TARGET_DIR=t cargo test -p c t",
                    "bash -c 'cargo build -p c' && grep -q x f",
                    'test -n "$(cargo --version)" && grep -q x f'):
            self.assertEqual(work.static_prefix(cmd), "", cmd)
        self.assertEqual(work.static_prefix("grep -q x f && ! grep -q s g || (grep -q z h && cargo test -p c t)"), "grep -q x f")
        self.assertEqual(work.static_prefix("grep -q x f && (cd apps/portal && npx vitest run)"), "grep -q x f")
        # A heavy word in a quoted pattern or a path is not a command.
        quoted = "grep -q 'cargo test' f && grep -rq \"roko serve\" crates/roko-cli/src"
        self.assertEqual(work.static_prefix(quoted), quoted)

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

    def run_work(self, *args, check=True):
        """Run tools/work.py in the repo as a separate process, the way the skills do."""
        env = {k: v for k, v in os.environ.items() if k != "WORK_SESSION"}
        return subprocess.run([sys.executable, str(Path(work.__file__)), *args], cwd=self.root,
                              env={**env, "WORK_REPO": str(self.root)}, check=check, capture_output=True, text=True)

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


class TestWorktreeBusy(RepoTest):
    def test_next_treats_files_changed_in_other_worktrees_as_busy(self):
        # One worktree has committed a change to src/c.rs; another has an uncommitted edit to src/a.rs; a third is
        # clean and fully merged, with only an item file edited, and adds nothing.
        one = self.add_worktree("work/one")
        (one / "src" / "c.rs").write_text("fn changed() {}\n")
        subprocess.run(["git", "commit", "-qam", "change c"], cwd=one, check=True, capture_output=True)
        two = self.add_worktree("work/two")
        (two / "src" / "a.rs").write_text("fn edited() {}\n")
        three = self.add_worktree("work/three")
        (three / "work" / "items" / "gap-cccccc-x.md").write_text("edited during a sweep\n")
        busy = work.worktree_changes()
        self.assertEqual(busy, {"src/c.rs": "wt (work/one)", "src/a.rs": "wt (work/two)"})
        details = []
        picked, skipped = work.pick_next(self.items(), n=3, claims={}, details=details)
        self.assertEqual(picked, [])
        self.assertEqual(skipped["touches files changed in worktree wt (work/one)"], 1)
        self.assertEqual(skipped["touches files changed in worktree wt (work/two)"], 2)
        self.assertEqual({d["id"]: (d["worktree"], d["branch"], d["file"]) for d in details},
                         {"bug-aaaaaa": ("wt", "work/two", "src/a.rs"), "bug-bbbbbb": ("wt", "work/two", "src/a.rs"),
                          "gap-cccccc": ("wt", "work/one", "src/c.rs")})
        # The CLI says which worktree holds an item back, and --ignore-worktrees turns the scan off.
        self.assertIn("touches files changed in worktree wt (work/one)", self.run_work("next", "--n", "3").stdout)
        picked = self.run_work("next", "--n", "3", "--ignore-worktrees", "--json").stdout
        self.assertEqual([r["id"] for r in json.loads(picked)], ["bug-aaaaaa", "gap-cccccc"])

    def test_a_worker_sees_the_main_checkouts_uncommitted_edits(self):
        (self.root / "src" / "c.rs").write_text("fn dirty() {}\n")
        work.set_repo(self.add_worktree("work/worker"))
        self.assertEqual(work.worktree_changes(), {"src/c.rs": f"{self.root.name} (main)"})


LANES = """milestones = ["MS0", "MS1"]

[pools]
p = 2

[lane.x]
paths = ["src/**"]
max = 2

[lane.y]
paths = ["src/**"]
max = 1
pool = "p"

[lane.z]
paths = ["src/**"]
max = 3
pool = "p"
"""


class TestLanes(RepoTest):
    def lane_items(self, specs):
        """One item per (id, lane, milestone), each on its own file."""
        for iid, lane, ms in specs:
            (self.root / "src" / f"{iid}.rs").write_text("fn x() {}\n")
            extra = f'lane = "{lane}"' + (f'\nmilestone = "{ms}"' if ms else "")
            self.write(iid, item(iid, iid, anchors=[f"src/{iid}.rs"], extra=extra))
        (self.root / "work" / "lanes.toml").write_text(LANES)
        self.commit("lane items")

    def test_check_rejects_unknown_lane_and_parent(self):
        self.write("spec-eeeeee", item("bug-eeeeee", "Epic: E").replace("bug-eeeeee", "spec-eeeeee").replace(
            'kind = "bug"', 'kind = "spec"'))
        self.lane_items([("gap-111111", "x", "MS0")])
        self.write("gap-222222", item("gap-222222", "unknown lane", extra='lane = "nowhere"\nparent = "spec-eeeeee"'))
        self.write("gap-333333", item("gap-333333", "parent is a bug", extra='lane = "x"\nparent = "bug-aaaaaa"'))
        self.write("gap-444444", item("gap-444444", "no such parent", extra='parent = "spec-999999"'))
        self.write("gap-555555", item("gap-555555", "unknown milestone", extra='lane = "y"\nmilestone = "MS9"'))
        errs = work.validate(self.items())
        self.assertEqual(sorted(e.removeprefix("work/items/") for e in errs), sorted([
            "gap-222222-x.md: lane 'nowhere' is not in work/lanes.toml",
            "gap-333333-x.md: parent 'bug-aaaaaa' is not a spec item",
            "gap-444444-x.md: parent 'spec-999999' is not a spec item",
            "gap-555555-x.md: milestone 'MS9' is not one of work/lanes.toml's milestones",
        ]))
        # Without lanes.toml, lanes and milestones are not checked; parents still are.
        (self.root / "work" / "lanes.toml").unlink()
        self.assertEqual(len(work.validate(self.items())), 2)

    def test_next_mix_respects_lane_caps(self):
        self.lane_items([("gap-a00001", "x", None), ("gap-a00002", "x", None), ("gap-a00003", "x", None),
                         ("gap-b00001", "y", None), ("gap-b00002", "y", None),
                         ("gap-c00001", "z", None), ("gap-c00002", "z", None), ("gap-c00003", "z", None)])
        # A live claim fills lane y, and also takes one of pool p's two places.
        claims = {"gap-b00001": {"id": "gap-b00001", "by": "t", "stale": False}}
        picked, skipped = work.pick_next(self.items(), claims=claims, worktrees={}, mix={"x": 3, "y": 1, "z": 3})
        lanes = Counter(work.item_lane(i) for i in picked)
        self.assertEqual(lanes, Counter({"x": 2, "z": 1}))
        self.assertEqual(skipped["lane x is at its cap of 2"], 1)
        self.assertEqual(skipped["lane y is at its cap of 1"], 1)
        self.assertEqual(skipped["pool p is at its cap of 2"], 2)
        # --lane picks one lane only, within its cap.
        picked, _ = work.pick_next(self.items(), n=5, claims={}, worktrees={}, lane="z")
        self.assertEqual([i["id"] for i in picked], ["gap-c00001", "gap-c00002"])  # pool p caps z at 2

    def test_next_sorts_by_milestone_within_a_goal(self):
        self.lane_items([("gap-d00001", "x", "MS1"), ("gap-d00002", "x", "MS0")])
        picked, _ = work.pick_next([i for i in self.items() if i["id"].startswith("gap-d")], n=2, claims={}, worktrees={})
        self.assertEqual([i["id"] for i in picked], ["gap-d00002", "gap-d00001"])
        # The CLI takes the same mix and says so in --json.
        out = json.loads(self.run_work("next", "--mix", "x=1", "--ignore-worktrees", "--json").stdout)
        self.assertEqual([(r["id"], r["lane"]) for r in out], [("gap-d00002", "x")])


class TestEpicsAndViews(RepoTest):
    """An epic with three children, one of them closed, in lanes x and y."""

    def setUp(self):
        super().setUp()
        epic = item("bug-eeeeee", "Epic: E").replace("bug-eeeeee", "spec-eeeeee").replace('kind = "bug"', 'kind = "spec"')
        self.write("spec-eeeeee", epic.replace("depends_on = []", 'depends_on = ["gap-333333"]'))
        self.write("gap-111111", item("gap-111111", "One", status="done", anchors=["src/one.rs"], extra=(
            'lane = "x"\nparent = "spec-eeeeee"\n\n[closed]\nat = 2026-09-02\nevidence = "done"')))
        self.write("gap-222222", item("gap-222222", "Two", anchors=["src/two.rs"], extra='lane = "x"\nparent = "spec-eeeeee"'))
        self.write("gap-333333", item("gap-333333", "Three", anchors=["src/three.rs"], extra='lane = "y"'))  # via depends_on
        self.write("gap-444444", item("gap-444444", "Four", anchors=["src/four.rs"]).replace(
            "depends_on = []", 'depends_on = ["gap-222222"]'))
        self.commit("an epic")

    def test_epics_view_counts_children(self):
        work.render_root("work", self.items())
        text = (self.root / "work" / "EPICS.md").read_text()
        self.assertIn("## [spec-eeeeee](items/spec-eeeeee-x.md) E", text)
        self.assertIn("- **1/3 closed** · goal `core` · severity p2", text)
        self.assertIn("- open by lane: x 1, y 1", text)
        self.assertIn("- next: [gap-333333](items/gap-333333-x.md) Three", text)  # p2 items by title: Three < Two
        self.assertIn("| x | 1 | 2 | 1 (1) |", text)
        self.assertIn("| y | 0 | 1 | 1 (1) |", text)

    def test_show_lists_children_and_dependents(self):
        self.run_work("claim", "gap-222222", "--by", "t", "--branch", "work/two", "--session", "s1")
        epic = json.loads(self.run_work("show", "spec-eeeeee", "--json").stdout)
        self.assertEqual(sorted(c["id"] for c in epic["children"]), ["gap-111111", "gap-222222", "gap-333333"])
        two = json.loads(self.run_work("show", "gap-222222", "--json").stdout)
        self.assertEqual([d["id"] for d in two["dependents"]], ["gap-444444"])
        self.assertEqual((two["claim"]["by"], two["claim"]["branch"]), ("t", "work/two"))
        text = self.run_work("show", "gap-222222").stdout
        self.assertIn("claim: t · work/two", text)
        self.assertIn("gap-444444  open  Four", text)
        # list filters by lane, parent and status.
        self.assertEqual([r["id"] for r in json.loads(self.run_work("list", "--lane", "x", "--json").stdout)], ["gap-222222"])
        rows = json.loads(self.run_work("list", "--parent", "spec-eeeeee", "--status", "all", "--json").stdout)
        self.assertEqual(sorted(r["id"] for r in rows), ["gap-111111", "gap-222222"])
        status = json.loads(self.run_work("status", "--json").stdout)
        self.assertEqual(status["epic"]["spec-eeeeee"], {"title": "Epic: E", "open": 2, "claimed": 1, "done": 1})
        self.assertEqual(status["lane"]["x"], {"open": 1, "claimed": 1, "done": 1})


class TestClaims(RepoTest):
    def claim_file(self, iid, **fields):
        d = work.claims_dir()
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{iid}.json").write_text(json.dumps({"id": iid, "by": "t", **fields}))

    def test_claim_refuses_an_overlapping_footprint(self):
        self.run_work("claim", "bug-aaaaaa", "--by", "t1", "--branch", "work/a")
        # bug-bbbbbb is anchored on the same file, so another worker may not claim it...
        refused = self.run_work("claim", "bug-bbbbbb", "--by", "t2", "--branch", "work/b", check=False)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("its files overlap bug-aaaaaa, claimed by t1", refused.stderr)
        self.assertNotIn("bug-bbbbbb", work.load_claims())
        # ...but the worker holding bug-aaaaaa may take it on the same branch,
        self.run_work("claim", "bug-bbbbbb", "--by", "t1", "--branch", "work/a")
        self.run_work("release", "bug-bbbbbb")
        # and --force overrides, saying what it overrode.
        forced = self.run_work("claim", "bug-bbbbbb", "--by", "t2", "--branch", "work/b", "--force")
        self.assertIn("--force overrides: its files overlap bug-aaaaaa", forced.stderr)
        self.assertEqual(work.load_claims()["bug-bbbbbb"]["by"], "t2")

    def test_claim_refuses_an_item_whose_dependency_is_open(self):
        self.write("gap-dddddd", item("gap-dddddd", "D", anchors=["src/d.rs"]).replace(
            "depends_on = []", 'depends_on = ["gap-cccccc"]'))
        self.commit("D waits on C")
        refused = self.run_work("claim", "gap-dddddd", "bug-aaaaaa", "--by", "t", check=False)
        self.assertIn("it depends on gap-cccccc, which is still open", refused.stderr)
        # The other item of the same command is claimed all the same, and the command fails.
        self.assertNotEqual(refused.returncode, 0)
        self.assertEqual(sorted(work.load_claims()), ["bug-aaaaaa"])
        self.run_work("release", "bug-aaaaaa")
        # One worker may take both on one branch and do them in turn.
        self.run_work("claim", "gap-cccccc", "gap-dddddd", "--by", "t", "--branch", "work/cd")
        self.assertEqual(sorted(work.load_claims()), ["gap-cccccc", "gap-dddddd"])

    def test_claim_renew_extends_the_ttl_by_size(self):
        now = dt.datetime.now().astimezone()
        ago = lambda h: (now - dt.timedelta(hours=h)).isoformat(timespec="seconds")  # noqa: E731
        self.claim_file("gap-cccccc", at=ago(30), size="M", branch="work/c")
        self.claim_file("bug-aaaaaa", at=ago(9), size="S")
        self.claim_file("bug-bbbbbb", at=ago(7), size="S")
        claims = work.load_claims()
        self.assertEqual({k: (c["ttl_h"], c["stale"]) for k, c in claims.items()},
                         {"gap-cccccc": (24, True), "bug-aaaaaa": (8, True), "bug-bbbbbb": (8, False)})
        # Only the claimant renews; the renewed M claim, made 30 h ago, is live again.
        other = self.run_work("claim", "gap-cccccc", "--renew", "--by", "someone-else", check=False)
        self.assertIn("only the claimant renews a claim", other.stderr)
        self.run_work("claim", "gap-cccccc", "--renew", "--by", "t", "--session", "s1")
        c = work.load_claims()["gap-cccccc"]
        self.assertEqual((c["stale"], c["at"]), (False, ago(30)))
        self.assertRegex(c["renewed_at"], work.TS_RE)
        (renewal,) = self.events("s1")
        self.assertEqual((renewal["event"], renewal["renew"], renewal["branch"]), ("claim", True, "work/c"))

    def test_next_and_claims_delete_nothing(self):
        self.write("gap-cccccc", item("gap-cccccc", "C", status="done",
                                      extra='\n[closed]\nat = 2026-09-02\nevidence = "done"'))
        self.write("gap-eeeeee", item("gap-eeeeee", "E", anchors=["src/c.rs"]))
        self.commit("close C; E touches the same file")
        self.claim_file("gap-cccccc", at=dt.datetime.now().astimezone().isoformat(timespec="seconds"))
        self.run_work("next", "--n", "3")
        self.run_work("claims")
        self.assertIn("gap-cccccc", work.load_claims())
        # Meanwhile a claim on a closed item holds no file back.
        self.assertIn("gap-eeeeee", [i["id"] for i in work.pick_next(self.items(), n=3, worktrees={})[0]])
        self.assertIn("pruned gap-cccccc (closed)", self.run_work("claims", "--prune").stdout)
        self.assertNotIn("gap-cccccc", work.load_claims())


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


class TestFolders(RepoTest):
    """Open items live in items/, done ones in done/, won't-fix and superseded ones in closed/, parked ones in parked/."""

    def test_close_moves_the_item_to_done(self):
        it = {i["id"]: i for i in self.items()}["gap-cccccc"]
        work.close_item(it, evidence="done in test", commit="abc1234")
        self.assertFalse((self.root / "work" / "items" / "gap-cccccc-x.md").exists())
        self.assertTrue((self.root / "work" / "done" / "gap-cccccc-x.md").exists())
        moved = {i["id"]: i for i in self.items()}["gap-cccccc"]
        self.assertEqual((moved["status"], moved["_dir"]), ("done", "done"))
        self.assertEqual(work.validate(self.items()), [])

    def test_a_superseded_item_moves_to_closed(self):
        self.run_work("close", "bug-bbbbbb", "--status", "superseded", "--evidence", "covered by bug-aaaaaa")
        self.assertTrue((self.root / "work" / "closed" / "bug-bbbbbb-x.md").exists())
        self.assertEqual(work.validate(self.items()), [])

    def test_check_flags_and_tidy_moves_a_closed_item_left_in_items(self):
        # An older work.py (or a branch merged from before the folders existed) closes an item in place.
        self.write("gap-cccccc", item("gap-cccccc", "C", status="done",
                                      extra='\n[closed]\nat = 2026-09-30\nevidence = "done elsewhere"'))
        errs = work.validate(self.items())
        self.assertTrue(any("run `work.py tidy`" in e for e in errs), errs)
        out = self.run_work("tidy").stdout
        self.assertIn("gap-cccccc → work/done/", out)
        self.assertTrue((self.root / "work" / "done" / "gap-cccccc-x.md").exists())
        self.assertEqual(work.validate(self.items()), [])
        self.assertIn("tidy: moved 0", self.run_work("tidy").stdout)

    def test_unpark_returns_an_item_to_items(self):
        self.run_work("park", "gap-cccccc", "--reason", "not planned")
        self.assertTrue((self.root / "work" / "parked" / "gap-cccccc-x.md").exists())
        self.run_work("unpark", "gap-cccccc")
        self.assertTrue((self.root / "work" / "items" / "gap-cccccc-x.md").exists())
        self.assertEqual(work.validate(self.items()), [])


if __name__ == "__main__":
    unittest.main(verbosity=1)
