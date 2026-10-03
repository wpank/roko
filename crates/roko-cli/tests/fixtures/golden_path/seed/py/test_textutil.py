import unittest

import textutil


class TitleWordsTest(unittest.TestCase):
    def test_capitalises_each_word(self):
        self.assertEqual(textutil.title_words("hello wide world"), "Hello Wide World")

    def test_collapses_spaces(self):
        self.assertEqual(textutil.title_words("  a   b "), "A B")


if __name__ == "__main__":
    unittest.main()
