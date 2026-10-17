//! Test support for bug-3c4600 (roko-cli has the same helper).
//!
//! Run one check path in a child test process whose environment holds the
//! person's credentials, and check what the path's command saw: no
//! credential, the run's job cap and niceness (bug-b3849c), and no cargo
//! build dir that other checkouts share (bug-be2a53).
//!
//! The parent test calls [`run_child`] with the name of a child test. The
//! child test starts with [`begin_child`], which returns `None` outside the
//! child process, runs its path with a command made by [`dump_env_command`],
//! and checks the dump with [`assert_check_env`].

use std::collections::BTreeMap;
use std::path::Path;

use roko_core::child_env::{
    self, CARGO_BUILD_DIR_VARS, CheckEnvSettings, CheckRun, DotenvNames, GITHUB_TOKEN_VARS,
    GateLimits,
};

/// Set in the child process, to the directory that holds the fake `gh`
/// login.
const CHILD: &str = "ROKO_CHECK_ENV_PROBE_CHILD";

/// A name the child's run passes through (`[gates] env_passthrough`); the
/// allowlist alone would drop it.
pub const PASSED: &str = "DATABASE_URL";
const PASSED_VALUE: &str = "postgres://probe";

/// A provider key the child records as loaded from a `.env` file.
const DOTENV_KEY: &str = "OPENAI_API_KEY";

/// The gate limits of the child's run: a check that shows them was
/// throttled, rather than handed a job count or a niceness by its parent.
const PROBE_JOBS: usize = 3;
const PROBE_NICE: i32 = 15;

/// Run test `child` (its full path, `module::tests::name`) in a child
/// process whose environment holds every GitHub token, a `GH_CONFIG_DIR`
/// with a `gh` login, a provider key, [`PASSED`], and each of
/// [`CARGO_BUILD_DIR_VARS`] naming a directory outside every check's, as an
/// exported `CARGO_TARGET_DIR` does, and assert it passed.
pub fn run_child(child: &str) {
    let login = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        login.path().join("hosts.yml"),
        "github.com:\n    user: probe\n",
    )
    .expect("a gh login");
    let shared = tempfile::tempdir().expect("tempdir");
    let mut command = std::process::Command::new(std::env::current_exe().expect("test binary"));
    command
        .args(["--exact", child, "--nocapture"])
        .env(CHILD, login.path())
        .env(child_env::GH_CONFIG_DIR, login.path())
        .env(DOTENV_KEY, "sk-probe-not-real")
        .env(PASSED, PASSED_VALUE)
        // The parent's job counts (heavy.sh sets them) must not stand in for
        // the throttle's.
        .env_remove("CARGO_BUILD_JOBS")
        .env_remove("RUST_TEST_THREADS");
    for name in GITHUB_TOKEN_VARS {
        command.env(name, "gh-probe-not-real");
    }
    for name in CARGO_BUILD_DIR_VARS {
        command.env(name, shared.path().join(name));
    }
    let output = command.output().expect("run the child test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("1 passed"),
        "the child test did not run: {stdout}"
    );
}

/// In [`run_child`]'s child process: record [`DOTENV_KEY`] as loaded from a
/// `.env` file and start a run whose `[gates] env_passthrough` names
/// [`PASSED`] and whose gate limits are [`PROBE_JOBS`] and [`PROBE_NICE`], as
/// a plan run does. `None` in any other process.
pub fn begin_child() -> Option<CheckRun> {
    std::env::var_os(CHILD)?;
    let mut dotenv = DotenvNames::new();
    dotenv.insert(DOTENV_KEY, true);
    child_env::record_startup_dotenv(dotenv);
    child_env::set_gate_limits(GateLimits {
        cargo_jobs: Some(PROBE_JOBS),
        nice: PROBE_NICE,
    });
    Some(child_env::begin_check_run(CheckEnvSettings {
        passthrough: vec![PASSED.to_string()],
        strip: Vec::new(),
    }))
}

/// A shell command that writes its environment, and its niceness as
/// `ROKO_PROBE_NICE`, to `out`.
pub fn dump_env_command(out: &Path) -> String {
    format!(
        "{{ env; echo \"ROKO_PROBE_NICE=$(ps -o nice= -p $$)\"; }} > '{}'",
        out.display()
    )
}

/// Assert that the environment the check of `path` wrote to `out` holds no
/// GitHub token and no key roko loaded, points `gh` at an empty directory,
/// keeps [`PASSED`], builds in none of the shared cargo build dirs
/// [`run_child`] set, and that the check ran with the run's job cap at its
/// niceness or nicer.
pub fn assert_check_env(out: &Path, path: &str) {
    let dump = std::fs::read_to_string(out)
        .unwrap_or_else(|error| panic!("{path}: the check wrote no environment: {error}"));
    let env: BTreeMap<&str, &str> = dump
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    for name in GITHUB_TOKEN_VARS {
        assert!(!env.contains_key(name), "{path}: {name} reached the check");
    }
    for name in CARGO_BUILD_DIR_VARS {
        let shared = std::env::var(name).expect("the shared build dir run_child set");
        assert_ne!(
            env.get(name).copied(),
            Some(shared.as_str()),
            "{path}: the check builds in a {name} other checkouts share"
        );
    }
    assert!(
        !env.contains_key(DOTENV_KEY),
        "{path}: a provider key roko loaded reached the check"
    );
    let login = std::env::var(CHILD).expect("the child's login directory");
    let gh_config = env
        .get(child_env::GH_CONFIG_DIR)
        .unwrap_or_else(|| panic!("{path}: the check has no GH_CONFIG_DIR"));
    assert_ne!(*gh_config, login, "{path}: gh would find the login");
    let entries = std::fs::read_dir(gh_config)
        .unwrap_or_else(|error| panic!("{path}: GH_CONFIG_DIR {gh_config}: {error}"))
        .count();
    assert_eq!(entries, 0, "{path}: GH_CONFIG_DIR {gh_config} is not empty");
    assert_eq!(
        env.get(PASSED).copied(),
        Some(PASSED_VALUE),
        "{path}: {PASSED} was not passed through"
    );
    let jobs = PROBE_JOBS.to_string();
    for name in ["CARGO_BUILD_JOBS", "RUST_TEST_THREADS"] {
        assert_eq!(
            env.get(name).copied(),
            Some(jobs.as_str()),
            "{path}: {name} is not the run's job cap"
        );
    }
    let nice: i32 = env
        .get("ROKO_PROBE_NICE")
        .and_then(|nice| nice.trim().parse().ok())
        .unwrap_or_else(|| panic!("{path}: the check wrote no niceness"));
    assert!(
        nice >= PROBE_NICE,
        "{path}: the check ran at niceness {nice}, below the run's {PROBE_NICE}"
    );
}
