#!/usr/bin/env python3
"""Tests for tools/paperlint.py: each --strict rule on a fixture section, budgets, identifiers and the report.

The fixtures live in a throwaway git repository with a small crates/ tree, so the tests never depend on the real
tree (gap-528762 will add identifiers that are phantoms today).

Run: python3 tools/test_paperlint.py [-k NAME]
"""

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import paperlint  # noqa: E402

CRATE = """\
pub struct ReadyQueue;
pub fn run_gate_once() {}
pub struct GraphTaskDispatcher;
impl GraphTaskDispatcher { pub fn dispatch(&self) {} }
"""

README = """\
# Whitepaper

## Outline

| File | Section | Budget (words) | Owner |
|---|---|---|---|
| `00-abstract.md` | Abstract | 40 | gap-aaaaaa |
| `01-introduction.md` | Introduction | 120 | gap-aaaaaa |
| [appendix-status-matrix.md](appendix-status-matrix.md) | Status matrix | — | gap-aaaaaa |

## Conventions

- Line 1 of every section is a status header.
- **Banned words:** "immune system", "dreams", "provably".

## Build

Nothing is "dreams" here: this section is not a convention.
"""

BIB = """\
@book{wiener1948,
  title = {Cybernetics},
  author = {Norbert Wiener},
  year = {1948}
}
"""

# Passes every --strict rule; {sha} is the fixture's HEAD. 116 words against a budget of 120.
CLEAN = """\
Status: draft · budget 120 words · owner gap-aaaaaa

# 1 Introduction

Roko runs a plan as a graph of tasks. The ready queue (`ReadyQueue` in `crates/core/src/lib.rs`) is
WIRED@{sha}, and each task passes its gates through `run_gate_once` before
`crates/core/src/lib.rs::GraphTaskDispatcher` records the verdict.

Each attempt will get a key, `AttemptKey` (designed), that names its run, plan and task. Later sections
use `AttemptKey` without repeating the tag. Feedback control has a long history [@wiener1948].

Before the fix, 101 of 373 recorded successes had a failing gate.[^b7] The operator loop cost about
16× what Roko recorded.[^op] Every plan lists [[task.verify]] tables in its `tasks.toml`, and a figure
shows the loop.

![Figure 1: the golden path](figures/fig1.svg)

[^b7]: Rollup `totals.escapes`, snapshot 2026-09-29, at commit {sha}.
[^op]: An estimate, tracked in gap-aaaaaa.
"""

STUB = """\
Status: stub · budget 40 words · owner gap-aaaaaa

# Abstract

[[TODO: write the abstract (gap-aaaaaa)]]

Claims: A1 frontier models plan and cheap models execute (tldr/04).
"""


def run(*args) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        try:
            code = paperlint.main([str(a) for a in args])
        except SystemExit as e:  # argparse
            code = e.code
    return code, out.getvalue()


def rules_in(output: str) -> set[str]:
    return {line.split("[", 1)[1].split("]", 1)[0] for line in output.splitlines() if ": [" in line}


class FixtureRepo(unittest.TestCase):
    """One throwaway repository for the whole class: crates/, a work item, and a commit off to the side."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name).resolve()
        cls.git("init", "-q", "-b", "main")
        (cls.root / "crates" / "core" / "src").mkdir(parents=True)
        (cls.root / "crates" / "core" / "src" / "lib.rs").write_text(CRATE)
        (cls.root / "work" / "items").mkdir(parents=True)
        (cls.root / "work" / "items" / "gap-aaaaaa-x.md").write_text("+++\nid = \"gap-aaaaaa\"\n+++\n")
        cls.git("add", "-A")
        cls.git("commit", "-q", "-m", "initial")
        cls.git("checkout", "-q", "-b", "side")
        cls.git("commit", "-q", "--allow-empty", "-m", "not merged")
        cls.side = cls.git("rev-parse", "--short=9", "HEAD").strip()
        cls.git("checkout", "-q", "main")
        cls.sha = cls.git("rev-parse", "--short=9", "HEAD").strip()
        cls.n = 0

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    @classmethod
    def git(cls, *args) -> str:
        cmd = ["git", "-c", "user.email=t@example.com", "-c", "user.name=t", "-c", "commit.gpgsign=false",
               "-c", "core.hooksPath=/dev/null", *args]
        return subprocess.run(cmd, cwd=cls.root, check=True, capture_output=True, text=True).stdout

    def paper(self, readme: str = README) -> Path:
        """A fresh whitepaper-style directory: README (outline and conventions), bibliography and a figure."""
        type(self).n += 1
        d = self.root / "docs" / f"paper{self.n}"
        (d / "figures").mkdir(parents=True)
        (d / "figures" / "fig1.svg").write_text("<svg xmlns='http://www.w3.org/2000/svg'/>\n")
        (d / "README.md").write_text(readme)
        (d / "references.bib").write_text(BIB)
        return d

    def section(self, d: Path, name: str, text: str) -> Path:
        p = d / name
        p.write_text(textwrap.dedent(text).replace("{sha}", self.sha).replace("{side}", self.side))
        return p


class TestStrict(FixtureRepo):
    def test_strict_passes_clean_section(self):
        p = self.section(self.paper(), "01-introduction.md", CLEAN)
        code, out = run("--strict", p)
        self.assertEqual(code, 0, out)
        self.assertIn("1 file clean", out)

    def test_strict_fails_on_each_rule(self):
        replace = lambda old, new: lambda t: t.replace(old, new, 1)  # noqa: E731
        append = lambda extra: lambda t: t.rstrip("\n") + "\n\n" + extra + "\n"  # noqa: E731
        cases = [
            # rule 6: status header, stub, word count, budget agreement
            ("no status header", "header", lambda t: t.split("\n", 1)[1]),
            ("stub", "header", replace("Status: draft", "Status: stub")),
            ("over budget", "header", append(" ".join(["word"] * 60) + ".")),
            ("under budget", "header", lambda t: t.split("Roko runs", 1)[0].rstrip() + "\n"),
            ("header budget differs from README", "header", replace("budget 120 words", "budget 200 words")),
            # rule 1: markers outside code
            ("leftover marker", "marker", append("[[TODO: cite the portal numbers]]")),
            ("unclosed marker", "marker", append("[[RESULT H1: the envelope")),
            # rule 2: citations
            ("unknown citation", "citation", replace("[@wiener1948]", "[@nobody2020]")),
            # rule 3: identifiers and paths at HEAD
            ("phantom identifier", "identifier", append("The loop reads `LoopSpec` at start.")),
            ("tag in another sentence", "identifier", append("Nothing is built yet (designed). It reads `LoopSpec`.")),
            ("missing path", "identifier", append("See `crates/core/src/missing.rs` for details.")),
            ("missing symbol", "identifier", replace("::GraphTaskDispatcher", "::NoSuchDispatcher")),
            # rule 4: status tags
            ("tag without commit", "status-tag", append("Integration is ORPHANED today.")),
            ("tag commit off HEAD", "status-tag", append(f"Integration is PARTIAL@{self.side} today.")),
            ("tag commit unknown", "status-tag", append("Integration is BROKEN@1234567 today.")),
            # rule 5: numbers without a source
            ("unsourced $ amount", "number", append("The pilot cost $12.50 in total.")),
            ("unsourced percentage", "number", append("About 27% of greens were false.")),
            ("unsourced ratio", "number", append("Cheap models were 3.5× cheaper.")),
            ("unsourced N/M", "number", append("The autonomy index is 3/40.")),
            ("footnote without a source", "number", append("It took 12 of 40 runs.[^x]\n\n[^x]: Trust me.")),
            # rule 7: banned words
            ("banned word", "banned", append("The dreams subsystem is parked.")),
            ("banned phrase", "banned", append("An Immune  System checks outputs.")),
            # rule 8: links and figure paths
            ("broken figure path", "link", replace("figures/fig1.svg", "figures/missing.svg")),
            ("broken link", "link", append("See [the plan](missing-plan.md).")),
        ]
        for name, rule, mutate in cases:
            with self.subTest(name):
                d = self.paper()
                text = mutate(CLEAN.replace("{sha}", self.sha))
                p = self.section(d, "01-introduction.md", text)
                code, out = run("--strict", p)
                self.assertEqual(code, 1, out)
                self.assertEqual(rules_in(out), {rule}, out)

    def test_require_status(self):
        p = self.section(self.paper(), "01-introduction.md", CLEAN)
        self.assertEqual(run("--strict", "--require-status", "draft", p)[0], 0)
        code, out = run("--strict", "--require-status", "reviewed", p)
        self.assertEqual(code, 1)
        self.assertIn("[status] status is draft, not reviewed", out)

    def test_stub_fails_strict_and_report_exits_zero(self):
        d = self.paper()
        p = self.section(d, "00-abstract.md", STUB)
        code, out = run("--strict", p)
        self.assertEqual(code, 1)
        self.assertIn("status is stub", out)
        self.assertIn("[marker] leftover marker [[TODO: write the abstract (gap-aaaaaa)]]", out)
        code, out = run("--report", p)
        self.assertEqual(code, 0, out)
        self.assertIn("markers  TODO 1", out)
        self.assertIn("status stub", out)
        self.assertIn("budget 40", out)

    def test_directory_means_its_section_files(self):
        d = self.paper()
        self.section(d, "01-introduction.md", CLEAN)
        self.section(d, "appendix-status-matrix.md", "Status: draft · owner gap-aaaaaa\n\n# Status matrix\n\n"
                                                     "Every row is WIRED@{sha}.\n")
        (d / "REVIEW.md").write_text("[[TODO: not a section]]\n")
        code, out = run("--strict", d)
        self.assertEqual(code, 0, out)
        self.assertIn("2 files clean", out)
        code, out = run("--report", d)
        self.assertIn("appendix-status-matrix.md\n  status draft · 6 words · no budget", out)


class TestMarkersAndCitations(FixtureRepo):
    def test_toml_headers_and_code_are_not_markers(self):
        p = self.section(self.paper(), "01-introduction.md", """\
            Status: draft · owner gap-aaaaaa

            # T

            A task has [[task.verify]] and [[verify]] tables, and the grammar is `[[RESULT H#: …]]`.

            ```toml
            [[TODO]]
            ```

                [[TODO: an indented example]] costs $5, 27% of it.

            - A list item's continuation:

                  ```
                  [[TODO]]
                  ```
            """)
        code, out = run("--strict", p)
        self.assertEqual(rules_in(out) & {"marker", "number"}, set(), out)

    def test_status_tags_in_code(self):
        p = self.section(self.paper(), "01-introduction.md", """\
            Status: draft · owner gap-aaaaaa

            # T

            The legend uses `WIRED` and `MISSING`; the queue is `WIRED@{sha}` but the router is `PARTIAL@1234567`.
            """)
        code, out = run("--strict", p)
        self.assertEqual(out.count("[status-tag]"), 1, out)
        self.assertIn("PARTIAL@1234567: 1234567 is not a commit in this repository", out)

    def test_markers_may_wrap_lines_and_report_counts_them(self):
        p = self.section(self.paper(), "01-introduction.md", """\
            Status: template · owner gap-aaaaaa

            # T

            Cheap models reach [[RESULT
            H1: R at E*]] of frontier-direct at [[RESULT H1: C]]; see [[FIG F2]] [[AS-BUILT: confirm]].
            """)
        code, out = run("--report", p)
        self.assertEqual(code, 0)
        self.assertIn("markers  AS-BUILT 1 · FIG 1 · RESULT 2", out)
        self.assertRegex(out, r"findings .*marker 4")

    def test_citation_groups_and_aliases(self):
        d = self.paper()
        (d / "key-aliases.json").write_text('{"wiener1948cyb": "wiener1948"}')
        p = self.section(d, "01-introduction.md", """\
            Status: draft · owner gap-aaaaaa

            # T

            Control [see @wiener1948, p. 3; @wiener1948cyb]. Mail t@example.com is not a citation, nor is WIRED@abc.
            """)
        code, out = run("--strict", p)
        self.assertIn("[citation] [@wiener1948cyb] is not in references.bib; it is an alias of wiener1948", out)
        self.assertEqual(out.count("[citation]"), 1, out)


class TestIdentifiers(FixtureRepo):
    def check(self, body: str) -> tuple[int, str]:
        p = self.section(self.paper(), "01-introduction.md", "Status: draft · owner gap-aaaaaa\n\n# T\n\n" + body)
        return run("--check-identifiers", p)

    def test_covered_uses(self):
        cases = {
            "inside AS-BUILT": "The key [[AS-BUILT: `AttemptKey`, built by gap-528762]] names attempts.\n",
            "(designed) in the sentence": "The key `AttemptKey` (designed) names attempts.\n",
            "(external) in the sentence": "Claude Code reports `modelUsage` (external) per model.\n",
            "MISSING in the table row": "| `AttemptKey` | MISSING@abc1234 |\n|---|---|\n",
            "MISSING@sha in the sentence": "Each attempt gets an `AttemptKey`, MISSING@abc1234 (gap-528762).\n",
            "first use covers later ones": "The key `AttemptKey` (designed) names attempts.\n\nLater, `AttemptKey` is"
                                           " used.\n",
            "code blocks are examples": "```rust\nlet k = AttemptKey::new();\n```\n",
            "runtime and relative paths": "State lives in `.roko/state/graph/<plan>/` and `../specs/S05.md`.\n",
        }
        for name, body in cases.items():
            with self.subTest(name):
                code, out = self.check(body)
                self.assertEqual(code, 0, out)

    def test_uncovered_uses(self):
        cases = {
            "phantom CamelCase": ("The key `AttemptKey` names attempts.\n", "`AttemptKey` is not in crates/ at HEAD"),
            "phantom snake_case": ("It calls `audit_select()`.\n", "`audit_select` is not in crates/ at HEAD"),
            "phantom in an expression": ("Set `cost_source = unknown`.\n", "`cost_source` is not in crates/"),
            "later cover is too late": ("It reads `LoopSpec`.\n\nThen `LoopSpec` (designed) again.\n",
                                        "`LoopSpec` is not in crates/ at HEAD (2 uses)"),
            "missing directory": ("See `crates/gate/`.\n", "path `crates/gate` does not exist at HEAD"),
            "untracked top-level path": ("See `work/notes.md`.\n", "path `work/notes.md` does not exist at HEAD"),
        }
        for name, (body, message) in cases.items():
            with self.subTest(name):
                code, out = self.check(body)
                self.assertEqual(code, 1, out)
                self.assertIn(message, out)

    def test_real_code_passes(self):
        code, out = self.check("It uses `ReadyQueue`, `run_gate_once()`, `crates/core/src/lib.rs:2`, "
                               "`crates/core/src/lib.rs::GraphTaskDispatcher::dispatch` and `crates/core/`. Plain "
                               "words such as `verify`, bib keys such as `wiener1948`, flags such as `--resume` and "
                               "tags such as `WIRED@abc1234` are not identifiers.\n")
        self.assertEqual(code, 0, out)

    def test_check_identifiers_alone_ignores_other_rules(self):
        code, out = self.check("[[TODO: later]] It cost $5, 27% more, with `ReadyQueue`. WIRED and dreams.\n")
        self.assertEqual(code, 0, out)


class TestBudgets(FixtureRepo):
    OUTLINE = """\
        # Outline

        > **Format:** single-column, roughly 100 words per page.

        | § | File | Title | Pages | Notes |
        |---|---|---|---|---|
        | 1 | `sections/01-intro.md` | Introduction | 1 | see `../01-THESIS.md` |
        | 3 | `sections/03a-rel.md` (3.1) + `sections/03b-rel.md` (3.2) | Related work | 2 | |
        | A | `sections/A-bench.md` | Benchmark | ∞ | |

        | ID | Kind | Content |
        |---|---|---|
        | F1 | diagram | `sections/02-missing.md` is named in a table with no budget column |
        """

    def research_paper(self) -> Path:
        type(self).n += 1
        d = self.root / "tmp" / f"paper{self.n}"
        (d / "sections").mkdir(parents=True)
        (d / "OUTLINE.md").write_text(textwrap.dedent(self.OUTLINE))
        (d / "00-README.md").write_text("## Conventions\n\n- **No \"first\", \"provably\" or \"self-aware\".**\n")
        return d / "sections"

    def words(self, n: int) -> str:
        return "Status: stable-draft · owner PS1\n\n# Title\n\n" + " ".join(["word"] * (n - 1)) + "\n"

    def test_budget_from_pages_split_over_files(self):
        s = self.research_paper()
        intro = self.section(s, "01-intro.md", self.words(115))
        rel_a = self.section(s, "03a-rel.md", self.words(130))
        rel_b = self.section(s, "03b-rel.md", self.words(90))
        bench = self.section(s, "A-bench.md", self.words(5000))
        self.assertEqual(run("--budget", "1.2", intro, rel_b, bench)[0], 0)
        self.assertEqual(run("--budget", "1.1", intro)[0], 1)
        code, out = run("--budget", "1.2", rel_a)
        self.assertEqual(code, 1)
        self.assertIn("130 words is 1.30× the budget of 100 (OUTLINE.md: 2 pages × 100 words, split over 2 files);"
                      " the limit is 1.2×, so cut 10 words", out)

    def test_file_missing_from_the_outline_fails_budget(self):
        s = self.research_paper()
        p = self.section(s, "02-missing.md", self.words(10))
        code, out = run("--budget", "1.2", p)
        self.assertEqual(code, 1)
        self.assertIn("no word budget for 02-missing.md", out)

    def test_header_budget_is_the_fallback(self):
        d = self.paper()
        p = self.section(d, "05-extra.md", "Status: draft · budget 10 words · owner gap-aaaaaa\n\n# T\n\n"
                                           + "word " * 20 + "\n")
        code, out = run("--budget", "1.5", p)
        self.assertEqual(code, 1)
        self.assertIn("the budget of 10 (its status header)", out)

    def test_word_count_leaves_out_editorial_parts(self):
        s = self.research_paper()
        p = self.section(s, "01-intro.md", """\
            Status: template · last edit 2026-09-28 · owner: PS1

            # 5 Section

            > **Template.** This note to the author is not counted.

            One two three four five.[^n]
            <!-- six seven eight -->
            [[RESULT H1: a long description of the result and where it comes from]] nine.
            [[AS-BUILT: ten eleven]]

            ```toml
            [[task]]
            ```

                an indented code block is not counted

            [^n]: Nor is a footnote definition.

            ## Alternative framing A

            Not counted at all.

            ## Placeholder map

            | Not | Counted |

            ## Claims ledger (§5)

            | ID | Claim |
            |---|---|
            | C5.1 | Not counted |
            """)
        # "5 Section" 2, "One … five." 5, one RESULT slot plus "nine." 2, "[[AS-BUILT: ten eleven]]" 3
        self.assertEqual(paperlint.Doc(p, "x").word_count(), 12)

    def test_word_count_leaves_out_the_claims_ledger(self):
        s = self.research_paper()
        p = self.section(s, "01-intro.md", """\
            Status: stable-draft · owner PS3

            # 3 Related work

            Prose one two.

            | A table | outside the ledger |
            |---|---|
            | counts | too |

            ### Claims ledger (§3.1–§3.3)

            | ID | Claim | Type (lit / design / result) | Evidence (key, spec, or H#) | Status |
            |---|---|---|---|---|
            | C3a.1 | A long claim that must not count toward the budget | lit | lee2026meta | supported |
            | C3a.2 | Another one | design | S05 | pending |
            """)
        # "3 Related work" 3, "Prose one two." 3, the other table 7; nothing under the ledger heading
        self.assertEqual(paperlint.Doc(p, "x").word_count(), 13)

    def test_banned_words_from_the_research_style_rules(self):
        s = self.research_paper()
        p = self.section(s, "01-intro.md", "Status: stable-draft · owner PS1\n\n# T\n\nThe first harness, provably "
                                           "safe. First-try rates and a verdict-first loop are fine.\n")
        code, out = run("--strict", p)
        self.assertEqual(out.count("[banned]"), 2, out)
        self.assertIn('banned word "first"', out)
        self.assertIn('banned word "provably"', out)


class TestCommandLine(FixtureRepo):
    def test_usage_errors_exit_2(self):
        p = self.section(self.paper(), "01-introduction.md", CLEAN)
        tool = Path(paperlint.__file__)
        for args in ([str(p)], ["--strict", str(self.root / "nope.md")], ["--budget", "0", str(p)]):
            with self.subTest(args):
                r = subprocess.run([sys.executable, str(tool), *args], capture_output=True, text=True,
                                   cwd=self.root, env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"})
                self.assertEqual(r.returncode, 2, r.stdout + r.stderr)

    def test_cli_strict_run(self):
        p = self.section(self.paper(), "01-introduction.md", CLEAN)
        tool = Path(paperlint.__file__)
        r = subprocess.run([sys.executable, str(tool), "--strict", "--check-identifiers", "--budget", "1.2",
                            str(p.relative_to(self.root))], capture_output=True, text=True, cwd=self.root)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_footnote_sources(self):
        d = self.paper()
        (d / "evidence").mkdir()
        (d / "evidence" / "2026-09-29-portal.json").write_text("{}\n")
        cases = {
            "frozen evidence file": ("Frozen as `evidence/2026-09-29-portal.json` (sha256 `3f2a9c1b7e44`).", 0),
            "evidence file from the root": (f"See `docs/{d.name}/evidence/2026-09-29-portal.json`.", 0),
            "rollup with its id": ("Rollup 2026-09-29T11:14:31, key `totals.runs`.", 0),
            "missing evidence file": ("Frozen as `evidence/2026-09-30-nothing.json`.", 1),
            "templated evidence file": ("Frozen as `evidence/<date>-<slug>.json`.", 1),
        }
        for name, (note, want) in cases.items():
            with self.subTest(name):
                p = self.section(d, "01-introduction.md", "Status: draft · owner gap-aaaaaa\n\n# T\n\n"
                                 f"Roko built 16 plans for $174.87.[^1-p]\n\n[^1-p]: {note}\n")
                code, out = run("--strict", p)
                self.assertEqual(out.count("[number]"), want, out)

    def test_commit_spans(self):
        cases = {
            "in HEAD's history": (f"Fixed in `{self.sha}`.", 0),
            "off HEAD": (f"Fixed in `{self.side}` on a side branch.", 1),
            "unknown": ("Fixed in `abc1234ef`.", 1),
            "a sha256 prefix": ("Frozen (sha256 `3f2a9c1b7e44`).", 0),
            "a sha256 prefix on the next line": ("Frozen as `evidence/x.md` (sha256\n    `3f2a9c1`).", 0),
            "a full sha": (f"Fixed in `{'0' * 39}a`.", 1),
            "not hex enough": ("Numbers such as `1234567` and words such as `defaced` are not commits.", 0),
        }
        for name, (body, want) in cases.items():
            with self.subTest(name):
                p = self.section(self.paper(), "01-introduction.md",
                                 "Status: draft · owner gap-aaaaaa\n\n# T\n\n" + body)
                code, out = run("--check-identifiers", p)
                self.assertEqual(out.count("[identifier] commit"), want, out)

    def test_footnotes_and_citations_are_not_link_definitions(self):
        p = self.section(self.paper(), "01-introduction.md", """\
            Status: draft · owner gap-aaaaaa

            # T

            [@wiener1948]: feedback came first.

            A claim.[^n] Another.[^m]

            [^n]: Source: gap-aaaaaa.
            [^m]: `wc -l` over the tracked files.
            """)
        code, out = run("--report", p)
        self.assertNotIn("link", out.split("findings", 1)[1].split("\n", 1)[0], out)


if __name__ == "__main__":
    unittest.main(verbosity=1)
