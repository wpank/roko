import unittest

import textutil


class SlugifyTest(unittest.TestCase):
    def test_lower_cases_and_joins_words(self):
        self.assertEqual(textutil.slugify("Hello World"), "hello-world")

    def test_collapses_punctuation_and_spaces(self):
        self.assertEqual(textutil.slugify("  Roko -- ships!  "), "roko-ships")

    def test_keeps_digits(self):
        self.assertEqual(textutil.slugify("Plan 2 of 3"), "plan-2-of-3")


if __name__ == "__main__":
    unittest.main()
