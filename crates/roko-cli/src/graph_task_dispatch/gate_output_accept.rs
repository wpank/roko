//! Acceptance: a failed verify step always says why. The output published with
//! each gate result is `$ <command>`, the tail of what the step printed (stdout,
//! then `---stderr---` and stderr), and — when the step failed — a closing `✗`
//! line with how it ended: `✗ exit status 1`, `✗ timed out after 600000 ms`.
//! Before, `test -f MISSING.md` published only its command.
//!
//! Copied verbatim from plans/portal-programme/08f-final-polish/accept/ to
//! crates/roko-cli/src/graph_task_dispatch/gate_output_accept.rs by the gate.

use roko_core::Verdict;

use super::published_gate_output;

fn failed(reason: &str, detail: &str) -> Verdict {
    Verdict::fail("verify[0:structural]", reason).with_detail(detail)
}

#[test]
fn a_silent_failure_says_how_it_ended() {
    assert_eq!(
        published_gate_output("test -f MISSING.md", &failed("exit code: 1", "")),
        "$ test -f MISSING.md\n✗ exit status 1"
    );
}

#[test]
fn a_failure_keeps_stdout_then_stderr_then_the_exit_status() {
    let detail = "   Compiling hello\n\n---stderr---\nerror: could not compile `hello`\n";
    assert_eq!(
        published_gate_output("cargo build", &failed("exit code: 101", detail)),
        "$ cargo build\n   Compiling hello\n\n---stderr---\nerror: could not compile `hello`\n✗ exit status 101"
    );
}

#[test]
fn a_timeout_a_signal_and_a_spawn_failure_say_so() {
    let timed_out = Verdict::fail("verify[0:test]", "timed out after 600000 ms");
    assert_eq!(
        published_gate_output("cargo test", &timed_out),
        "$ cargo test\n✗ timed out after 600000 ms"
    );
    assert_eq!(
        published_gate_output("sleep 9", &failed("exit code: terminated by signal", "")),
        "$ sleep 9\n✗ terminated by a signal"
    );
    let spawn = Verdict::fail("verify[0]", "spawn failed: No such file or directory");
    assert!(
        published_gate_output("x", &spawn).ends_with("\n✗ spawn failed: No such file or directory")
    );
}

#[test]
fn a_pass_publishes_its_output_and_no_ending() {
    let passed = Verdict::pass("verify[0:test]").with_detail("test result: ok\n");
    assert_eq!(
        published_gate_output("cargo test", &passed),
        "$ cargo test\ntest result: ok"
    );
    assert_eq!(
        published_gate_output("true", &Verdict::pass("verify[1]")),
        "$ true"
    );
}

#[test]
fn long_output_keeps_its_last_sixty_lines() {
    let detail: String = (1..=100).map(|n| format!("line {n}\n")).collect();
    let published = published_gate_output("make", &failed("exit code: 2", &detail));
    let lines: Vec<&str> = published.lines().collect();
    assert_eq!(lines[0], "$ make");
    assert_eq!(lines[1], "… 40 earlier lines not shown");
    assert_eq!(lines[2], "line 41");
    assert_eq!(lines[lines.len() - 2], "line 100");
    assert_eq!(lines[lines.len() - 1], "✗ exit status 2");
    assert_eq!(lines.len(), 63);
}

#[test]
fn huge_output_is_bounded() {
    let detail = "é".repeat(100_000);
    let published = published_gate_output("cat big", &failed("exit code: 1", &detail));
    assert!(
        published.len() <= 17 * 1024,
        "published {} bytes",
        published.len()
    );
    assert!(published.contains("\n… earlier output not shown\n"));
    assert!(published.ends_with("\n✗ exit status 1"));
}

#[test]
fn an_empty_reason_still_says_the_step_failed() {
    assert_eq!(
        published_gate_output("false", &failed("", "")),
        "$ false\n✗ failed"
    );
}
