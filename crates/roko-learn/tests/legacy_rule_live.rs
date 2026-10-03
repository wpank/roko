//! The live `PromptExperiment` still reproduces the captured legacy
//! experiment rule (backlog 5109), through its public API alone. Backlog
//! 5118 deletes this file when it replaces the rule; `loop_audit::sim`'s
//! `LegacyRule` keeps the captured behaviour runnable after that.

use std::path::Path;

use roko_learn::loop_audit::sim::{LegacyFixture, SplitMix64};
use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

/// The live rule on one A/A sequence: `(outcomes recorded, winner)` when
/// `record_outcome` reports a conclusion within `horizon` outcomes.
fn live_aa_run(seed: u64, arms: usize, p: f64, horizon: u64) -> Option<(u64, String)> {
    let variants = (0..arms)
        .map(|arm| PromptVariant {
            id: format!("v{arm}"),
            name: format!("v{arm}"),
            section_name: "aa".to_string(),
            content: format!("v{arm}"),
            slug: None,
            active: true,
        })
        .collect();
    let mut experiment = PromptExperiment::new("legacy-aa", "aa", variants);
    let mut outcomes = SplitMix64::new(seed);
    for trial in 1..=horizon {
        let variant = experiment
            .assign_variant()
            .expect("a running experiment assigns a variant")
            .id
            .clone();
        if experiment.record_outcome(&variant, outcomes.next_f64() < p) {
            return experiment.winner_id.clone().map(|winner| (trial, winner));
        }
    }
    None
}

#[test]
fn live_prompt_experiment_reproduces_the_legacy_fixture() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy_experiment_rule.json");
    let text = std::fs::read_to_string(path).expect("the legacy-rule fixture");
    let fixture: LegacyFixture = serde_json::from_str(&text).expect("parse the fixture");
    let mut winners = 0_u32;
    for sequence in &fixture.sequences {
        let got = live_aa_run(sequence.seed, sequence.arms, sequence.p, fixture.horizon);
        assert_eq!(got, sequence.outcome(), "seed {}", sequence.seed);
        winners += u32::from(got.is_some());
    }
    println!(
        "live PromptExperiment: A/A false-winner rate {:.3} over {} sequences",
        f64::from(winners) / fixture.sequences.len() as f64,
        fixture.sequences.len()
    );
}
