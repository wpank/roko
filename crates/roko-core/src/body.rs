//! Engram body — the typed payload carried by a signal.
//!
//! A signal's body can be JSON (structured data), bytes (binary), text (UTF-8),
//! or empty (marker signals). The body's format is explicit — consumers can
//! inspect `body.kind_hint()` before decoding.

use crate::error::{Result, RokoError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The payload carried by a [`Engram`](crate::Engram).
///
/// Bodies are tagged: consumers can tell at runtime whether the body is
/// structured JSON, raw bytes, text, or absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", content = "data", rename_all = "snake_case")]
pub enum Body {
    /// Empty body — the signal is purely a marker (its kind and tags carry meaning).
    Empty,
    /// UTF-8 text (logs, prompts, messages).
    Text(String),
    /// Structured JSON value.
    Json(serde_json::Value),
    /// Raw bytes (binary artifacts, compressed data).
    Bytes(#[serde(with = "base64_bytes")] Vec<u8>),
}

impl Body {
    /// Create an empty body.
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Create a text body.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    /// Serialize any serde-compatible type as the body.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::BodyEncode`] if serialization fails.
    pub fn from_json<T: Serialize>(value: &T) -> Result<Self> {
        let json = serde_json::to_value(value).map_err(RokoError::body_encode)?;
        Ok(Self::Json(json))
    }

    /// Create a bytes body.
    #[must_use]
    pub fn bytes(b: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(b.into())
    }

    /// Decode a JSON body into a typed value.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::BodyDecode`] if the body is not JSON, or if the
    /// JSON does not match the requested shape.
    pub fn as_json<T: DeserializeOwned>(&self) -> Result<T> {
        match self {
            Self::Json(v) => serde_json::from_value(v.clone()).map_err(RokoError::body_decode),
            other => Err(RokoError::BodyDecode(format!(
                "expected JSON body, got {}",
                other.kind_hint()
            ))),
        }
    }

    /// Get the text of a text body.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::BodyDecode`] if the body is not text.
    pub fn as_text(&self) -> Result<&str> {
        match self {
            Self::Text(s) => Ok(s.as_str()),
            other => Err(RokoError::BodyDecode(format!(
                "expected text body, got {}",
                other.kind_hint()
            ))),
        }
    }

    /// Get the bytes of a bytes body.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::BodyDecode`] if the body is not bytes.
    pub fn as_bytes(&self) -> Result<&[u8]> {
        match self {
            Self::Bytes(b) => Ok(b.as_slice()),
            other => Err(RokoError::BodyDecode(format!(
                "expected bytes body, got {}",
                other.kind_hint()
            ))),
        }
    }

    /// Human-readable hint of the body's format (for error messages).
    #[must_use]
    pub const fn kind_hint(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Text(_) => "text",
            Self::Json(_) => "json",
            Self::Bytes(_) => "bytes",
        }
    }

    /// Approximate size of the body's payload in bytes (for budget tracking).
    #[must_use]
    pub fn byte_size(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Text(s) => s.len(),
            Self::Json(v) => v.to_string().len(),
            Self::Bytes(b) => b.len(),
        }
    }

    /// Canonical byte encoding of the body (stable for content-hashing).
    ///
    /// Uses a JSON encoding for all body types so content hashes are stable
    /// across serde versions.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Stable: same content → same bytes → same hash.
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Serde support: encode/decode byte bodies as base64 so JSON stays valid UTF-8.
mod base64_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&encode_base64(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        decode_base64(&s).map_err(serde::de::Error::custom)
    }

    fn encode_base64(bytes: &[u8]) -> String {
        const CHARS: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        let chunks = bytes.chunks_exact(3);
        let remainder = chunks.remainder();
        for chunk in chunks {
            let n = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
            out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
            out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
            out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
            out.push(CHARS[(n & 0x3f) as usize] as char);
        }
        match remainder {
            [a] => {
                let n = u32::from(*a) << 16;
                out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
                out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
                out.push('=');
                out.push('=');
            }
            [a, b] => {
                let n = (u32::from(*a) << 16) | (u32::from(*b) << 8);
                out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
                out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
                out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
                out.push('=');
            }
            _ => {}
        }
        out
    }

    fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
        const fn val(b: u8) -> Option<u8> {
            match b {
                b'A'..=b'Z' => Some(b - b'A'),
                b'a'..=b'z' => Some(b - b'a' + 26),
                b'0'..=b'9' => Some(b - b'0' + 52),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        }
        let bytes = s.as_bytes();
        if bytes.len() % 4 != 0 {
            return Err("base64 length not multiple of 4".into());
        }
        let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
        for chunk in bytes.chunks_exact(4) {
            let mut vals = [0u8; 4];
            let mut pad = 0;
            for (i, &c) in chunk.iter().enumerate() {
                if c == b'=' {
                    pad += 1;
                    vals[i] = 0;
                } else {
                    vals[i] = val(c).ok_or_else(|| format!("invalid base64 byte: {c}"))?;
                }
            }
            let n = (u32::from(vals[0]) << 18)
                | (u32::from(vals[1]) << 12)
                | (u32::from(vals[2]) << 6)
                | u32::from(vals[3]);
            out.push(((n >> 16) & 0xff) as u8);
            if pad < 2 {
                out.push(((n >> 8) & 0xff) as u8);
            }
            if pad < 1 {
                out.push((n & 0xff) as u8);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_body_has_zero_size() {
        assert_eq!(Body::empty().byte_size(), 0);
    }

    #[test]
    fn text_body_roundtrip() {
        let b = Body::text("hello");
        assert_eq!(b.as_text().unwrap(), "hello");
    }

    #[test]
    fn decoding_wrong_type_errors() {
        let b = Body::text("hi");
        assert!(b.as_bytes().is_err());
        assert!(b.as_json::<serde_json::Value>().is_err());
    }

    #[test]
    fn json_body_typed_decode() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Thing {
            name: String,
            count: u32,
        }
        let orig = Thing {
            name: "x".into(),
            count: 7,
        };
        let body = Body::from_json(&orig).unwrap();
        let decoded: Thing = body.as_json().unwrap();
        assert_eq!(orig, decoded);
    }

    #[test]
    fn bytes_body_base64_roundtrip() {
        let data = vec![0u8, 1, 2, 3, 255, 128, 64];
        let body = Body::bytes(data.clone());
        let json = serde_json::to_string(&body).unwrap();
        let parsed: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, parsed);
        assert_eq!(parsed.as_bytes().unwrap(), data.as_slice());
    }

    #[test]
    fn canonical_bytes_stable() {
        let a = Body::text("stable");
        let b = Body::text("stable");
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn byte_size_reflects_content() {
        assert_eq!(Body::text("12345").byte_size(), 5);
        assert_eq!(Body::bytes(vec![0, 1, 2]).byte_size(), 3);
    }

    // ---- from_json serialization roundtrip ----

    #[test]
    fn from_json_serde_roundtrip() {
        // Body::from_json → serialize to JSON string → deserialize back → still equal
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Payload {
            id: u64,
            tags: Vec<String>,
        }
        let orig = Payload {
            id: 42,
            tags: vec!["a".into(), "b".into()],
        };
        let body = Body::from_json(&orig).unwrap();
        let json_str = serde_json::to_string(&body).unwrap();
        let restored: Body = serde_json::from_str(&json_str).unwrap();
        assert_eq!(body, restored);
        // And the typed decode still works after the roundtrip
        let decoded: Payload = restored.as_json().unwrap();
        assert_eq!(decoded, orig);
    }

    #[test]
    fn from_json_primitive_types() {
        // Primitives: number, bool, string, null
        let num = Body::from_json(&123_i64).unwrap();
        assert_eq!(num.as_json::<i64>().unwrap(), 123);

        let b = Body::from_json(&true).unwrap();
        assert_eq!(b.as_json::<bool>().unwrap(), true);

        let s = Body::from_json(&"hello").unwrap();
        assert_eq!(s.as_json::<String>().unwrap(), "hello");

        let n = Body::from_json(&()).unwrap();
        assert_eq!(n.kind_hint(), "json");
    }

    // ---- as_json accessor ----

    #[test]
    fn as_json_on_text_body_errors() {
        let b = Body::text("not json");
        let err = b.as_json::<serde_json::Value>().unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("text"),
            "error should mention 'text', got: {msg}"
        );
    }

    #[test]
    fn as_json_on_bytes_body_errors() {
        let b = Body::bytes(vec![1, 2, 3]);
        assert!(b.as_json::<serde_json::Value>().is_err());
    }

    #[test]
    fn as_json_on_empty_body_errors() {
        let b = Body::empty();
        assert!(b.as_json::<serde_json::Value>().is_err());
    }

    #[test]
    fn as_json_type_mismatch_errors() {
        // JSON body exists but shape doesn't match target type
        let body = Body::from_json(&serde_json::json!({"x": 1})).unwrap();
        let result = body.as_json::<Vec<String>>();
        assert!(result.is_err());
    }

    // ---- as_text accessor ----

    #[test]
    fn as_text_on_json_body_errors() {
        let b = Body::from_json(&42).unwrap();
        let err = b.as_text().unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("json"),
            "error should mention 'json', got: {msg}"
        );
    }

    #[test]
    fn as_text_on_bytes_body_errors() {
        let b = Body::bytes(vec![0xff]);
        assert!(b.as_text().is_err());
    }

    #[test]
    fn as_text_on_empty_body_errors() {
        let b = Body::empty();
        assert!(b.as_text().is_err());
    }

    // ---- as_bytes accessor ----

    #[test]
    fn as_bytes_on_json_body_errors() {
        let b = Body::from_json(&"nope").unwrap();
        assert!(b.as_bytes().is_err());
    }

    #[test]
    fn as_bytes_on_text_body_errors() {
        let b = Body::text("nope");
        assert!(b.as_bytes().is_err());
    }

    #[test]
    fn as_bytes_on_empty_body_errors() {
        let b = Body::empty();
        assert!(b.as_bytes().is_err());
    }

    // ---- canonical_bytes stability across all variants ----

    #[test]
    fn canonical_bytes_empty_stable() {
        let a = Body::empty();
        let b = Body::empty();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert!(
            !a.canonical_bytes().is_empty(),
            "canonical_bytes should produce non-empty output even for Empty"
        );
    }

    #[test]
    fn canonical_bytes_json_stable() {
        let a = Body::from_json(&serde_json::json!({"k": "v"})).unwrap();
        let b = Body::from_json(&serde_json::json!({"k": "v"})).unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_bytes_stable() {
        let a = Body::bytes(vec![10, 20, 30]);
        let b = Body::bytes(vec![10, 20, 30]);
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_differ_across_variants() {
        // Different variants with "same-ish" content should produce different hashes
        let text = Body::text("hello");
        let json = Body::from_json(&"hello").unwrap();
        let bytes = Body::bytes(b"hello".to_vec());
        let empty = Body::empty();

        // All four should be distinct
        let ctext = text.canonical_bytes();
        let cjson = json.canonical_bytes();
        let cbytes = bytes.canonical_bytes();
        let cempty = empty.canonical_bytes();

        assert_ne!(ctext, cjson);
        assert_ne!(ctext, cbytes);
        assert_ne!(ctext, cempty);
        assert_ne!(cjson, cbytes);
        assert_ne!(cjson, cempty);
        assert_ne!(cbytes, cempty);
    }

    // ---- All 4 variants: construction + kind_hint ----

    #[test]
    fn kind_hint_empty() {
        assert_eq!(Body::empty().kind_hint(), "empty");
    }

    #[test]
    fn kind_hint_text() {
        assert_eq!(Body::text("x").kind_hint(), "text");
    }

    #[test]
    fn kind_hint_json() {
        assert_eq!(Body::from_json(&1).unwrap().kind_hint(), "json");
    }

    #[test]
    fn kind_hint_bytes() {
        assert_eq!(Body::bytes(vec![]).kind_hint(), "bytes");
    }

    // ---- Serde roundtrip for every variant ----

    #[test]
    fn serde_roundtrip_empty() {
        let body = Body::empty();
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn serde_roundtrip_text() {
        let body = Body::text("round we go");
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn serde_roundtrip_json() {
        let body = Body::from_json(&serde_json::json!({"nested": [1, 2, 3]})).unwrap();
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn serde_roundtrip_bytes() {
        let body = Body::bytes(vec![0, 127, 255]);
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    // ---- Edge cases ----

    #[test]
    fn empty_string_text() {
        let body = Body::text("");
        assert_eq!(body.as_text().unwrap(), "");
        assert_eq!(body.byte_size(), 0);
        assert_eq!(body.kind_hint(), "text");
        // Roundtrip
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn empty_bytes() {
        let body = Body::bytes(vec![]);
        assert_eq!(body.as_bytes().unwrap(), &[] as &[u8]);
        assert_eq!(body.byte_size(), 0);
        assert_eq!(body.kind_hint(), "bytes");
        // Roundtrip
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn empty_text_differs_from_empty_variant() {
        let text = Body::text("");
        let empty = Body::empty();
        assert_ne!(text, empty);
        assert_ne!(text.canonical_bytes(), empty.canonical_bytes());
    }

    #[test]
    fn empty_bytes_differs_from_empty_variant() {
        let bytes = Body::bytes(vec![]);
        let empty = Body::empty();
        assert_ne!(bytes, empty);
        assert_ne!(bytes.canonical_bytes(), empty.canonical_bytes());
    }

    #[test]
    fn nested_json_roundtrip() {
        let nested = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": [1, "two", null, true, {"level4": []}]
                }
            },
            "siblings": [{"a": 1}, {"b": 2}]
        });
        let body = Body::from_json(&nested).unwrap();
        let decoded: serde_json::Value = body.as_json().unwrap();
        assert_eq!(decoded, nested);
        // Serde roundtrip
        let json_str = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json_str).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn json_null_body() {
        let body = Body::from_json(&serde_json::Value::Null).unwrap();
        let decoded: serde_json::Value = body.as_json().unwrap();
        assert_eq!(decoded, serde_json::Value::Null);
    }

    #[test]
    fn json_empty_object() {
        let body = Body::from_json(&serde_json::json!({})).unwrap();
        let decoded: serde_json::Value = body.as_json().unwrap();
        assert_eq!(decoded, serde_json::json!({}));
    }

    #[test]
    fn json_empty_array() {
        let body = Body::from_json(&serde_json::json!([])).unwrap();
        let decoded: serde_json::Value = body.as_json().unwrap();
        assert_eq!(decoded, serde_json::json!([]));
    }

    #[test]
    fn large_bytes_roundtrip() {
        // 1024 bytes — exercises multi-chunk base64 encoding
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let body = Body::bytes(data.clone());
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_bytes().unwrap(), data.as_slice());
    }

    #[test]
    fn text_with_unicode() {
        let text = "hello \u{1F600} world \u{00E9}\u{4E16}\u{754C}";
        let body = Body::text(text);
        assert_eq!(body.as_text().unwrap(), text);
        // Serde roundtrip preserves unicode
        let json = serde_json::to_string(&body).unwrap();
        let back: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_text().unwrap(), text);
    }

    #[test]
    fn canonical_bytes_deterministic_across_calls() {
        let body = Body::from_json(&serde_json::json!({"a": 1, "b": [2, 3]})).unwrap();
        let first = body.canonical_bytes();
        let second = body.canonical_bytes();
        let third = body.canonical_bytes();
        assert_eq!(first, second);
        assert_eq!(second, third);
    }
}
