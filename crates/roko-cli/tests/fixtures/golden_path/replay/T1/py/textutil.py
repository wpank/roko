"""Text helpers for the golden-path seed."""

import re


def title_words(text):
    """Return `text` with each word capitalised and single spaces between them."""
    return " ".join(word.capitalize() for word in text.split())


def slugify(text):
    """Return `text` as a URL slug: lower case, words joined by single hyphens."""
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
