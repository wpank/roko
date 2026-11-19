//! The workspace's own build system, for the plans written for it
//! (bug-8723fc). The generator prompt teaches by Rust examples, which a
//! planner copies whatever the project is: round v7's Undertow phase 2, a
//! C++/CMake repo with no `Cargo.toml`, got `cargo check -p undertow_core` in
//! every task. A CMake workspace gets the examples in its own commands and a
//! note on how it builds instead, and a task with no check gets one that
//! builds with the workspace's presets, never with cargo.

use std::fmt::Write as _;
use std::path::Path;

/// How a CMake workspace configures, builds and runs its tests: with the
/// presets of its `CMakePresets.json`, else in a `build` directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CmakeCommands {
    /// Each build preset that is not hidden, with the configure preset it
    /// builds: (build, configure).
    presets: Vec<(String, String)>,
    /// Each test preset that is not hidden, with its configure preset.
    tests: Vec<(String, String)>,
}

impl CmakeCommands {
    /// The commands of `workdir` when it is a CMake project with no
    /// `Cargo.toml`: a `CMakeLists.txt` at the top or up to two directories
    /// down (a seed's `tests/spec/CMakeLists.txt` counts before the top one is
    /// written). `None` for any other workspace.
    pub(super) fn of(workdir: &Path) -> Option<Self> {
        if workdir.join("Cargo.toml").is_file() || !super::runnable::holds_cmake_lists(workdir, 2) {
            return None;
        }
        let presets = std::fs::read_to_string(workdir.join("CMakePresets.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let named = |key: &str| -> Vec<(String, String)> {
            presets
                .as_ref()
                .and_then(|presets| presets.get(key))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter(|preset| {
                    preset.get("hidden").and_then(serde_json::Value::as_bool) != Some(true)
                })
                .filter_map(|preset| {
                    let name = preset.get("name")?.as_str()?;
                    let configure = preset
                        .get("configurePreset")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(name);
                    Some((name.to_string(), configure.to_string()))
                })
                .collect()
        };
        Some(Self {
            presets: named("buildPresets"),
            tests: named("testPresets"),
        })
    }

    /// The build preset for work in `dir`, a path's first directory: the
    /// preset of that name, else the first one. `None` without presets.
    fn preset(&self, dir: Option<&str>) -> Option<&(String, String)> {
        dir.and_then(|dir| self.presets.iter().find(|(name, _)| name == dir))
            .or_else(|| self.presets.first())
    }

    /// The command that configures and builds the work in `dir`.
    pub(super) fn build_command(&self, dir: Option<&str>) -> String {
        format!("{} && {}", self.configure(dir), self.build(dir))
    }

    /// Configures the build tree: `cmake --preset core`.
    fn configure(&self, dir: Option<&str>) -> String {
        self.preset(dir).map_or_else(
            || "cmake -S . -B build".to_string(),
            |(_, configure)| format!("cmake --preset {configure}"),
        )
    }

    /// Builds it: `cmake --build --preset core`.
    fn build(&self, dir: Option<&str>) -> String {
        self.preset(dir).map_or_else(
            || "cmake --build build".to_string(),
            |(name, _)| format!("cmake --build --preset {name}"),
        )
    }

    /// Runs the tests whose names match the word that follows it: `ctest
    /// --preset core -R`.
    fn test(&self) -> String {
        let configure = self.preset(None).map(|(_, configure)| configure);
        self.tests
            .iter()
            .find(|(_, tested)| Some(tested) == configure)
            .or_else(|| self.tests.first())
            .map_or_else(
                || "ctest --test-dir build -R".to_string(),
                |(name, _)| format!("ctest --preset {name} -R"),
            )
    }

    /// The prompt's note on how the workspace builds.
    fn note(&self) -> String {
        let mut note =
            String::from("## This workspace's build\n\nThis workspace builds with CMake");
        if !self.presets.is_empty() {
            let names: Vec<&str> = self.presets.iter().map(|(name, _)| name.as_str()).collect();
            let _ = write!(note, " and its presets ({})", names.join(", "));
        }
        let _ = writeln!(
            note,
            ". It has no `Cargo.toml`, so no verify step runs `cargo`. Configure with `{}`, build \
             with `{}`, and run the tests whose names match with `{} <name>`. A check whose \
             command the request gives runs that command alone and unchanged, with no `grep -q` \
             before it.",
            self.configure(None),
            self.build(None),
            self.test()
        );
        note
    }
}

/// The generator's system prompt for `workdir`: for a CMake workspace, its
/// Rust examples rewritten in the workspace's own commands, and a note on how
/// it builds; for any other, `prompt` itself.
pub(super) fn localize(prompt: &str, workdir: &Path) -> String {
    let Some(cmake) = CmakeCommands::of(workdir) else {
        return prompt.to_string();
    };
    let mut localized = prompt.to_string();
    for (rust, cpp) in rewrites(&cmake) {
        localized = localized.replacen(rust, &cpp, 1);
    }
    localized.push('\n');
    localized.push_str(&cmake.note());
    localized
}

/// The prompt's Rust-only passages, each with what a CMake workspace reads
/// instead. Every passage occurs in the prompt once (the tests hold it to
/// that), so a change to the prompt that drops one is caught.
fn rewrites(cmake: &CmakeCommands) -> Vec<(&'static str, String)> {
    let (configure, build, test) = (cmake.configure(None), cmake.build(None), cmake.test());
    vec![
        (
            MINIMUM_RUST,
            MINIMUM_CPP.replace("{BUILD}", &format!("{configure} && {build}")),
        ),
        (
            "(`grep -q 'fn new_test' crates/x/src/lib.rs && cargo test -p x new_test`)",
            format!(
                "(`grep -q 'new_test' tests/x_test.cpp && {configure} && {build} && {test} new_test`)"
            ),
        ),
        (
            "`test -f crates/roko-core/src/types.rs && echo \"exists\" || echo \"MISSING\"`",
            "`test -f src/gain.cpp && echo \"exists\" || echo \"MISSING\"`".to_string(),
        ),
        (
            LANGUAGES_RUST,
            format!(
                "This workspace builds with CMake: `{configure}`, then `{build}`, then `{test} \
                 <test>`. It has no `Cargo.toml`, so no check runs `cargo`."
            ),
        ),
        (
            "Prefer the crate's own gate (e.g. `cargo check -p roko-core`, `cargo test -p roko-core parse_config`)",
            format!("Prefer the project's own gate (e.g. `{build}`, `{test} parse_config`)"),
        ),
        (ACCEPT_RUST, String::new()),
        (
            "1. Use CONCRETE file paths: `\"crates/my-crate/src/lib.rs\"` NOT `\"crates/\"` or `\"crates/*/src/*.rs\"`.",
            "1. Use CONCRETE file paths: `\"src/gain.cpp\"` NOT `\"src/\"` or `\"src/*.cpp\"`."
                .to_string(),
        ),
        (
            NEW_CRATE_RUST,
            "6. If a task creates a new library, executable or test, list its source files and \
             the `CMakeLists.txt` that names them."
                .to_string(),
        ),
        (
            OUTPUT_EXAMPLE_RUST,
            OUTPUT_EXAMPLE_CPP
                .replace("{CONFIGURE}", &configure)
                .replace("{BUILD}", &build)
                .replace("{TEST}", &test),
        ),
        (
            COMPLETE_EXAMPLE_RUST,
            COMPLETE_EXAMPLE_CPP
                .replace("{CONFIGURE}", &configure)
                .replace("{BUILD}", &build)
                .replace("{TEST}", &test),
        ),
    ]
}

/// The prompt's minimum valid structure, from its task's `files`.
const MINIMUM_RUST: &str = r#"files = ["crates/roko-core/src/lib.rs"]
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-core/src/lib.rs", lines = "1-50", why = "Read existing types." },
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core""#;

/// [`MINIMUM_RUST`] for a CMake workspace; `{BUILD}` is its build command.
const MINIMUM_CPP: &str = r#"files = ["src/gain.cpp"]
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "src/gain.cpp", lines = "1-50", why = "Read the existing code." },
]

[[task.verify]]
phase = "compile"
command = "{BUILD}""#;

/// The prompt's list of languages and their commands.
const LANGUAGES_RUST: &str = "Detect the project language and use the right commands:
- Cargo.toml → Rust: `cargo check`, `cargo test`, `cargo clippy`
- package.json → TypeScript: `npx tsc`, `npx jest`, `npx eslint`
- go.mod → Go: `go build`, `go test`, `golangci-lint`
- pyproject.toml/setup.py → Python: `python -m py_compile`, `pytest` (or `python3 -m unittest` when the tests use unittest), `ruff`
- CMakeLists.txt → C/C++: `cmake --build`, `ctest -R <test>` (with CMakePresets.json: `cmake --preset <name>`, `cmake --build --preset <name>`, `ctest --preset <name> -R <test>`)";

/// The prompt's Rust example of a pinned acceptance test.
const ACCEPT_RUST: &str = "Rust example, an integration test the run copies into the crate's `tests/`:

```toml
[task.accept]
files = [
    { src = \"accept/slug_accept.rs\", dest = \"crates/roko-core/tests/slug_accept.rs\", runner = \"cargo test -p roko-core --test slug_accept\", count = 3 },
]
```

";

/// The prompt's rule for a new crate's files.
const NEW_CRATE_RUST: &str = "6. If a task creates a NEW crate, list the specific files: `\"crates/new-crate/src/lib.rs\"`, `\"crates/new-crate/Cargo.toml\"`. Use the plan slug as the crate name (e.g., slug \"btc-funding-alert\" → `\"crates/btc-funding-alert/src/lib.rs\"`).";

/// The prompt's example of a whole `tasks.toml`, from its `[meta]` to the end
/// of its last verify step.
const OUTPUT_EXAMPLE_RUST: &str = r#"[meta]
plan = "add-funding-rate"  # MUST match the plan slug exactly
total = 3
done = 0
status = "ready"
# max_parallel is omitted: tasks that do not depend on each other run together

[[task]]
id = "T1"
title = "Add FundingRate struct to core types"
goal = "roko-core exports a FundingRate type that other crates can construct and read."
description = "Define the FundingRate data structure in roko-core for storing funding rate observations."
acceptance = [
    "AC1: types.rs defines `pub struct FundingRate`, and `cargo check -p roko-core` passes.",
]
non_goals = ["Do not change any existing type in types.rs."]
open_questions = []
status = "ready"
tier = "mechanical"       # mechanical | focused | integrative | architectural
# model_hint omitted — runtime picks the best model automatically
max_loc = 20              # maximum lines of change
files = ["crates/roko-core/src/types.rs"]   # REAL file paths only, never <path> or <crate>
# allowed_tools and denied_tools omitted — the role's own tools fit the task
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = []
role = "implementer"      # REQUIRED: implementer | architect | researcher | strategist | quick-reviewer | scribe

# SURGICAL CONTEXT: exactly what the agent needs to read
[task.context]
read_files = [
    { path = "crates/roko-core/src/types.rs", lines = "1-50", why = "Find existing type definitions to follow naming conventions." },
]
symbols = [
    "Signal — existing base type to reference",
]
anti_patterns = [
    "Do NOT create new files. Modify crates/roko-core/src/types.rs only.",
]

# A check you write yourself: a grep for what the task adds, then the gate.
# A check whose command the source gives runs that command alone, unguarded.
[[task.verify]]
phase = "compile"
command = "grep -q 'pub struct FundingRate' crates/roko-core/src/types.rs && cargo check -p roko-core"
covers = ["AC1"]
expect = "fail_on_base"

[[task]]
id = "T2"
title = "Wire FundingRate display into CLI status output"
goal = "`roko status` prints the latest funding rate."
description = "Import FundingRate from roko-core and add it to the status command output."
acceptance = [
    "AC1: `cargo test -p roko-cli status_shows_funding_rate` passes: the status output has a `funding rate:` line.",
]
non_goals = ["Do not modify roko-core."]
open_questions = []
status = "ready"
tier = "focused"
# model_hint omitted — runtime selects automatically
max_loc = 40
files = ["crates/roko-cli/src/commands/status.rs"]
# allowed_tools and denied_tools omitted — the role's own tools fit the task
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = ["T1"]
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/status.rs", lines = "1-80", why = "Understand current status output format." },
    { path = "crates/roko-core/src/types.rs", lines = "1-30", why = "Import the new FundingRate type." },
]
symbols = [
    "StatusOutput — struct that collects status display fields",
]
anti_patterns = [
    "Do NOT modify roko-core. Only change the CLI crate.",
]

[[task.verify]]
phase = "test"
command = "grep -q 'fn status_shows_funding_rate' crates/roko-cli/src/commands/status.rs && cargo test -p roko-cli status_shows_funding_rate"
covers = ["AC1"]
expect = "fail_on_base""#;

/// [`OUTPUT_EXAMPLE_RUST`] for a CMake workspace; `{CONFIGURE}`, `{BUILD}`
/// and `{TEST}` are its commands.
const OUTPUT_EXAMPLE_CPP: &str = r#"[meta]
plan = "add-gain-stage"  # MUST match the plan slug exactly
total = 2
done = 0
status = "ready"
# max_parallel is omitted: tasks that do not depend on each other run together

[[task]]
id = "T1"
title = "Add the GainStage class"
goal = "The library has a GainStage that scales a sample by a gain in decibels."
description = "Add GainStage (src/gain.h, src/gain.cpp) to the library and list gain.cpp among its sources."
acceptance = [
    "AC1: src/gain.h declares `class GainStage`, and the library builds with gain.cpp in it.",
]
non_goals = ["Do not change any existing class."]
open_questions = []
status = "ready"
tier = "focused"          # mechanical | focused | integrative | architectural
# model_hint omitted — runtime picks the best model automatically
max_loc = 40              # maximum lines of change
files = ["src/gain.h", "src/gain.cpp", "CMakeLists.txt"]   # REAL file paths only, never <path>
# allowed_tools and denied_tools omitted — the role's own tools fit the task
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = []
role = "implementer"      # REQUIRED: implementer | architect | researcher | strategist | quick-reviewer | scribe

# SURGICAL CONTEXT: exactly what the agent needs to read
[task.context]
read_files = [
    { path = "CMakeLists.txt", lines = "1-40", why = "Find the library target and its list of sources." },
]
symbols = [
    "Engine — existing class whose header layout to follow",
]
anti_patterns = [
    "Do NOT add a new library target. Add gain.cpp to the existing one.",
]

# A check you write yourself: a grep for what the task adds, then the gate.
# A check whose command the source gives runs that command alone, unguarded.
[[task.verify]]
phase = "compile"
command = "grep -q 'class GainStage' src/gain.h && {CONFIGURE} && {BUILD}"
covers = ["AC1"]
expect = "fail_on_base"

[[task]]
id = "T2"
title = "Test the gain stage"
goal = "A test proves that GainStage scales by its gain in decibels."
description = "Write tests/gain_test.cpp with the gain checks and register it with the tests."
acceptance = [
    "AC1: `{TEST} gain` passes: 0 dB leaves a sample unchanged, and -6 dB halves it within 0.01.",
]
non_goals = ["Do not change GainStage."]
open_questions = []
status = "ready"
tier = "focused"
# model_hint omitted — runtime selects automatically
max_loc = 50
files = ["tests/gain_test.cpp", "tests/CMakeLists.txt"]
# allowed_tools and denied_tools omitted — the role's own tools fit the task
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = ["T1"]
role = "implementer"

[task.context]
read_files = [
    { path = "src/gain.h", lines = "1-40", why = "The API under test." },
    { path = "tests/CMakeLists.txt", lines = "1-40", why = "How a test file is registered." },
]
symbols = [
    "GainStage — the class under test",
]
anti_patterns = [
    "Do NOT change GainStage to make a test pass.",
]

[[task.verify]]
phase = "test"
command = "grep -q 'GainStage' tests/gain_test.cpp && {CONFIGURE} && {BUILD} && {TEST} gain"
covers = ["AC1"]
expect = "fail_on_base""#;

/// The prompt's end-to-end example, from its request to the end of its one
/// task's verify step.
const COMPLETE_EXAMPLE_RUST: &str = r#"A realistic plan for "Add health check endpoint to roko-serve":

```toml
[meta]
plan = "add-health-check"
total = 1
done = 0
status = "ready"

[[task]]
id = "T1"
title = "Implement and prove GET /health"
goal = "GET /health answers 200 with a JSON body that says the server is up."
description = "Add the response type and handler, register GET /health, and add one exact API integration test as one observable endpoint outcome."
acceptance = [
    "AC1: GET /health returns HTTP 200 with the JSON body {\"status\": \"ok\"}.",
    "AC2: GET /health needs no token: a request without one also gets 200.",
]
non_goals = [
    "Do not add readiness or dependency checks; /health reports only that the server answers.",
]
open_questions = []
status = "ready"
tier = "integrative"
max_loc = 150
files = [
    "crates/roko-serve/src/routes/health.rs",
    "crates/roko-serve/src/routes/mod.rs",
    "crates/roko-serve/tests/api_integration.rs",
]
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/providers.rs", lines = "1-80", why = "Follow the existing JSON response and handler conventions." },
    { path = "crates/roko-serve/src/routes/mod.rs", lines = "270-340", why = "Exact build_router registration seam." },
    { path = "crates/roko-serve/tests/api_integration.rs", lines = "60-180", why = "Reuse the bounded test_app and GET helper pattern." },
]
symbols = [
    "build_router — existing route registration function",
    "test_app — existing integration test fixture",
]
anti_patterns = ["Do NOT add new dependencies. Use only std and existing crate types."]

[[task.verify]]
phase = "test"
command = "grep -q 'fn health_endpoint' crates/roko-serve/tests/api_integration.rs && cargo test -p roko-serve --test api_integration health_endpoint"
covers = ["AC1", "AC2"]
expect = "fail_on_base"
fail_msg = "The exact health endpoint integration test failed or was not found""#;

/// [`COMPLETE_EXAMPLE_RUST`] for a CMake workspace; `{CONFIGURE}`, `{BUILD}`
/// and `{TEST}` are its commands.
const COMPLETE_EXAMPLE_CPP: &str = r#"A realistic plan for "Add a soft clipper to the output stage":

```toml
[meta]
plan = "add-soft-clipper"
total = 1
done = 0
status = "ready"

[[task]]
id = "T1"
title = "Implement and prove the soft clipper"
goal = "The output stage soft-clips: quiet samples pass unchanged, and loud ones stay under the ceiling."
description = "Add softClip to the output stage, run each output sample through it, and add one exact test of its curve."
acceptance = [
    "AC1: softClip(x) returns x unchanged for |x| <= 0.5.",
    "AC2: softClip never returns more than 0.966 in magnitude, however loud its input.",
]
non_goals = [
    "Do not change the gain staging before the clipper.",
]
open_questions = []
status = "ready"
tier = "integrative"
max_loc = 150
files = [
    "src/output_stage.h",
    "src/output_stage.cpp",
    "tests/output_stage_test.cpp",
    "tests/CMakeLists.txt",
]
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "src/output_stage.h", lines = "1-60", why = "The output stage's API and where a sample is written." },
    { path = "src/output_stage.cpp", lines = "1-120", why = "The per-sample loop the clipper joins." },
    { path = "tests/CMakeLists.txt", lines = "1-40", why = "How a test file is registered." },
]
symbols = [
    "OutputStage::process — the per-sample loop",
]
anti_patterns = ["Do NOT add a dependency. Use only the standard library."]

[[task.verify]]
phase = "test"
command = "grep -q 'softClip' tests/output_stage_test.cpp && {CONFIGURE} && {BUILD} && {TEST} output_stage"
covers = ["AC1", "AC2"]
expect = "fail_on_base"
fail_msg = "The soft clipper's test failed or was not found""#;

#[cfg(test)]
mod tests {
    use super::*;

    /// A CMake workspace with Undertow's presets (`core` and `plugin`).
    fn undertow_like() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("CMakeLists.txt"), "project(undertow)\n").expect("cmake");
        std::fs::write(
            temp.path().join("CMakePresets.json"),
            r#"{"version": 6,
                "configurePresets": [{"name": "base", "hidden": true}, {"name": "core"}, {"name": "plugin"}],
                "buildPresets": [{"name": "core", "configurePreset": "core"}, {"name": "plugin", "configurePreset": "plugin"}],
                "testPresets": [{"name": "core", "configurePreset": "core"}, {"name": "plugin", "configurePreset": "plugin"}]}"#,
        )
        .expect("presets");
        temp
    }

    /// Every Rust passage the rewrite replaces is in the prompt once, so a
    /// change to the prompt that drops one is caught here.
    #[test]
    fn every_rust_passage_is_in_the_prompt_once() {
        let workspace = undertow_like();
        let cmake = CmakeCommands::of(workspace.path()).expect("a CMake workspace");
        let prompt = super::super::PLAN_GENERATOR_SYSTEM_PROMPT;
        for (rust, _) in rewrites(&cmake) {
            assert_eq!(prompt.matches(rust).count(), 1, "{rust}");
        }
    }

    /// The commands come from the presets: the build preset named for a
    /// path's first directory, else the first, and the test preset of the
    /// same configure preset. Without presets, a `build` directory.
    #[test]
    fn the_commands_follow_the_presets() {
        let workspace = undertow_like();
        let cmake = CmakeCommands::of(workspace.path()).expect("a CMake workspace");
        assert_eq!(
            cmake.build_command(Some("plugin")),
            "cmake --preset plugin && cmake --build --preset plugin"
        );
        assert_eq!(
            cmake.build_command(Some("core")),
            "cmake --preset core && cmake --build --preset core"
        );
        assert_eq!(
            cmake.build_command(Some("docs")),
            "cmake --preset core && cmake --build --preset core"
        );
        assert_eq!(cmake.test(), "ctest --preset core -R");

        std::fs::remove_file(workspace.path().join("CMakePresets.json")).expect("no presets");
        let plain = CmakeCommands::of(workspace.path()).expect("still CMake");
        assert_eq!(
            plain.build_command(None),
            "cmake -S . -B build && cmake --build build"
        );
        assert_eq!(plain.test(), "ctest --test-dir build -R");

        std::fs::write(workspace.path().join("Cargo.toml"), "[workspace]\n").expect("cargo");
        assert_eq!(CmakeCommands::of(workspace.path()), None);
    }

    /// The rewritten examples are plans that parse, with no cargo in them.
    #[test]
    fn the_cpp_examples_parse() {
        let workspace = undertow_like();
        let cmake = CmakeCommands::of(workspace.path()).expect("a CMake workspace");
        for (_, cpp) in rewrites(&cmake) {
            assert!(!cpp.contains("cargo "), "{cpp}");
        }
        let output = OUTPUT_EXAMPLE_CPP
            .replace("{CONFIGURE}", "cmake --preset core")
            .replace("{BUILD}", "cmake --build --preset core")
            .replace("{TEST}", "ctest --preset core -R");
        let plan = crate::task_parser::TasksFile::parse_str(&output).expect("the output example");
        assert_eq!(plan.tasks.len(), 2);
        let complete = COMPLETE_EXAMPLE_CPP
            .replace("{CONFIGURE}", "cmake --preset core")
            .replace("{BUILD}", "cmake --build --preset core")
            .replace("{TEST}", "ctest --preset core -R");
        let toml = complete
            .split_once("```toml\n")
            .map(|(_, toml)| toml)
            .expect("the complete example's TOML");
        let plan = crate::task_parser::TasksFile::parse_str(toml).expect("the complete example");
        assert_eq!(plan.tasks.len(), 1);
    }
}
