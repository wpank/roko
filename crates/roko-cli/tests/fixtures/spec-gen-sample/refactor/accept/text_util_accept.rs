use roko_cli::text_util::truncate_chars;

#[test]
fn truncates_longer_text_with_ellipsis() {
    assert_eq!(truncate_chars("abcdef", 3), "abc...");
}

#[test]
fn keeps_text_at_exact_limit_unchanged() {
    assert_eq!(truncate_chars("abc", 3), "abc");
}

#[test]
fn empty_text_with_zero_limit_is_empty() {
    assert_eq!(truncate_chars("", 0), "");
}

#[test]
fn zero_limit_on_nonempty_text_is_only_ellipsis() {
    assert_eq!(truncate_chars("a", 0), "...");
}

#[test]
fn counts_multibyte_chars_not_bytes() {
    assert_eq!(truncate_chars("héllo wörld", 2), "hé...");
}
