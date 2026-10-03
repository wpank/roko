use alpha::initials;

#[test]
fn takes_the_first_letter_of_each_word() {
    assert_eq!(initials("ada lovelace"), "AL");
}

#[test]
fn ignores_extra_spaces() {
    assert_eq!(initials("  grace   brewster hopper "), "GBH");
}
