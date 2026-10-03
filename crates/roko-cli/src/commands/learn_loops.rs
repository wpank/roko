//! `roko learn loops`: the M2 loop census (S03 T15; backlog 5107). The CLI
//! and roko-learn's `loop_census` example print the same
//! `census::render_json`, so their output cannot drift.

use std::path::Path;

use anyhow::Context as _;
use roko_learn::loop_audit::{Registry, census};

/// What `roko learn loops` prints for `workdir`: the report-only census, the
/// only mode until the measured audit (backlog 5123) lands, as
/// `roko.loop_census/1` JSON or as a table.
///
/// # Errors
///
/// Returns an error when the workspace's loop-registry override does not
/// load.
pub fn loops_output(workdir: &Path, json: bool) -> anyhow::Result<String> {
    let registry = Registry::load(workdir).context("load the loop registry")?;
    let sha = census::harness_sha(workdir);
    let report = census::run(workdir, &registry, sha.as_deref());
    if json {
        Ok(census::render_json(&report)?)
    } else {
        Ok(census::render_text(&report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copy the directory tree at `from` to `to`.
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("create the copy");
        for entry in std::fs::read_dir(from).expect("read the fixture") {
            let path = entry.expect("a fixture entry").path();
            let target = to.join(path.file_name().expect("a file name"));
            if path.is_dir() {
                copy_tree(&path, &target);
            } else {
                std::fs::copy(&path, &target).expect("copy a fixture file");
            }
        }
    }

    /// S03 T15: on the 09-29 fixture (backlog 5105), `roko learn loops
    /// --census --json` prints exactly the library's census JSON, and the
    /// table has one line per loop.
    #[test]
    fn learn_loops_census_json_matches_library() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../roko-learn/tests/fixtures/loop_census");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_tree(&fixture, &temp.path().join(".roko"));
        let workdir = temp.path();

        let printed = loops_output(workdir, true).expect("the census");
        let registry = Registry::load(workdir).expect("the registry");
        let sha = census::harness_sha(workdir);
        let report = census::run(workdir, &registry, sha.as_deref());
        assert_eq!(printed, census::render_json(&report).expect("render"));
        let parsed: serde_json::Value = serde_json::from_str(&printed).expect("JSON");
        assert_eq!(parsed["schema"], census::CENSUS_SCHEMA);
        let linucb = report.row("L-linucb").expect("an L-linucb row");
        assert_eq!(linucb.reason.map(|reason| reason.as_str()), Some("dormant:no_learning"));

        let table = loops_output(workdir, false).expect("the table");
        assert_eq!(table.lines().count(), registry.loops().len() + 1, "{table}");
        assert!(
            table
                .lines()
                .any(|line| line.starts_with("L-linucb") && line.contains("dormant:no_learning")),
            "{table}"
        );
    }
}
