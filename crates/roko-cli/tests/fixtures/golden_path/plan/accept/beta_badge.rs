use beta::badge;

#[test]
fn badge_leads_with_the_initials() {
    assert_eq!(badge("ada lovelace"), "[AL] ada lovelace");
}

#[test]
fn badge_trims_the_name() {
    assert_eq!(badge("  alan turing "), "[AT] alan turing");
}
