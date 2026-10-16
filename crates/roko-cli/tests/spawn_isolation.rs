//! bug-eed697: roko-cli's integration tests never run the `roko` binary with
//! the developer's `HOME`. With it, roko loads `~/.roko/.env`, whose provider
//! keys routed fixture tasks to real, paid models, and merges
//! `~/.roko/config.toml`'s providers and models into every fixture.

mod common;
use common::Isolated;

use std::path::Path;

/// Every test file that spawns `roko` isolates each spawn: through
/// `common::Isolated`, or by setting `HOME` itself.
#[test]
fn every_test_that_spawns_roko_isolates_it() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    for dir in [tests.clone(), tests.join("common")] {
        for entry in std::fs::read_dir(&dir).expect("read a test directory") {
            let path = entry.expect("a directory entry").path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    let mut leaks = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read a test file");
        let spawns = text.matches("cargo_bin(\"roko\")").count();
        let isolated = text.matches(".isolated()").count();
        if spawns > isolated && !text.contains("env(\"HOME\"") {
            leaks.push(format!(
                "{}: {spawns} spawn(s), {isolated} isolated",
                path.display()
            ));
        }
    }
    assert!(
        leaks.is_empty(),
        "these tests run roko with the developer's HOME; spawn it through \
         common::Isolated: {leaks:#?}"
    );
}

/// An isolated `roko` reads its global config from the scratch `HOME`, not
/// from the developer's.
#[test]
fn an_isolated_roko_uses_the_scratch_home() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let output = assert_cmd::Command::cargo_bin("roko")
        .expect("roko binary")
        .isolated()
        .current_dir(workdir.path())
        .args(["config", "path"])
        .output()
        .expect("run roko config path");
    let text = String::from_utf8_lossy(&output.stdout);
    let global = common::scratch_home().join(".roko").join("config.toml");
    assert!(
        text.contains(&global.display().to_string()),
        "roko config path names another global config: {text}"
    );
}
