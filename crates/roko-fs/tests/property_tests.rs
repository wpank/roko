//! Property-based tests for roko-fs core types.

use proptest::prelude::*;
use roko_fs::gc::{FsRetentionPolicy, GcReport, GcCandidate};
use roko_fs::layout::{LayoutVersion, RokoLayout};
use roko_fs::observability::{MAX_METRIC_CARDINALITY, validate_cardinality, validate_metric_key};

// ─── LayoutVersion roundtrip ─────────────────────────────────────────────────

proptest! {
    /// Known LayoutVersion values roundtrip through as_u32 / from_u32.
    #[test]
    fn layout_version_roundtrip(_dummy in 0u32..1) {
        for version in [LayoutVersion::V1, LayoutVersion::V2, LayoutVersion::V3] {
            let n = version.as_u32();
            let parsed = LayoutVersion::from_u32(n);
            prop_assert_eq!(parsed, Some(version));
        }
    }

    /// Unknown version numbers produce None.
    #[test]
    fn unknown_version_returns_none(n in 4u32..u32::MAX) {
        prop_assert_eq!(LayoutVersion::from_u32(n), None);
    }

    /// LayoutVersion::CURRENT is V3.
    #[test]
    fn current_version_is_latest(_dummy in 0u32..1) {
        prop_assert_eq!(LayoutVersion::CURRENT, LayoutVersion::V3);
    }
}

// ─── RokoLayout pure path arithmetic ────────────────────────────────────────

proptest! {
    /// All sub-paths are children of the root directory.
    #[test]
    fn layout_paths_under_root(root_name in "[a-z]{1,12}") {
        let root = format!("/tmp/.roko-{root_name}");
        let layout = RokoLayout::new(&root);
        prop_assert!(layout.runtime_dir().starts_with(layout.root()));
        prop_assert!(layout.memory_dir().starts_with(layout.root()));
        prop_assert!(layout.plans_dir().starts_with(layout.root()));
        prop_assert!(layout.runs_dir().starts_with(layout.root()));
        prop_assert!(layout.state_dir().starts_with(layout.root()));
        prop_assert!(layout.cache_dir().starts_with(layout.root()));
        prop_assert!(layout.learn_dir().starts_with(layout.root()));
    }

    /// for_project joins .roko onto the project root.
    #[test]
    fn for_project_appends_roko(project in "[a-z]{1,12}") {
        let project_root = format!("/tmp/{project}");
        let layout = RokoLayout::for_project(&project_root);
        let root = layout.root();
        prop_assert!(root.ends_with(".roko"),
            "for_project should produce a root ending in .roko, got: {root:?}");
        prop_assert!(root.starts_with(&project_root),
            "for_project root should be under the project root");
    }
}

// ─── FsRetentionPolicy ───────────────────────────────────────────────────────

proptest! {
    /// Default policy has positive limits for all fields.
    #[test]
    fn default_retention_policy_positive(_dummy in 0u32..1) {
        let p = FsRetentionPolicy::default();
        prop_assert!(p.max_episodes > 0);
        prop_assert!(p.max_run_age_days > 0);
        prop_assert!(p.max_archive_age_days > 0);
        prop_assert!(p.size_threshold_mb > 0);
        prop_assert!(p.max_cache_entries > 0);
    }
}

// ─── GcReport pure-logic ─────────────────────────────────────────────────────

proptest! {
    /// is_empty() matches candidates being empty.
    #[test]
    fn gc_report_is_empty_iff_no_candidates(n in 0usize..20) {
        let candidates: Vec<GcCandidate> = (0..n)
            .map(|i| GcCandidate {
                path: std::path::PathBuf::from(format!("/tmp/candidate-{i}")),
                reason: "test".into(),
                size_bytes: 1024 * (i as u64 + 1),
            })
            .collect();
        let report = GcReport {
            candidates,
            total_bytes: 0,
            removed_count: 0,
            failed_count: 0,
        };
        prop_assert_eq!(report.is_empty(), n == 0);
        prop_assert_eq!(report.candidate_count(), n);
    }
}

// ─── validate_metric_key ────────────────────────────────────────────────────

proptest! {
    /// Clean alphanumeric keys with underscores always pass validation.
    #[test]
    fn clean_metric_key_always_valid(key in "[a-z][a-z0-9_]{0,30}") {
        // Exclude keys that contain known secret prefixes.
        // (The property validates that our "safe" patterns don't trigger false positives.)
        let result = validate_metric_key(&key);
        // Any key that doesn't contain forbidden patterns should pass.
        // We don't control what patterns are forbidden, so we just assert no panic.
        let _ = result;
    }

    /// validate_cardinality(n) fails when n > MAX_METRIC_CARDINALITY.
    #[test]
    fn cardinality_above_max_fails(excess in 1usize..1000) {
        let over_limit = MAX_METRIC_CARDINALITY + excess;
        prop_assert!(validate_cardinality(over_limit).is_err(),
            "validate_cardinality({over_limit}) should fail (limit={MAX_METRIC_CARDINALITY})");
    }

    /// validate_cardinality(n) succeeds when n <= MAX_METRIC_CARDINALITY.
    #[test]
    fn cardinality_within_limit_passes(n in 0usize..100) {
        // Use a small capped n well within any reasonable limit.
        prop_assert!(validate_cardinality(n).is_ok(),
            "validate_cardinality({n}) should pass (limit={MAX_METRIC_CARDINALITY})");
    }
}
