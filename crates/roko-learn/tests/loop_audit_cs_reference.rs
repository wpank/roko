//! The Rust betting confidence sequence against S08's Python toolkit
//! (backlog 5111). `fixtures/loop_audit_cs/widths.json` holds fixed-seed
//! increment sequences with their bounds; `reference.json` holds the widths
//! `benchmarks/viabilitybench/analysis/cs.py::confidence_sequence` gives on
//! them. The Python suite (`analysis/test_toolkit.py`) checks the same
//! reference, so a change on either side fails both.

use std::path::Path;

use roko_learn::loop_audit::cs::BettingCs;
use serde_json::Value;

/// The JSON fixture `name` under `tests/fixtures/loop_audit_cs`.
fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/loop_audit_cs")
        .join(name);
    let text = std::fs::read_to_string(&path).expect("a CS fixture");
    serde_json::from_str(&text).expect("parse a CS fixture")
}

#[test]
fn cs_widths_match_python_reference() {
    let input = fixture("widths.json");
    let reference = fixture("reference.json");
    let alpha = input["alpha"].as_f64().expect("alpha");
    let grid = input["grid"].as_u64().expect("grid");
    assert_eq!(reference["alpha"].as_f64(), Some(alpha));
    assert_eq!(reference["grid"].as_u64(), Some(grid));
    let sequences = input["sequences"].as_array().expect("sequences");
    let expected = reference["sequences"].as_array().expect("reference sequences");
    assert_eq!(sequences.len(), expected.len());
    for (sequence, python) in sequences.iter().zip(expected) {
        let name = sequence["name"].as_str().expect("a name");
        assert_eq!(python["name"].as_str(), Some(name));
        let bound = sequence["bound"].as_f64().expect("a bound");
        let values = sequence["values"].as_array().expect("values");
        let widths = python["widths"].as_array().expect("widths");
        assert_eq!(values.len(), widths.len(), "{name}");
        let grid = usize::try_from(grid).expect("a grid size");
        let mut cs = BettingCs::with_grid(alpha, -bound, bound, grid);
        for (step, (value, want)) in values.iter().zip(widths).enumerate() {
            cs.push(value.as_f64().expect("a value"), bound);
            match (cs.width(), want.as_f64()) {
                (Some(got), Some(want)) => assert!(
                    (got - want).abs() <= 1e-6 * want.abs().max(f64::MIN_POSITIVE),
                    "{name} step {step}: Rust {got}, Python {want}"
                ),
                (got, want) => assert!(
                    got.is_none() && want.is_none(),
                    "{name} step {step}: Rust {got:?}, Python {want:?}"
                ),
            }
        }
    }
}
