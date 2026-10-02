"""Planner-written tests for the sq02-planner-test fixture's slugify task (named so that pytest does not collect them)."""

import unittest

from slug import slugify


class SlugifyTest(unittest.TestCase):
    def test_lowercases_and_joins_words(self):
        self.assertEqual(slugify("Hello World"), "hello-world")

    def test_collapses_and_trims_separators(self):
        self.assertEqual(slugify("  --Rust & Python!--  "), "rust-python")


if __name__ == "__main__":
    unittest.main()
