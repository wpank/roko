import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.docs_integrity import check_markdown_links as checker
from tools.docs_integrity.check_markdown_links import (
    CoverageLedgerRow,
    CoverageLedgerSpec,
    REPO_ROOT,
    Limits,
    SourceManifestRow,
    check_paths,
    check_status_disposition_registry,
    github_slug,
    parse_coverage_ledger,
    parse_doc_plan,
    parse_source_manifest,
    validate_source_manifest,
    validate_source_ownership,
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

    def test_status_disposition_registry_enforces_exact_109_file_rule(self) -> None:
        registry = """# Index
> **Disposition: CURRENT NAVIGATION (test).**
The 109 direct Markdown files under `tmp/status-quo/` are classified.
| `00-INDEX.md` | 1 | `CURRENT-NAVIGATION` |
| `MASTER-EXECUTION-CHECKLIST.md` | 1 | `CURRENT-CONTROL` |
| `DOC-MANIFEST.md` | 1 | `GENERATED-HISTORICAL` |
| Numbered `01-*.md` through `106-*.md` | 106 | `HISTORICAL-AUDIT` |
| **Total** | **109** | all classified |
"""
        files = {
            "tmp/status-quo/00-INDEX.md": registry,
            "tmp/status-quo/DOC-MANIFEST.md": (
                "# Manifest\n> **Disposition: GENERATED-HISTORICAL.**\n"
            ),
            "tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md": (
                "# Control\n> Status: active control document\n"
            ),
        }
        for number in range(1, 107):
            files[f"tmp/status-quo/{number:02d}-DOC.md"] = f"# Historical {number}\n"
        temporary, root = self.fixture(files)
        self.addCleanup(temporary.cleanup)

        self.assertEqual(check_status_disposition_registry(root), [])
        (root / "tmp/status-quo/EXTRA.md").write_text("# Extra\n", encoding="utf-8")
        findings = check_status_disposition_registry(root)
        self.assertTrue(any("expected 109" in finding.message for finding in findings))
        self.assertTrue(any("unclassified" in finding.message for finding in findings))

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
        self.assertIn('"tmp/status-quo/*.md"', docs_workflow)
        self.assertIn('"tmp/status-quo/backlog/source-coverage/**"', docs_workflow)
        self.assertIn('".github/workflows/plan-validate.yml"', docs_workflow)
        self.assertIn("target/debug/roko plan index --check --workdir .", plan_workflow)
        self.assertIn('"plans/INDEX.md"', plan_workflow)
        self.assertIn('"plans/_meta/IMPLEMENTATION_ORDER.md"', plan_workflow)
        self.assertIn("executor.json mentions must be explicitly identified", docs_workflow)

    def test_source_manifest_parser_is_exact_and_reports_malformed_rows(self) -> None:
        rows, malformed = parse_source_manifest(
            "| Source path | Title | Status tag | Suggested owner |\n"
            "| `docs/v1/a.md` | A | `legacy-design-or-partial` | `30-CORE-SIGNAL.md` |\n"
            "| `docs/v2/b.md` | B | `migration-design` | `31-GRAPH-CELLS-ENGINE.md` |\n"
            "| `docs/v2/not-markdown.txt` | Bad | `migration-design` | `31-GRAPH-CELLS-ENGINE.md` |\n"
        )

        self.assertEqual(
            rows,
            [
                SourceManifestRow("docs/v1/a.md", "30-CORE-SIGNAL.md"),
                SourceManifestRow("docs/v2/b.md", "31-GRAPH-CELLS-ENGINE.md"),
            ],
        )
        self.assertEqual(
            malformed,
            [
                "| `docs/v2/not-markdown.txt` | Bad | `migration-design` | `31-GRAPH-CELLS-ENGINE.md` |"
            ],
        )

    def test_coverage_ledger_and_named_task_ownership_are_exact(self) -> None:
        spec = CoverageLedgerSpec(
            "ledger.md", ("Source", "Owner"), 0, 1, "DOC-plan"
        )
        rows, errors = parse_coverage_ledger(
            "| Source | Owner |\n"
            "|---|---|\n"
            "| `docs/v2/a.md` | `DOC-A` |\n"
            "| `docs/v2/b.md` | `DOC-B` |\n",
            spec,
        )
        contexts, plan_errors = parse_doc_plan(
            b'[[task]]\nid = "DOC-A"\n[task.context]\nread_files = [{ path = "docs/v2/a.md" }]\n\n'
            b'[[task]]\nid = "DOC-B"\n[task.context]\nread_files = [{ path = "docs/v2/b.md" }]\n'
        )

        self.assertEqual(errors, [])
        self.assertEqual(plan_errors, [])
        self.assertEqual(
            validate_source_ownership(
                {"docs/v2/a.md", "docs/v2/b.md"},
                [(spec.plan_directory, row) for row in rows],
                {spec.plan_directory: contexts},
            ),
            [],
        )

    def test_coverage_ownership_mutations_reject_duplicate_extra_missing_and_wrong_owner(self) -> None:
        rows = [
            ("DOC-plan", CoverageLedgerRow("docs/v2/a.md", "DOC-A", 3)),
            ("DOC-plan", CoverageLedgerRow("docs/v2/a.md", "DOC-B", 4)),
            ("DOC-plan", CoverageLedgerRow("docs/v2/extra.md", "DOC-B", 5)),
        ]
        contexts = {
            "DOC-plan": {
                "DOC-A": frozenset({"docs/v2/wrong.md"}),
                "DOC-B": frozenset({"docs/v2/extra.md"}),
            }
        }

        errors = validate_source_ownership(
            {"docs/v2/a.md", "docs/v2/missing.md"}, rows, contexts
        )

        self.assertTrue(any("duplicate ledger source" in error for error in errors))
        self.assertTrue(any("missing from coverage ledgers" in error for error in errors))
        self.assertTrue(any("extra source" in error for error in errors))
        self.assertTrue(any("owner context omits source" in error for error in errors))

    def test_doc_plan_parser_rejects_duplicate_task_ids(self) -> None:
        _, errors = parse_doc_plan(
            b'[[task]]\nid = "DOC-A"\n[[task]]\nid = "DOC-A"\n'
        )

        self.assertTrue(any("duplicate task id" in error for error in errors))

    def test_source_manifest_mutations_reject_membership_and_unresolved_owner(self) -> None:
        errors = validate_source_manifest(
            {"docs/v2/a.md", "docs/v2/missing.md"},
            [
                SourceManifestRow("docs/v2/a.md", "30-CORE-SIGNAL.md"),
                SourceManifestRow("docs/v2/a.md", "MISSING.md"),
                SourceManifestRow("docs/v2/extra.md", "30-CORE-SIGNAL.md"),
            ],
            {"30-CORE-SIGNAL.md"},
        )

        self.assertTrue(any("duplicate row" in error for error in errors))
        self.assertTrue(any("missing from manifest" in error for error in errors))
        self.assertTrue(any("does not exist" in error for error in errors))
        self.assertTrue(any("owner does not resolve" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
