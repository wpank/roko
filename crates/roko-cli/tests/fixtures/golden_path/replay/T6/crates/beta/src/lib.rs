//! Greetings for the golden-path seed, built on `alpha`.

/// A greeting for `name`, shouted.
pub fn greeting(name: &str) -> String {
    format!("HELLO, {}!", alpha::shout(name))
}

/// A goodbye for `name`, trimmed and shouted.
pub fn farewell(name: &str) -> String {
    format!("GOODBYE, {}.", alpha::shout(name.trim()))
}

/// `name`, trimmed, after its initials in brackets.
pub fn badge(name: &str) -> String {
    format!("[{}] {}", alpha::initials(name), name.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_shouts_the_name() {
        assert_eq!(greeting("ada"), "HELLO, ADA!");
    }
}
