import re
import unittest
from pathlib import Path

DOC = Path(__file__).resolve().parents[2] / "docs" / "v3" / "21-CONFIG.md"
HEADER_RE = re.compile(r"^\|\s*Key\s*\|\s*Type\s*\|\s*Default\s*\|\s*Effect\s*\|\s*$", re.M)


def doc_text():
    return DOC.read_text(encoding="utf-8")


def subsection(name):
    text = doc_text()
    heading = "### `[" + name + "]`"
    start = text.find(heading)
    if start < 0:
        raise AssertionError("missing subsection heading: " + heading)
    rest = text[start + len(heading):]
    m = re.search(r"^#{2,3} ", rest, re.M)
    return rest[: m.start()] if m else rest


def row(body, key):
    for line in body.splitlines():
        if line.strip().startswith("| `" + key + "`"):
            return line
    raise AssertionError("missing table row for key: " + key)


def cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


class ConfigDocSections(unittest.TestCase):
    def test_section_numbering(self):
        text = doc_text()
        headings = [
            "## 12. Plan Authoring and Spec Quality",
            "## 13. Minimal Config",
            "## 14. Verification",
            "## 15. References",
        ]
        positions = []
        for h in headings:
            m = re.search("^" + re.escape(h) + r"\s*$", text, re.M)
            self.assertIsNotNone(m, "missing heading: " + h)
            positions.append(m.start())
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(len(re.findall(r"^## 12\. ", text, re.M)), 1)
        new_section = text[positions[0]:positions[1]]
        self.assertIn("### `[authoring]`", new_section)
        self.assertIn("### `[spec_quality]`", new_section)

    def test_authoring_table(self):
        body = subsection("authoring")
        self.assertRegex(body, HEADER_RE)
        c = cells(row(body, "planner_model"))
        self.assertGreaterEqual(len(c), 4)
        self.assertIn("string", c[1].lower())
        self.assertIn('""', c[2])

    def test_spec_quality_table(self):
        body = subsection("spec_quality")
        self.assertRegex(body, HEADER_RE)
        expected = {
            "mode": "advise",
            "allow_threshold": "70",
            "block_threshold": "40",
            "red_on_base": "true",
            "red_on_base_cargo": "false",
            "red_on_base_timeout_secs": "120",
            "holdout_frac": "0.05",
        }
        for key, default in expected.items():
            c = cells(row(body, key))
            self.assertGreaterEqual(len(c), 4, key)
            self.assertIn(default, c[2], key)
        mode_row = row(body, "mode")
        self.assertIn("off", mode_row)
        self.assertIn("enforce", mode_row)

    def test_toml_examples(self):
        for name in ("authoring", "spec_quality"):
            body = subsection(name)
            m = re.search(r"^```toml\s*\n\s*\[" + name + r"\]\s*$", body, re.M)
            self.assertIsNotNone(m, "missing toml example starting with [" + name + "]")


if __name__ == "__main__":
    unittest.main()
