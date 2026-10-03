//! Text helpers for the golden-path seed.

/// `text` in upper case.
pub fn shout(text: &str) -> String {
    text.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shout_upper_cases_every_letter() {
        assert_eq!(shout("hi there"), "HI THERE");
    }
}
