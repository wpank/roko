use beta::farewell;

#[test]
fn farewell_shouts_the_name() {
    assert_eq!(farewell("ada"), "GOODBYE, ADA.");
}

#[test]
fn farewell_trims_the_name() {
    assert_eq!(farewell("  bob "), "GOODBYE, BOB.");
}
