#[test]
fn version_is_the_crate_version() {
    assert_eq!(alpha::VERSION, env!("CARGO_PKG_VERSION"));
}
