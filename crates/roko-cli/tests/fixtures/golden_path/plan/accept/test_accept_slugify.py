import inspect
import unittest

import textutil

# `title_words` as the seed has it. The task keeps it as it is, docstring
# included.
SEED_TITLE_WORDS = '''def title_words(text):
    """Return `text` with each word capitalised and single spaces between them."""
    return " ".join(word.capitalize() for word in text.split())
'''


class SlugifyTest(unittest.TestCase):
    def test_lower_cases_and_joins_words(self):
        self.assertEqual(textutil.slugify("Hello World"), "hello-world")

    def test_collapses_punctuation_and_spaces(self):
        self.assertEqual(textutil.slugify("  Roko -- ships!  "), "roko-ships")

    def test_keeps_digits(self):
        self.assertEqual(textutil.slugify("Plan 2 of 3"), "plan-2-of-3")


class TitleWordsKeptTest(unittest.TestCase):
    maxDiff = None

    def test_title_words_is_kept_as_it_is(self):
        self.assertEqual(inspect.getsource(textutil.title_words), SEED_TITLE_WORDS)


if __name__ == "__main__":
    unittest.main()
