import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.docs_integrity import check_markdown_links as checker
from tools.docs_integrity.check_markdown_links import (
    REPO_ROOT,
    Limits,
    check_paths,
    github_slug,
)


class MarkdownLinkCheckerTests(unittest.TestCase):
    def fixture(self, files: dict[str, str]) -> tuple[tempfile.TemporaryDirectory, Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative, content in files.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        return temporary, root

    def test_valid_relative_paths_anchors_duplicates_and_unicode(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "[section](docs/guide.md#client--server)\n"
                    "[duplicate](docs/guide.md#repeat-1)\n"
                    "[same file](#home)\n"
                    "# Home\n"
                ),
                "docs/guide.md": "# Client → Server\n## Repeat\n## Repeat\n",
            }
        )
        self.addCleanup(temporary.cleanup)

        self.assertEqual(github_slug("Client → Server"), "client--server")
        self.assertEqual(check_paths(root, ["README.md", "docs"]), [])

    def test_github_slug_spaces_whitespace_emphasis_and_explicit_ids(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "[emphasis](guide.md#emphasis)\n"
                    "[first](guide.md#same)\n"
                    "[second](guide.md#same-1)\n"
                    "[whitespace](guide.md#a-bc)\n"
                    "[generated collision](guide.md#collision-2)\n"
                ),
                "guide.md": (
                    "<a id=\"same\"></a>\n"
                    "# _Emphasis_\n"
                    "# Same\n"
                    "# Same\n"
                    "# A B\tC\n"
                    "# Collision\n"
                    "# Collision-1\n"
                    "# Collision\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)

        self.assertEqual(github_slug("A B\tC\nD"), "a-bcd")
        self.assertEqual(check_paths(root, ["README.md", "guide.md"]), [])

    def test_commonmark_literal_underscores_code_spans_and_autolinks(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "[unmatched literal](commonmark.md#foo_bar_)\n"
                    "[code whitespace](commonmark.md#_literal_--code)\n"
                    "[autolink](commonmark.md#httpsexamplecom)\n"
                ),
                "commonmark.md": (
                    "# foo_bar_\n"
                    "# ` _literal_  code `\n"
                    "# <https://example.com>\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)

        self.assertEqual(check_paths(root, ["README.md", "commonmark.md"]), [])

    def test_lazy_anchor_targets_are_cached_and_do_not_evade_link_budget(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": "[one](target.md#target)\n[two](target.md#target)\n",
                "target.md": "# Target\n[ignored](elsewhere.md)\n[also ignored](other.md)\n",
            }
        )
        self.addCleanup(temporary.cleanup)
        limits = Limits(max_files=4, max_file_bytes=1024, max_total_bytes=4096, max_links=2)

        with mock.patch.object(
            checker, "_read_markdown", wraps=checker._read_markdown
        ) as read_markdown:
            findings = check_paths(root, ["README.md"], limits)

        self.assertEqual(findings, [])
        target = (root / "target.md").resolve()
        self.assertEqual(
            sum(call.args[0] == target for call in read_markdown.call_args_list), 1
        )

    def test_lazy_anchor_target_failure_is_cached(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": "[one](large.md#target)\n[two](large.md#target)\n",
                "large.md": "# Target\n" + ("x" * 128),
            }
        )
        self.addCleanup(temporary.cleanup)
        limits = Limits(max_files=4, max_file_bytes=100, max_total_bytes=4096, max_links=2)

        with mock.patch.object(
            checker, "_read_markdown", wraps=checker._read_markdown
        ) as read_markdown:
            findings = check_paths(root, ["README.md"], limits)

        self.assertEqual(len(findings), 2)
        target = (root / "large.md").resolve()
        self.assertEqual(
            sum(call.args[0] == target for call in read_markdown.call_args_list), 1
        )

    def test_selected_target_crossing_aggregate_cap_is_not_reread_lazily(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": "[one](target.md#target)\n[two](target.md#target)\n",
                "target.md": "# Target\n",
            }
        )
        self.addCleanup(temporary.cleanup)
        readme_size = (root / "README.md").stat().st_size
        target_size = (root / "target.md").stat().st_size
        limits = Limits(
            max_files=4,
            max_file_bytes=1024,
            max_total_bytes=readme_size + target_size - 1,
            max_links=2,
        )

        with mock.patch.object(
            checker, "_read_markdown", wraps=checker._read_markdown
        ) as read_markdown:
            findings = check_paths(root, ["README.md", "target.md"], limits)

        self.assertTrue(any("aggregate limit" in finding.message for finding in findings))
        target = (root / "target.md").resolve()
        self.assertEqual(
            sum(call.args[0] == target for call in read_markdown.call_args_list), 1
        )

    def test_reports_missing_file_and_missing_anchor(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "[missing file](docs/missing.md)\n"
                    "[missing anchor](docs/guide.md#absent)\n"
                ),
                "docs/guide.md": "# Present\n",
            }
        )
        self.addCleanup(temporary.cleanup)

        messages = [finding.message for finding in check_paths(root, ["README.md", "docs"])]
        self.assertTrue(any("target does not exist" in message for message in messages))
        self.assertTrue(any("anchor does not exist" in message for message in messages))

    def test_footnote_definitions_are_not_links(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "A claim.[^1] Another.[^note] A third.[^log]\n"
                    "\n"
                    "[^1]: Commit abc1234; see the log.\n"
                    "[^note]: Measured at `a17d4dadd`.\n"
                    "[^log]: See [the log](log.md).\n"
                    "[missing]: docs/missing.md\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)

        findings = check_paths(root, ["README.md"])

        self.assertEqual(
            [(finding.line, finding.message) for finding in findings],
            [
                (5, "local link target does not exist: log.md"),
                (6, "local link target does not exist: docs/missing.md"),
            ],
        )

    def test_ignores_external_links_and_fenced_or_inline_code(self) -> None:
        temporary, root = self.fixture(
            {
                "README.md": (
                    "[external](https://example.com/missing.md#nope)\n"
                    "`[inline](missing.md)`\n"
                    "```markdown\n[fenced](missing.md)\n```\n"
                )
            }
        )
        self.addCleanup(temporary.cleanup)

        self.assertEqual(check_paths(root, ["README.md"]), [])

    def test_bounds_are_early_and_deterministic(self) -> None:
        temporary, root = self.fixture(
            {
                "a.md": "[b](b.md)\n",
                "b.md": "# B\n",
            }
        )
        self.addCleanup(temporary.cleanup)
        limits = Limits(max_files=1, max_file_bytes=64, max_total_bytes=64, max_links=1)

        first = check_paths(root, ["."], limits)
        second = check_paths(root, ["."], limits)

        self.assertEqual(first, second)
        self.assertEqual(len(first), 1)
        self.assertIn("file count", first[0].message)

    def test_oversized_file_fails_without_reading_links(self) -> None:
        temporary, root = self.fixture({"README.md": "[missing](nope.md)\n"})
        self.addCleanup(temporary.cleanup)
        limits = Limits(max_files=4, max_file_bytes=8, max_total_bytes=64, max_links=4)

        findings = check_paths(root, ["README.md"], limits)

        self.assertEqual(len(findings), 1)
        self.assertIn("file size", findings[0].message)

    def test_ci_workflows_run_both_integrity_gates(self) -> None:
        docs_workflow = (REPO_ROOT / ".github/workflows/docs-lint.yml").read_text(
            encoding="utf-8"
        )
        plan_workflow = (REPO_ROOT / ".github/workflows/plan-validate.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            "python3 -m unittest tools.docs_integrity.test_check_markdown_links",
            docs_workflow,
        )
        self.assertIn("python3 tools/docs_integrity/check_markdown_links.py", docs_workflow)
        self.assertIn('".github/workflows/plan-validate.yml"', docs_workflow)
        self.assertIn("target/debug/roko plan index --check --workdir .", plan_workflow)
        self.assertIn('"plans/INDEX.md"', plan_workflow)
        self.assertIn('"plans/_meta/IMPLEMENTATION_ORDER.md"', plan_workflow)
        self.assertIn("executor.json mentions must be explicitly identified", docs_workflow)


if __name__ == "__main__":
    unittest.main()
