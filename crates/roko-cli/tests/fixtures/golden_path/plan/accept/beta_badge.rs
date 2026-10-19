use beta::badge;

#[test]
fn badge_leads_with_the_initials() {
    assert_eq!(badge("ada lovelace"), "[AL] ada lovelace");
}

#[test]
fn badge_trims_the_name() {
    assert_eq!(badge("  alan turing "), "[AT] alan turing");
}

#[test]
fn badge_upper_cases_non_ascii_initials() {
    assert_eq!(badge("émile zola"), "[ÉZ] émile zola");
}

#[test]
fn badge_keeps_every_letter_of_an_upper_cased_initial() {
    assert_eq!(badge("ßeta"), "[SS] ßeta");
}
