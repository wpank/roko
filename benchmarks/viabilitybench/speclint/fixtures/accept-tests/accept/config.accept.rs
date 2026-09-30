// The planner's acceptance test for the retry limit; [task.accept] pins it and copies it to
// tests/config_accept.rs before running it.
use fixture::parse_config;

#[test]
fn missing_key_defaults_to_three() {
    assert_eq!(parse_config("").retry_limit, 3);
}

#[test]
fn reads_the_key() {
    assert_eq!(parse_config("retry_limit = 5").retry_limit, 5);
}

#[test]
fn rejects_zero() {
    assert!(std::panic::catch_unwind(|| parse_config("retry_limit = 0")).is_err());
}
