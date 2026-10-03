import unittest

import textutil


class WordListTest(unittest.TestCase):
    def test_splits_on_any_whitespace(self):
        self.assertEqual(textutil.word_list(" one\ttwo  three\n"), ["one", "two", "three"])

    def test_empty_text_has_no_words(self):
        self.assertEqual(textutil.word_list(""), [])


if __name__ == "__main__":
    unittest.main()
