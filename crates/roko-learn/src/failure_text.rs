//! What a failed check printed, read for its retry (bug-3e3d08).
//!
//! A retry is told why its last try failed from the check's output. A C++
//! project's checks are test binaries: doctest, Catch2 or GoogleTest. They
//! print each failing case with its failed assertions, the values the
//! assertion saw and what the test logged, then a summary of counts. Their
//! failure lines hold neither "test" nor "failed", and a blind tail of the
//! output keeps whichever case the runner printed last: Undertow's phase 4.2
//! T1#1 failed 2 cases, and its retry was told about one in full and the other
//! by half a line.
//!
//! This module reads which lines say that something failed, which are build
//! progress or a runner's summary, and which test cases failed, each with its
//! failures. Then [`failure_excerpt`] keeps each failing case's header, its
//! first failure with the lines that say why, and one line for each further
//! failure, before the summary.

/// The most lines under a failure that its excerpt keeps.
const DETAIL_LINES: usize = 8;

/// Lines kept above the first failure line of an output with no failing
/// case, for the error that line belongs to.
const LEAD_LINES: usize = 3;

/// The most failure lines printed outside any case that an excerpt keeps.
const LEAD_FAILURES: usize = 3;

/// The most summary lines an excerpt keeps.
const SUMMARY_LINES: usize = 3;

/// Whether `line` says that something failed.
///
/// That is an error (`error: …`, `error[E0425]: …`, `x.cpp:3:1: error: …`,
/// `fatal error: …`), a failed assertion of a C++ test runner (doctest's
/// `x.cpp:9: ERROR: CHECK( … ) is NOT correct!`, Catch2's `x.cpp:9: FAILED:`,
/// GoogleTest's `x.cc:9: Failure`), a failed test (`FAILED`, CTest's
/// `***Failed`, `FAIL:`), a panic, a Python assertion or a traceback. A
/// warning is not one, and neither is a count of no failures
/// (`0 tests failed`).
#[must_use]
pub fn is_failure_line(line: &str) -> bool {
    let line = line.trim_start();
    let lower = line.to_ascii_lowercase();
    lower.starts_with("error:")
        || lower.starts_with("error[")
        || line.contains(": error:")
        || line.contains(": error[")
        || line.contains("fatal error:")
        || line.contains("ERROR:")
        || line.contains(": Failure")
        || line.contains("FAILED")
        || line.contains("***Failed")
        || line.contains("FAIL:")
        || line.contains("panicked at")
        || line.contains("AssertionError")
        || line.contains("Traceback (most recent call last)")
}

/// Whether `line` is build progress, which says nothing about a failure:
/// cargo's (`Compiling …`, `Running …`), CMake and make's (`[ 22%] Building
/// CXX object …`, `[100%] Built target …`) and ninja's (`[3/88] Linking …`).
#[must_use]
pub fn is_progress_line(line: &str) -> bool {
    let line = line.trim_start();
    let cargo = [
        "Compiling ",
        "Checking ",
        "Finished ",
        "Fresh ",
        "Blocking waiting",
        "Downloaded ",
        "Downloading ",
        "Running ",
        "Doc-tests ",
    ];
    if cargo.iter().any(|prefix| line.starts_with(prefix)) {
        return true;
    }
    let digits = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    match line
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .map(|(counter, _)| counter.trim())
    {
        Some(counter) => {
            counter.strip_suffix('%').is_some_and(digits)
                || counter
                    .split_once('/')
                    .is_some_and(|(done, all)| digits(done) && digits(all))
        }
        None => line.starts_with("Scanning dependencies of target"),
    }
}

/// Whether `line` is a test runner's summary: doctest's and Catch2's `test
/// cases:` and `assertions:` counts and doctest's `Status:`, cargo's `test
/// result:`, CTest's `96% tests passed, …` and `The following tests FAILED:`,
/// GoogleTest's `[==========]`, `[  PASSED  ]`, `[  FAILED  ] 2 tests, listed
/// below:` and ` 2 FAILED TESTS`, and pytest's `=== 1 failed, 4 passed in
/// 0.12s ===`.
#[must_use]
pub fn is_summary_line(line: &str) -> bool {
    let line = line.trim();
    let bare = line.strip_prefix("[doctest]").map_or(line, str::trim_start);
    let lower = line.to_ascii_lowercase();
    bare.starts_with("test cases:")
        || bare.starts_with("assertions:")
        || bare.starts_with("Status:")
        || lower.starts_with("test result:")
        || lower.contains("% tests passed")
        || line.starts_with("[==========]")
        || line.starts_with("[  PASSED  ]")
        || (line.starts_with("[  FAILED  ]") && lower.contains(" listed below"))
        || lower.ends_with("failed tests")
        || lower.ends_with("failed test")
        || lower.starts_with("the following tests failed")
        || (line.starts_with("===") && lower.contains(" failed") && lower.contains(" in "))
}

/// A failing test case of a check's output, with its failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedCase {
    /// The line that names the case: doctest's `TEST CASE:  …`, Catch2's
    /// name between its dashed rules, GoogleTest's `[ RUN      ] Suite.Test`,
    /// cargo's `---- name stdout ----` or pytest's `____ test_name ____`.
    pub header: String,
    /// Its failures, in order.
    pub failures: Vec<CheckFailure>,
}

/// One failure of a failing case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailure {
    /// The line that says it failed.
    pub line: String,
    /// The lines under it that say why: the values an assertion saw, what
    /// the test logged, an expansion, at most [`DETAIL_LINES`].
    pub detail: Vec<String>,
}

impl CheckFailure {
    /// The failed assertion without where it is: `CHECK( found != nullptr )`
    /// of doctest's `x.cpp:219: ERROR: CHECK( found != nullptr ) is NOT
    /// correct!`; the whole line, trimmed, when it has no such form.
    #[must_use]
    pub fn assertion(&self) -> &str {
        let line = self.line.trim();
        let said = [": ERROR: ", ": FATAL ERROR: ", ": FAILED:", ": Failure"]
            .iter()
            .find_map(|marker| line.split_once(marker).map(|(_, rest)| rest.trim()))
            .unwrap_or(line);
        said.strip_suffix(" is NOT correct!").unwrap_or(said)
    }

    /// Where it failed, as `file:line` without the file's directories:
    /// `layout_test.cpp:219`. `None` when its line starts with no location.
    #[must_use]
    pub fn location(&self) -> Option<String> {
        let (place, _) = self.line.trim().split_once(": ")?;
        let (path, number) = place.trim_end_matches(':').rsplit_once(':')?;
        let number = number.trim_start_matches('(').trim_end_matches(')');
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        file.contains('.').then(|| format!("{file}:{number}"))
    }

    /// What tells this failure apart from `first`, an earlier failure of its
    /// case: its assertion when that differs, else the first line of what it
    /// logged or its runner expanded. `None` when nothing does.
    fn apart_from(&self, first: &Self) -> Option<String> {
        if self.assertion() != first.assertion() {
            return Some(self.assertion().to_string());
        }
        self.detail
            .iter()
            .map(|line| line.trim())
            .find(|line| !line.is_empty() && !line.starts_with("values:"))
            .map(|line| {
                line.strip_prefix("logged:")
                    .unwrap_or(line)
                    .trim()
                    .to_string()
            })
    }
}

impl FailedCase {
    /// The case's name: its header without the runner's marks.
    #[must_use]
    pub fn name(&self) -> &str {
        let header = self.header.trim();
        let header = header
            .strip_prefix("TEST CASE:")
            .or_else(|| header.strip_prefix("[ RUN      ]"))
            .unwrap_or(header)
            .trim();
        let header = header
            .strip_prefix("----")
            .and_then(|name| name.strip_suffix("----"))
            .map_or(header, |name| name.trim().trim_end_matches(" stdout"));
        header.trim_matches('_').trim()
    }
}

/// The failing cases of `output`, in the order it printed them, each with
/// its failures: doctest, Catch2 and GoogleTest cases, cargo's failing tests
/// and pytest's. A case that printed no failure, as GoogleTest prints every
/// case it runs, is left out.
#[must_use]
pub fn failed_cases(output: &str) -> Vec<FailedCase> {
    parse(output).1
}

/// The failure lines `output` printed outside any case, and its failing
/// cases ([`failed_cases`]). A line outside a case is one printed before the
/// first case, or CTest's line for a test binary that failed
/// (`23/28 Test #23: spec_p5_demos ...***Failed`), which ends the case before
/// it: the binary that printed it is done.
fn parse(output: &str) -> (Vec<String>, Vec<FailedCase>) {
    let lines: Vec<&str> = output.lines().collect();
    let mut loose = Vec::new();
    let mut cases: Vec<FailedCase> = Vec::new();
    let mut open: Option<FailedCase> = None;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some((header, span)) = case_header(&lines, index) {
            cases.extend(open.take().filter(|case| !case.failures.is_empty()));
            open = Some(FailedCase {
                header,
                failures: Vec::new(),
            });
            index += span;
            continue;
        }
        index += 1;
        // GoogleTest closes a case, and lists the failed ones, with
        // `[  FAILED  ] Suite.Test`: its failures are its `Failure` lines.
        if !is_failure_line(line)
            || is_summary_line(line)
            || line.trim_start().starts_with("[  FAILED  ]")
        {
            continue;
        }
        if is_ctest_failure(line) {
            cases.extend(open.take().filter(|case| !case.failures.is_empty()));
            loose.push(line.trim_end().to_string());
            continue;
        }
        let Some(case) = open.as_mut() else {
            if cases.is_empty() {
                loose.push(line.trim_end().to_string());
            }
            continue;
        };
        // What follows a failure says why, up to a blank line, the next
        // failure or case, a runner's status line or a rule.
        let detail: Vec<String> = lines[index..]
            .iter()
            .take_while(|next| !ends_detail(next))
            .take(DETAIL_LINES)
            .map(|next| next.trim_end().to_string())
            .collect();
        index += detail.len();
        case.failures.push(CheckFailure {
            line: line.trim_end().to_string(),
            detail,
        });
    }
    cases.extend(open.filter(|case| !case.failures.is_empty()));
    (loose, cases)
}

/// Whether `line` is CTest's line for a test binary that failed:
/// `23/28 Test #23: spec_p5_demos ...***Failed    0.29 sec`.
fn is_ctest_failure(line: &str) -> bool {
    line.contains(" Test #") && (line.contains("***Failed") || line.contains("***Exception"))
}

/// The lines of `output` that say what failed, for a hand-back to read names
/// from (bug-32f73a): the failure lines outside any case, such as CTest's
/// line for each failed test binary, then each failing case's header and its
/// failures with the lines under them; without failing cases, each failure
/// line with the lines under it. A warning, build progress or a runner's
/// summary is never one of them: phase 5's rehearsal sent a failure to the
/// task whose file a clang warning named.
#[must_use]
pub fn failure_lines(output: &str) -> Vec<String> {
    let (loose, cases) = parse(output);
    if cases.is_empty() {
        let lines: Vec<&str> = output.lines().collect();
        let mut kept = Vec::new();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            index += 1;
            if !is_failure_line(line) || is_summary_line(line) {
                continue;
            }
            kept.push(line.trim_end().to_string());
            let detail: Vec<String> = lines[index..]
                .iter()
                .take_while(|next| !ends_detail(next))
                .take(DETAIL_LINES)
                .map(|next| next.trim_end().to_string())
                .collect();
            index += detail.len();
            kept.extend(detail);
        }
        return kept;
    }
    let mut kept = loose;
    for case in cases {
        kept.push(case.header);
        for failure in case.failures {
            kept.push(failure.line);
            kept.extend(failure.detail);
        }
    }
    kept
}

/// The case header at `index` of `lines`, and how many lines it spans.
fn case_header(lines: &[&str], index: usize) -> Option<(String, usize)> {
    let line = lines[index].trim();
    let rule = |line: &str| line.len() >= 10 && line.chars().all(|c| c == '-');
    if line.starts_with("TEST CASE:")
        || line.starts_with("[ RUN      ]")
        || (line.starts_with("---- ") && line.ends_with(" ----"))
        || (line.len() > 8 && line.starts_with("___") && line.ends_with("___"))
    {
        return Some((line.to_string(), 1));
    }
    // Catch2: a name between two dashed rules.
    if rule(line)
        && let (Some(name), Some(closing)) = (lines.get(index + 1), lines.get(index + 2))
        && !name.trim().is_empty()
        && !rule(name.trim())
        && rule(closing.trim())
    {
        return Some((name.trim().to_string(), 3));
    }
    None
}

/// Whether `line` ends what a failure's lines say: a blank line, another
/// failure, the next case, a runner's status line (`[       OK ]`,
/// `[doctest] …`) or a rule of dashes or equals signs.
fn ends_detail(line: &str) -> bool {
    let line = line.trim();
    line.is_empty()
        || is_failure_line(line)
        || line.starts_with('[')
        || line.starts_with("TEST CASE:")
        || line.chars().all(|c| c == '-' || c == '=')
}

/// The part of a failed check's `output` its retry needs, within about
/// `budget` characters; `None` when no line of it says that something
/// failed.
///
/// With failing cases, it keeps the failure lines printed outside any case,
/// such as CTest's `***Failed` lines, then each case's header and first
/// failure with the lines under it. As far as the budget goes, each case then
/// gets one `also:` line for each further failure that something tells
/// apart, and the runner's summary closes it. The first case shows even past
/// the budget, and a further failure never crowds out another case. Without
/// failing cases, it keeps the output from a few lines above its first
/// failure line, without build progress.
#[must_use]
pub fn failure_excerpt(output: &str, budget: usize) -> Option<String> {
    let (loose, cases) = parse(output);
    if cases.is_empty() {
        return plain_excerpt(output, budget);
    }
    let cost = |line: &str| line.chars().count() + 1;
    let mut lines: Vec<String> = loose.into_iter().take(LEAD_FAILURES).collect();
    let mut used: usize = lines.iter().map(|line| cost(line)).sum();
    let blocks: Vec<(Vec<String>, Vec<String>)> = cases
        .iter()
        .map(|case| {
            let first = &case.failures[0];
            let mut core = vec![case.header.clone(), first.line.clone()];
            core.extend(first.detail.iter().cloned());
            let mut also: Vec<String> = Vec::new();
            for failure in &case.failures[1..] {
                if let Some(apart) = failure.apart_from(first)
                    && !also.contains(&apart)
                {
                    also.push(apart);
                }
            }
            (core, also)
        })
        .collect();
    let mut shown = 0;
    for (core, _) in &blocks {
        let size = core.iter().map(|line| cost(line)).sum::<usize>() + 1;
        if shown > 0 && used + size > budget {
            break;
        }
        used += size;
        shown += 1;
    }
    let mut alsos = Vec::new();
    for (_, also) in &blocks[..shown] {
        let mut kept = Vec::new();
        for (n, apart) in also.iter().enumerate() {
            let line = format!("  also: {apart}");
            if used + cost(&line) > budget {
                kept.push(format!("  … and {} more like it", also.len() - n));
                break;
            }
            used += cost(&line);
            kept.push(line);
        }
        alsos.push(kept);
    }
    for ((core, _), also) in blocks.iter().zip(alsos) {
        lines.extend(core.iter().cloned());
        lines.extend(also);
        lines.push(String::new());
    }
    if shown < cases.len() {
        lines.push(format!(
            "[… {} more failing case(s) not shown]",
            cases.len() - shown
        ));
    }
    let summary: Vec<&str> = output
        .lines()
        .filter(|line| is_summary_line(line))
        .collect();
    let skip = summary.len().saturating_sub(SUMMARY_LINES);
    lines.extend(summary[skip..].iter().map(|line| line.trim().to_string()));
    let mut text = lines.join("\n");
    text.push('\n');
    Some(text)
}

/// The output of `output` from a few lines above its first failure line,
/// within `budget` characters and without build progress; `None` when no
/// line of it says that something failed.
fn plain_excerpt(output: &str, budget: usize) -> Option<String> {
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !is_progress_line(line))
        .collect();
    let first = lines.iter().position(|line| is_failure_line(line))?;
    let mut text = String::new();
    for line in &lines[first.saturating_sub(LEAD_LINES)..] {
        if text.chars().count() + line.chars().count() + 1 > budget {
            text.push_str("[…]\n");
            break;
        }
        text.push_str(line);
        text.push('\n');
    }
    Some(text)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// What `tools/spec.sh 4_2 layout` printed in Undertow's phase 4.2, run
    /// 1447385e, T1#1, re-run on the try's best checked tree (bug-3e3d08):
    /// the build, then 2 failing doctest cases, the second with 4 failures.
    pub(crate) const PHASE_4_2_LAYOUT: &str =
        include_str!("../tests/fixtures/phase_4_2_layout.txt");

    #[test]
    fn every_failing_doctest_case_is_read_with_its_failures() {
        let cases = failed_cases(PHASE_4_2_LAYOUT);

        let names: Vec<&str> = cases.iter().map(FailedCase::name).collect();
        assert_eq!(
            names,
            [
                "P4.2.01 every control sits in its own panel, clear of the CHEST, the pump field, \
                 the GUT and each other",
                "P4.2.04 every control has the design's short label, drawn under it",
            ]
        );
        assert_eq!(cases[0].failures.len(), 1);
        assert_eq!(cases[1].failures.len(), 4);
        let first = &cases[0].failures[0];
        assert_eq!(first.assertion(), "CHECK( !b.intersects(otherBounds) )");
        assert_eq!(first.location().as_deref(), Some("layout_test.cpp:109"));
        assert_eq!(first.detail.len(), 3, "{first:?}");
        assert!(
            first.detail[1].contains("`lfo2_phase` (MOD/05)"),
            "{first:?}"
        );
    }

    #[test]
    fn the_excerpt_keeps_each_failing_case_and_its_first_failure() {
        let excerpt = failure_excerpt(PHASE_4_2_LAYOUT, 2_400).expect("an excerpt");

        assert!(excerpt.starts_with("TEST CASE:  P4.2.01"), "{excerpt}");
        for needed in [
            "CHECK( !b.intersects(otherBounds) ) is NOT correct!",
            "`lfo2_phase` (MOD/05)",
            "its bounds 992 468 32 32 overlap `lfo2_shape`'s 988 444 40 40",
            "TEST CASE:  P4.2.04",
            "`osc_fine`, label \"FINE\"",
            "  also: `sub_level`, label \"SUB\"",
            "  also: `sub_octave`, label \"SUB OCT\"",
            "  also: `filter_track`, label \"TRACK\"",
            "[doctest] test cases:    5 |    3 passed | 2 failed | 0 skipped",
        ] {
            assert!(excerpt.contains(needed), "{needed} in:\n{excerpt}");
        }
        assert!(!excerpt.contains("Building CXX object"), "{excerpt}");
        assert!(excerpt.chars().count() < 2_400, "{excerpt}");

        // A budget that holds both cases' first failures and little more
        // keeps both cases and cuts the further failures; one that holds
        // less keeps the first case and says how many more failed.
        let cores: usize = failed_cases(PHASE_4_2_LAYOUT)
            .iter()
            .map(|case| {
                let first = &case.failures[0];
                let detail: usize = first
                    .detail
                    .iter()
                    .map(|line| line.chars().count() + 1)
                    .sum();
                case.header.chars().count() + first.line.chars().count() + detail + 3
            })
            .sum();
        let tight = failure_excerpt(PHASE_4_2_LAYOUT, cores + 10).expect("an excerpt");
        assert!(tight.contains("TEST CASE:  P4.2.04"), "{tight}");
        assert!(tight.contains("  … and 3 more like it"), "{tight}");
        let tighter = failure_excerpt(PHASE_4_2_LAYOUT, 100).expect("an excerpt");
        assert!(tighter.starts_with("TEST CASE:  P4.2.01"), "{tighter}");
        assert!(
            tighter.contains("[… 1 more failing case(s) not shown]"),
            "{tighter}"
        );
    }

    /// GoogleTest prints every case it runs, and its failures' lines follow
    /// unindented; Catch2 names a case between dashed rules; cargo prints a
    /// failing test's output under `---- name stdout ----`.
    #[test]
    fn googletest_catch2_and_cargo_cases_are_read() {
        let gtest = "[==========] Running 2 tests from 1 test suite.\n\
                     [ RUN      ] Layout.Overlap\n\
                     /src/layout_test.cc:42: Failure\n\
                     Expected equality of these values:\n  a\n    Which is: 1\n  b\n    Which is: 2\n\
                     [  FAILED  ] Layout.Overlap (0 ms)\n\
                     [ RUN      ] Layout.Labels\n\
                     [       OK ] Layout.Labels (0 ms)\n\
                     [==========] 2 tests from 1 test suite ran. (0 ms total)\n\
                     [  PASSED  ] 1 test.\n\
                     [  FAILED  ] 1 test, listed below:\n\
                     [  FAILED  ] Layout.Overlap\n\n 1 FAILED TEST\n";
        let cases = failed_cases(gtest);
        assert_eq!(cases.len(), 1, "{cases:?}");
        assert_eq!(cases[0].name(), "Layout.Overlap");
        assert_eq!(cases[0].failures.len(), 1, "{cases:?}");
        assert_eq!(
            cases[0].failures[0].detail[0],
            "Expected equality of these values:"
        );
        assert_eq!(
            cases[0].failures[0].location().as_deref(),
            Some("layout_test.cc:42")
        );

        let catch2 = "-------------------------------------------------------------------------------\n\
                      every control has a label\n\
                      -------------------------------------------------------------------------------\n\
                      /src/layout_test.cpp:12\n\
                      ...............................................................................\n\n\
                      /src/layout_test.cpp:15: FAILED:\n  REQUIRE( found != nullptr )\n\
                      with expansion:\n  nullptr != nullptr\n\n\
                      ===============================================================================\n\
                      test cases: 2 | 1 passed | 1 failed\nassertions: 9 | 8 passed | 1 failed\n";
        let cases = failed_cases(catch2);
        assert_eq!(cases.len(), 1, "{cases:?}");
        assert_eq!(cases[0].name(), "every control has a label");
        assert_eq!(
            cases[0].failures[0].detail,
            [
                "  REQUIRE( found != nullptr )",
                "with expansion:",
                "  nullptr != nullptr"
            ]
        );

        let cargo = "running 2 tests\ntest tests::adds ... ok\ntest tests::splits ... FAILED\n\n\
                     failures:\n\n---- tests::splits stdout ----\n\n\
                     thread 'tests::splits' panicked at src/lib.rs:9:5:\n\
                     assertion `left == right` failed\n  left: 1\n right: 2\n\n\
                     failures:\n    tests::splits\n\n\
                     test result: FAILED. 1 passed; 1 failed; 0 ignored\n";
        let cases = failed_cases(cargo);
        assert_eq!(cases.len(), 1, "{cases:?}");
        assert_eq!(cases[0].name(), "tests::splits");
        assert!(
            cases[0].failures[0]
                .line
                .contains("panicked at src/lib.rs:9:5"),
            "{cases:?}"
        );
        assert_eq!(cases[0].failures[0].detail.len(), 3, "{cases:?}");
    }

    /// Without a failing case, the excerpt runs from a few lines above the
    /// first failure line, build progress left out; with no failure line,
    /// there is none.
    #[test]
    fn an_output_without_cases_is_cut_from_its_first_failure() {
        let clippy = "    Checking alpha v0.1.0\nwarning: unused import\n\
                      error: items after a test module\n  --> crates/alpha/src/lib.rs:56:1\n";
        let excerpt = failure_excerpt(clippy, 400).expect("an excerpt");
        assert!(
            excerpt.starts_with("warning: unused import\nerror: items"),
            "{excerpt}"
        );
        assert_eq!(failure_excerpt("all good\n", 400), None);
        for (line, progress) in [
            (
                "[ 22%] Building CXX object plugin/CMakeFiles/Undertow.dir/ui/Look.cpp.o",
                true,
            ),
            ("[100%] Built target spec_p4_2_layout", true),
            ("[3/88] Linking CXX executable spec", true),
            ("   Compiling roko-cli v0.1.0", true),
            ("[doctest] test cases: 5 | 3 passed | 2 failed", false),
            ("[  FAILED  ] Layout.Overlap (0 ms)", false),
        ] {
            assert_eq!(is_progress_line(line), progress, "{line}");
        }
        for (line, summary) in [
            (
                "[doctest] test cases:    5 |    3 passed | 2 failed | 0 skipped",
                true,
            ),
            (
                "[doctest] assertions: 2777 | 2772 passed | 5 failed |",
                true,
            ),
            ("[doctest] Status: FAILURE!", true),
            ("test result: FAILED. 1 passed; 1 failed; 0 ignored", true),
            ("96% tests passed, 1 tests failed out of 28", true),
            ("The following tests FAILED:", true),
            ("[  FAILED  ] 1 test, listed below:", true),
            ("[  FAILED  ] Layout.Overlap", false),
            ("x_test.cpp:57: ERROR: CHECK( a ) is NOT correct!", false),
        ] {
            assert_eq!(is_summary_line(line), summary, "{line}");
        }
    }
}
