use alpha::word_count;

#[test]
fn counts_words_separated_by_spaces() {
    assert_eq!(word_count("cheap models execute"), 3);
}

#[test]
fn ignores_repeated_whitespace() {
    assert_eq!(word_count("  one \t two\n"), 2);
}

#[test]
fn counts_nothing_in_an_empty_string() {
    assert_eq!(word_count(""), 0);
}
