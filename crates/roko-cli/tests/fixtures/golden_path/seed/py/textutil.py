"""Text helpers for the golden-path seed."""


def title_words(text):
    """Return `text` with each word capitalised and single spaces between them."""
    return " ".join(word.capitalize() for word in text.split())
