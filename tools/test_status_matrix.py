#!/usr/bin/env python3
"""Tests for tools/status_matrix.py: row checks at the pinned commit, rendering, --check and --probe-head.

Every test builds its own temporary git repository, so the real tree and its later renames never affect the result.

Run: python3 tools/test_status_matrix.py
"""

import contextlib
import io
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import status_matrix as sm  # noqa: E402

LIB = "pub fn alpha() {}\npub struct Beta;\n\n#[test]\nfn alpha_works() {}\n"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True, text=True).stdout.strip()


def commit_all(repo: Path, msg: str) -> str:
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", msg)
    return git(repo, "rev-parse", "--short=9", "HEAD")


def data(pin: str, *, rows: str = "", claims: str = "") -> str:
    rows = rows or textwrap.dedent("""
        [[row]]
        id = "AU1"
        group = "AU"
        name = "Alpha planner"
        tag = "WIRED"
        verdict = "keep"
        note = "Calls `alpha` on every run."
        anchors = ["crates/demo/src/lib.rs::alpha", "crates/demo/src/lib.rs::Beta"]
        evidence = ["{pin}", "crates/demo/src/lib.rs::alpha_works"]

        [[row]]
        id = "AU2"
        group = "AU"
        name = "Gamma lint"
        tag = "MISSING"
        verdict = "build"
        note = "Nothing lints plans yet."
        anchors = []
        evidence = ["gap-aaaaaa"]
        item = "gap-aaaaaa"
    """)
    claims = claims or textwrap.dedent("""
        [[claim]]
        id = "V1"
        name = "Plans are checked"
        tag = "PARTIAL"
        rows = ["AU1", "AU2"]
        blocker = "No lint exists."
    """)
    head = textwrap.dedent(f"""
        [matrix]
        status = "draft"
        budget = "none"
        owner = "gap-35a614"
        pinned = "{pin}"
        intro = "Status at {{pin}} ({{date}})."

        [[group]]
        id = "AU"
        name = "Authoring"
    """)
    return head + rows.replace("{pin}", pin) + claims


class MatrixRepo(unittest.TestCase):
    """A repository with one crate, one work item and a data file pinned at its first commit."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.com")
        git(self.repo, "config", "user.name", "Test")
        self.write("crates/demo/src/lib.rs", LIB)
        self.write("work/items/gap-aaaaaa-demo-item.md", '+++\nid = "gap-aaaaaa"\n+++\n')
        self.write("work/items/gap-35a614-status-matrix.md", '+++\nid = "gap-35a614"\n+++\n')
        self.pin = commit_all(self.repo, "base")
        self.set_data(data(self.pin))

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, rel: str, text: str):
        path = self.repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def set_data(self, text: str):
        self.write("docs/whitepaper/data/mechanisms.toml", text)

    def run_tool(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = sm.main(["--repo", str(self.repo), *args])
        return code, out.getvalue(), err.getvalue()

    def appendix(self) -> str:
        return (self.repo / "docs/whitepaper/appendix-status-matrix.md").read_text()


class TestRender(MatrixRepo):
    def test_write_then_check_passes(self):
        code, _, err = self.run_tool()
        self.assertEqual(code, 0, err)
        code, out, err = self.run_tool("--check")
        self.assertEqual(code, 0, err)
        self.assertIn("2 rows ok", out)

    def test_status_header_comes_from_the_data_file(self):
        self.run_tool()
        self.assertEqual(self.appendix().splitlines()[0], "Status: draft · budget none · owner gap-35a614")
        self.set_data(data(self.pin).replace('status = "draft"', 'status = "reviewed"'))
        self.run_tool()
        self.assertTrue(self.appendix().startswith("Status: reviewed · "))

    def test_every_tag_carries_the_pinned_commit(self):
        self.run_tool()
        text = self.appendix()
        tags = re.findall(r"(?<![\w-])(WIRED|PARTIAL|MISSING|BUILT-UNWIRED|UNPROVEN)(@\w+)?", text)
        self.assertTrue(tags)
        self.assertEqual({suffix for _, suffix in tags}, {f"@{self.pin}"})
        self.assertIn(f"| AU1 | Alpha planner | WIRED@{self.pin} |", text)
        self.assertIn("`crates/demo/src/lib.rs`: `alpha`, `Beta`", text)
        self.assertIn(f"code read at `{self.pin}`", text)
        self.assertIn("| V1 Plans are checked | PARTIAL@" + self.pin, text)

    def test_missing_item_on_an_actionable_row_warns_but_passes(self):
        rows = data(self.pin).split("[[row]]")
        unfiled = rows[1].replace('tag = "WIRED"', 'tag = "PARTIAL"').replace('verdict = "keep"', 'verdict = "fix"')
        self.set_data("[[row]]".join([rows[0], unfiled, rows[2]]))
        code, _, err = self.run_tool()
        self.assertEqual(code, 0, err)
        self.assertIn("warning: row AU1: PARTIAL with verdict 'fix' but no work item filed", err)
        self.assertIn("| fix · no item filed |", self.appendix())


class TestCheck(MatrixRepo):
    def setUp(self):
        super().setUp()
        code, _, err = self.run_tool()
        self.assertEqual(code, 0, err)

    def test_check_fails_when_an_anchor_is_removed(self):
        self.write("crates/demo/src/lib.rs", LIB.replace("pub fn alpha() {}\n", ""))
        new_pin = commit_all(self.repo, "remove alpha")
        self.set_data(data(new_pin))
        code, _, err = self.run_tool("--check")
        self.assertEqual(code, 1)
        self.assertIn("row AU1: anchor 'crates/demo/src/lib.rs::alpha': alpha is not in crates/demo/src/lib.rs", err)

    def test_check_fails_when_an_anchor_file_is_gone(self):
        self.set_data(data(self.pin).replace("crates/demo/src/lib.rs::Beta", "crates/demo/src/gone.rs::Beta"))
        code, _, err = self.run_tool("--check")
        self.assertEqual(code, 1)
        self.assertIn("crates/demo/src/gone.rs does not exist", err)

    def test_check_fails_when_the_appendix_differs(self):
        path = self.repo / "docs/whitepaper/appendix-status-matrix.md"
        path.write_text(path.read_text().replace("Alpha planner", "Alpha planner, edited by hand"))
        code, _, err = self.run_tool("--check")
        self.assertEqual(code, 1)
        self.assertIn("is out of date", err)

    def test_check_fails_when_the_appendix_is_missing(self):
        (self.repo / "docs/whitepaper/appendix-status-matrix.md").unlink()
        self.assertEqual(self.run_tool("--check")[0], 1)

    def test_probe_head_lists_rows_whose_anchor_is_gone(self):
        self.write("crates/demo/src/lib.rs", LIB.replace("pub struct Beta;\n", ""))
        commit_all(self.repo, "remove Beta")
        # The pin still has Beta, so the matrix itself stays valid...
        self.assertEqual(self.run_tool("--check")[0], 0)
        # ...but the probe reports the row that a refresh must revisit.
        code, out, _ = self.run_tool("--probe-head")
        self.assertEqual(code, 1)
        self.assertIn("AU1\tcrates/demo/src/lib.rs::Beta", out)
        self.assertNotIn("alpha", out)

    def test_probe_head_passes_when_every_anchor_exists(self):
        code, _, err = self.run_tool("--probe-head")
        self.assertEqual(code, 0, err)


class TestRowRules(MatrixRepo):
    def assert_rejected(self, text: str, message: str):
        self.set_data(text)
        code, _, err = self.run_tool("--check")
        self.assertEqual(code, 1, err)
        self.assertIn(message, err)

    def test_row_without_anchors_must_be_missing_docs_only_or_removed(self):
        self.assert_rejected(data(self.pin).replace('tag = "MISSING"', 'tag = "PARTIAL"'),
                             "row AU2: a row without anchors must be tagged")

    def test_row_without_anchors_must_name_an_item(self):
        self.assert_rejected(data(self.pin).replace('item = "gap-aaaaaa"', ""),
                             "row AU2: a row without anchors must name the item")

    def test_unknown_tag_and_verdict_are_rejected(self):
        text = data(self.pin).replace('tag = "WIRED"', 'tag = "DONE"')
        self.assert_rejected(text, "row AU1: unknown tag 'DONE'")
        self.assert_rejected(data(self.pin).replace('verdict = "keep"', 'verdict = "ship"'),
                             "row AU1: unknown verdict 'ship'")

    def test_evidence_commit_must_be_an_ancestor_of_the_pin(self):
        git(self.repo, "checkout", "-q", "-b", "side")
        self.write("side.txt", "x\n")
        side = commit_all(self.repo, "side")
        git(self.repo, "checkout", "-q", "-")
        self.assert_rejected(data(self.pin).replace(f'evidence = ["{self.pin}"', f'evidence = ["{side}"'),
                             f"evidence commit {side} is not an ancestor of the pin")

    def test_evidence_must_not_be_empty(self):
        self.assert_rejected(data(self.pin).replace('evidence = ["gap-aaaaaa"]', "evidence = []"),
                             "row AU2: evidence is empty")

    def test_work_items_must_exist_at_the_pin(self):
        self.assert_rejected(data(self.pin).replace('item = "gap-aaaaaa"', 'item = "gap-bbbbbb"'),
                             "work item gap-bbbbbb does not exist")

    def test_text_rejects_bare_tags_and_unknown_identifiers(self):
        self.assert_rejected(data(self.pin).replace("Calls `alpha` on every run.", "WIRED since the start."),
                             "row AU1 note: bare status tag WIRED")
        self.assert_rejected(data(self.pin).replace("Calls `alpha`", "Calls `omega`"),
                             "row AU1 note: `omega` is not in crates/")
        self.assert_rejected(data(self.pin).replace("Calls `alpha`", "Calls `roko plan run`"),
                             "is neither one identifier nor a repository path")

    def test_text_commits_must_be_in_the_pins_history(self):
        self.write("later.txt", "x\n")
        later = commit_all(self.repo, "after the pin")
        self.set_data(data(self.pin).replace("Nothing lints plans yet.", f"Nothing lints plans since `{self.pin}`."))
        self.assertEqual(self.run_tool()[0], 0)
        self.assertIn(f"since `{self.pin}`", self.appendix())
        self.assert_rejected(data(self.pin).replace("Nothing lints plans yet.", f"Fixed in `{later}`."),
                             f"row AU2 note: commit `{later}` is not in the history of the pin")

    def test_claims_must_name_existing_rows(self):
        self.assert_rejected(data(self.pin).replace('rows = ["AU1", "AU2"]', 'rows = ["AU1", "AU9"]'),
                             "claim V1: rows must name existing rows (unknown: AU9)")

    def test_pin_must_be_a_commit(self):
        self.assert_rejected(data(self.pin).replace(f'pinned = "{self.pin}"', 'pinned = "0123abcd"'),
                             "matrix: pinned '0123abcd' is not a commit")


if __name__ == "__main__":
    unittest.main()
