//! The config fingerprint (S01 §4.7): one hash rule shared by every run
//! manifest and bench record.
//!
//! `fingerprint = "b3:" + hex(blake3(canon(redact(config))))`, the full
//! 256-bit digest:
//!
//! - [`redact`] replaces the value of every key that is `secrets`, ends with
//!   `_key` or contains `token` (ASCII case-insensitive) with [`REDACTED`],
//!   and counts the values it replaced. A number or a boolean cannot be a
//!   secret, so it stays: `max_tokens` is a limit, and changing it changes the
//!   hash.
//! - [`canonical_json`] is RFC 8785 JSON canonicalization.
//!
//! Secrets never reach the hash, so two machines with different keys and the
//! same settings share a fingerprint. [`ConfigHash`](crate::metric::ConfigHash)
//! is the first 16 hex digits of the same digest, so the two never disagree.
//! Every implementation (this one and the S08 bench driver) reproduces the
//! golden vectors in `config_fingerprint_golden.json`, next to this file.

use std::cmp::Ordering;
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::{Number, Value};

use super::schema::RokoConfig;

/// What [`redact`] writes in place of a secret value.
pub const REDACTED: &str = "<redacted>";

/// The fingerprint of a config (S01 §4.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFingerprint {
    /// `"b3:"` plus the 64 hex digits of the BLAKE3 digest.
    pub hash: String,
    /// How many values [`redact`] replaced before hashing.
    pub redacted_keys: u32,
}

impl ConfigFingerprint {
    /// The 64 hex digits of the digest, without the `b3:` prefix.
    #[must_use]
    pub fn hex(&self) -> &str {
        self.hash.strip_prefix("b3:").unwrap_or(&self.hash)
    }
}

/// Fingerprint the effective config of a run.
///
/// # Errors
///
/// Returns an error when the config cannot be serialized to JSON.
pub fn fingerprint(config: &RokoConfig) -> Result<ConfigFingerprint, serde_json::Error> {
    fingerprint_of(config)
}

/// Fingerprint any serializable value by the config rule.
///
/// # Errors
///
/// Returns an error when `value` cannot be serialized to JSON.
pub fn fingerprint_of<T: Serialize + ?Sized>(
    value: &T,
) -> Result<ConfigFingerprint, serde_json::Error> {
    let mut value = serde_json::to_value(value)?;
    let redacted_keys = redact(&mut value);
    let digest = blake3::hash(canonical_json(&value).as_bytes());
    Ok(ConfigFingerprint {
        hash: format!("b3:{}", digest.to_hex()),
        redacted_keys,
    })
}

/// Whether `key` names a secret: it is `secrets`, ends with `_key` or
/// contains `token`, compared ASCII case-insensitively (`GITHUB_TOKEN` in an
/// environment map is one). `api_key_env` names a variable; it is not one.
#[must_use]
pub fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key == "secrets" || key.ends_with("_key") || key.contains("token")
}

/// Replace every secret value in `value` with [`REDACTED`] and return how
/// many were replaced. A secret key's value is replaced whole, whatever it
/// holds, unless it is a number or a boolean.
pub fn redact(value: &mut Value) -> u32 {
    match value {
        Value::Object(map) => map
            .iter_mut()
            .map(|(key, value)| {
                if is_secret_key(key) && !matches!(value, Value::Number(_) | Value::Bool(_)) {
                    *value = Value::String(REDACTED.to_string());
                    1
                } else {
                    redact(value)
                }
            })
            .sum(),
        Value::Array(items) => items.iter_mut().map(redact).sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}

/// RFC 8785 canonical JSON of `value`: object keys sorted by their UTF-16
/// code units, no insignificant whitespace, numbers as ECMAScript writes them
/// (`5.0` is `5`, `1e21` is `1e+21`), and only `"`, `\` and control
/// characters escaped. Integers are written exactly.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&canonical_number(number)),
        Value::String(text) => write_string(text, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|(a, _), (b, _)| utf16_order(a, b));
            out.push('{');
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_canonical(item, out);
            }
            out.push('}');
        }
    }
}

/// RFC 8785 orders object keys by UTF-16 code units, not by code points.
fn utf16_order(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// A JSON string with only `"`, `\` and control characters escaped, the
/// short forms where JSON has them and `\u00xx` (lowercase) otherwise.
fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if u32::from(ch) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(ch));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn canonical_number(number: &Number) -> String {
    if let Some(integer) = number.as_i64() {
        return integer.to_string();
    }
    if let Some(integer) = number.as_u64() {
        return integer.to_string();
    }
    number
        .as_f64()
        .map_or_else(|| number.to_string(), ecmascript_number)
}

/// `x` as ECMAScript's `Number.prototype.toString` writes it (RFC 8785
/// §3.2.2.3): the shortest digits that round-trip, positional from 1e-6 up
/// to 1e21 and in exponent form outside that range. Both zeros are `0`.
fn ecmascript_number(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        // JSON has no NaN or infinity, and serde_json never produces them.
        return "0".to_string();
    }
    // `{:e}` writes the shortest round-trip digits as `d[.ddd]e<exponent>`.
    let scientific = format!("{:e}", x.abs());
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return x.to_string();
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return x.to_string();
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i32;
    // x = 0.<digits> × 10^n
    let n = exponent + 1;
    let body = if (k..=21).contains(&n) {
        format!("{digits}{}", "0".repeat((n - k).unsigned_abs() as usize))
    } else if (1..=21).contains(&n) {
        let (integer, fraction) = digits.split_at(n.unsigned_abs() as usize);
        format!("{integer}.{fraction}")
    } else if (-5..=0).contains(&n) {
        format!("0.{}{digits}", "0".repeat(n.unsigned_abs() as usize))
    } else {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let sign = if n > 0 { '+' } else { '-' };
        format!("{first}{fraction}e{sign}{}", (n - 1).unsigned_abs())
    };
    if x < 0.0 { format!("-{body}") } else { body }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The golden vectors every fingerprint implementation reproduces. The
    /// hashes come from the reference `blake3` Python package, and each
    /// canonical form matches the `rfc8785` Python package.
    const GOLDEN: &str = include_str!("config_fingerprint_golden.json");

    #[test]
    fn config_fingerprint_is_order_independent_and_redacts_secrets() {
        let golden: Value = serde_json::from_str(GOLDEN).expect("parse the golden vectors");
        let cases = golden["cases"].as_array().expect("golden cases");
        assert_eq!(cases.len(), 5);
        for case in cases {
            let name = case["name"].as_str().unwrap_or_default();
            let mut redacted = case["input"].clone();
            let count = redact(&mut redacted);
            let canonical = case["canonical"].as_str().unwrap_or_default();
            assert_eq!(canonical_json(&redacted), canonical, "{name}");
            let fingerprint = fingerprint_of(&case["input"]).expect("fingerprint a case");
            assert_eq!(
                fingerprint.hash,
                case["hash"].as_str().unwrap_or_default(),
                "{name}"
            );
            let expected_redactions = case["redacted_keys"].as_u64().unwrap_or(u64::MAX);
            assert_eq!(u64::from(count), expected_redactions, "{name}");
            assert_eq!(fingerprint.redacted_keys, count, "{name}");
        }

        // Key order never matters.
        let forward: Value =
            serde_json::from_str(r#"{"a":1,"b":{"c":2,"d":[1,2]}}"#).expect("parse");
        let backward: Value =
            serde_json::from_str(r#"{"b":{"d":[1,2],"c":2},"a":1}"#).expect("parse");
        assert_eq!(
            fingerprint_of(&forward).expect("fingerprint"),
            fingerprint_of(&backward).expect("fingerprint")
        );

        // On a RokoConfig, a secret never changes the hash and a setting does.
        let config = RokoConfig::default();
        let base = fingerprint(&config).expect("fingerprint the default config");
        assert_eq!(base.hex().len(), 64, "{}", base.hash);
        assert!(base.redacted_keys > 0, "serve.auth.api_key is redacted");
        let mut keyed = config.clone();
        keyed.serve.auth.api_key = "sk-live-123".to_string();
        assert_eq!(fingerprint(&keyed).expect("fingerprint"), base);
        let mut budgeted = config.clone();
        budgeted.budget.max_plan_usd += 1.0;
        assert_ne!(fingerprint(&budgeted).expect("fingerprint").hash, base.hash);

        // The 16-digit ConfigHash is a prefix of the same digest.
        let short = crate::metric::ConfigHash::of(&config).expect("config hash");
        assert_eq!(short.as_str(), &base.hex()[..16]);
    }

    #[test]
    fn numbers_follow_the_ecmascript_form() {
        let cases = [
            (1.0, "1"),
            (5.5, "5.5"),
            (0.1, "0.1"),
            (123.0, "123"),
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (1.5e300, "1.5e+300"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (2.5e-7, "2.5e-7"),
            (-2.25, "-2.25"),
            (-0.0, "0"),
        ];
        for (x, expected) in cases {
            assert_eq!(ecmascript_number(x), expected, "{x:e}");
        }
    }
}
