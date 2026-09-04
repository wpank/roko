//! Canonical single-unit duration parser.
//!
//! Accepted input: a trimmed positive base-10 integer followed immediately by
//! one lowercase unit (`ms`, `s`, `m`, `h`, or `d`). Zero, signs, decimals,
//! whitespace between value and unit, uppercase units, missing units, unknown
//! suffixes, compound values such as `1d12h`, and millisecond overflow are errors.

use std::fmt;
use std::time::Duration;

/// Error returned when a duration string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationParseError {
    /// Input was empty or whitespace-only.
    Empty,
    /// No digits found before the unit suffix.
    MissingValue,
    /// The numeric portion could not be parsed as a positive integer.
    InvalidNumber,
    /// The value was zero.
    Zero,
    /// No unit suffix was found after the numeric portion.
    MissingUnit,
    /// The unit suffix is not one of `ms`, `s`, `m`, `h`, or `d`.
    UnknownUnit,
    /// The resulting millisecond value overflows `u64`.
    Overflow,
}

impl fmt::Display for DurationParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "duration string is empty"),
            Self::MissingValue => write!(f, "duration requires a positive integer before the unit"),
            Self::InvalidNumber => {
                write!(
                    f,
                    "duration must start with a positive integer (no signs, decimals, or whitespace)"
                )
            }
            Self::Zero => write!(f, "duration must be greater than zero"),
            Self::MissingUnit => {
                write!(f, "duration requires a unit suffix: ms, s, m, h, or d")
            }
            Self::UnknownUnit => {
                write!(f, "duration unit must be one of: ms, s, m, h, or d")
            }
            Self::Overflow => write!(f, "duration value is too large"),
        }
    }
}

impl std::error::Error for DurationParseError {}

/// Parse a single-unit duration string into a [`Duration`].
///
/// Accepted grammar: `<positive-integer><unit>` where unit is one of
/// `ms`, `s`, `m`, `h`, or `d`.
///
/// # Errors
///
/// Returns [`DurationParseError`] for any input that does not exactly match
/// the accepted grammar.
pub fn parse_duration(value: &str) -> Result<Duration, DurationParseError> {
    parse_duration_ms(value).map(Duration::from_millis)
}

/// Parse a single-unit duration string into milliseconds.
///
/// Same grammar as [`parse_duration`] but returns raw `u64` milliseconds.
pub fn parse_duration_ms(value: &str) -> Result<u64, DurationParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DurationParseError::Empty);
    }

    // Find the boundary between digits and the unit suffix.
    let split = value
        .find(|c: char| !c.is_ascii_digit())
        .ok_or(DurationParseError::MissingUnit)?;

    if split == 0 {
        return Err(DurationParseError::MissingValue);
    }

    let (num_str, unit) = value.split_at(split);

    let amount: u64 = num_str
        .parse()
        .map_err(|_| DurationParseError::InvalidNumber)?;
    if amount == 0 {
        return Err(DurationParseError::Zero);
    }

    let multiplier: u64 = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err(DurationParseError::UnknownUnit),
    };

    amount
        .checked_mul(multiplier)
        .ok_or(DurationParseError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_accepts_all_units() {
        assert_eq!(parse_duration_ms("1ms").unwrap(), 1);
        assert_eq!(parse_duration_ms("250ms").unwrap(), 250);
        assert_eq!(parse_duration_ms("1s").unwrap(), 1_000);
        assert_eq!(parse_duration_ms("60s").unwrap(), 60_000);
        assert_eq!(parse_duration_ms("1m").unwrap(), 60_000);
        assert_eq!(parse_duration_ms("2m").unwrap(), 120_000);
        assert_eq!(parse_duration_ms("1h").unwrap(), 3_600_000);
        assert_eq!(parse_duration_ms("3h").unwrap(), 10_800_000);
        assert_eq!(parse_duration_ms("1d").unwrap(), 86_400_000);
        assert_eq!(parse_duration_ms("30d").unwrap(), 2_592_000_000);
    }

    #[test]
    fn parse_duration_returns_std_duration() {
        assert_eq!(
            parse_duration("7d").unwrap(),
            Duration::from_millis(604_800_000)
        );
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn parse_duration_rejects_empty() {
        assert_eq!(parse_duration_ms(""), Err(DurationParseError::Empty));
        assert_eq!(parse_duration_ms("   "), Err(DurationParseError::Empty));
    }

    #[test]
    fn parse_duration_rejects_zero() {
        assert_eq!(parse_duration_ms("0s"), Err(DurationParseError::Zero));
        assert_eq!(parse_duration_ms("0d"), Err(DurationParseError::Zero));
        assert_eq!(parse_duration_ms("0ms"), Err(DurationParseError::Zero));
    }

    #[test]
    fn parse_duration_rejects_missing_unit() {
        assert_eq!(
            parse_duration_ms("60"),
            Err(DurationParseError::MissingUnit)
        );
        assert_eq!(
            parse_duration_ms("100"),
            Err(DurationParseError::MissingUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_unknown_unit() {
        assert_eq!(
            parse_duration_ms("1fortnight"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("30seconds"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("1w"),
            Err(DurationParseError::UnknownUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_compound() {
        // "1d12h" — digits stop at 'd', unit becomes "d12h" which is unknown.
        assert_eq!(
            parse_duration_ms("1d12h"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("2h30m"),
            Err(DurationParseError::UnknownUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_uppercase() {
        assert_eq!(
            parse_duration_ms("1D"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("1H"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("1S"),
            Err(DurationParseError::UnknownUnit)
        );
        assert_eq!(
            parse_duration_ms("1MS"),
            Err(DurationParseError::UnknownUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_signs_and_decimals() {
        assert_eq!(
            parse_duration_ms("-1s"),
            Err(DurationParseError::MissingValue)
        );
        assert_eq!(
            parse_duration_ms("+1s"),
            Err(DurationParseError::MissingValue)
        );
        assert_eq!(
            parse_duration_ms("1.5s"),
            Err(DurationParseError::UnknownUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_embedded_whitespace() {
        // Space between value and unit: digits stop at ' ', unit becomes " s".
        assert_eq!(
            parse_duration_ms("1 s"),
            Err(DurationParseError::UnknownUnit)
        );
    }

    #[test]
    fn parse_duration_rejects_overflow() {
        assert_eq!(
            parse_duration_ms("18446744073709551615d"),
            Err(DurationParseError::Overflow)
        );
    }

    #[test]
    fn parse_duration_trims_outer_whitespace() {
        assert_eq!(parse_duration_ms("  30d  ").unwrap(), 2_592_000_000);
    }

    #[test]
    fn parse_duration_rejects_trailing_text() {
        // "30dfoo" — unit becomes "dfoo" which is unknown.
        assert_eq!(
            parse_duration_ms("30dfoo"),
            Err(DurationParseError::UnknownUnit)
        );
    }
}
