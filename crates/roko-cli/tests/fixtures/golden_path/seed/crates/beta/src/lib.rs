//! Greetings for the golden-path seed, built on `alpha`.

/// A greeting for `name`, shouted.
pub fn greeting(name: &str) -> String {
    format!("HELLO, {}!", alpha::shout(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_shouts_the_name() {
        assert_eq!(greeting("ada"), "HELLO, ADA!");
    }
}
