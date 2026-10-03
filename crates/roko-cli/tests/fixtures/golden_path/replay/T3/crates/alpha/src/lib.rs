//! Text helpers for the golden-path seed.

/// `text` in upper case.
pub fn shout(text: &str) -> String {
    text.to_uppercase()
}

/// The number of whitespace-separated words in `text`.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shout_upper_cases_every_letter() {
        assert_eq!(shout("hi there"), "HI THERE");
    }
}
