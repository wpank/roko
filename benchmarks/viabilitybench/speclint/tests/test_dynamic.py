"""Tests for speclint's dynamic mode (S07.2), the red-on-base check.

Run from the repo root with the benchmark venv:
``benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_dynamic.py``.

Every check here runs against the fixture directory in fixture mode or a small git repo built in
``tmp_path``, and every verify step is plain shell. No test runs a real plan's steps or touches
this checkout's worktrees.
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

import pytest

SPECLINT_DIR = Path(__file__).resolve().parents[1]
FIXTURE = SPECLINT_DIR / "fixtures" / "dynamic" / "red-on-base"
EXPECTED = json.loads((FIXTURE / "expected.json").read_text())

sys.path.insert(0, str(SPECLINT_DIR))
import dynamic  # noqa: E402
import speclint  # noqa: E402

GIT_CONFIG = [
    arg
    for setting in (
        "user.name=speclint-test",
        "user.email=speclint-test@localhost",
        "commit.gpgsign=false",
        "core.hooksPath=/dev/null",
        "init.defaultBranch=main",
    )
    for arg in ("-c", setting)
]

# For the repos built here. T1's step writes a file, then fails until greet.sh exists; T2's step
# already passes.
PLAN = """
[meta]
plan = "p"

[[task]]
id = "T1"
title = "Greeting script"
role = "implementer"
files = ["greet.sh"]

[[task.verify]]
phase = "test"
command = "echo ran > ran.txt && bash greet.sh | grep -qx 'hello, world'"

[[task]]
id = "T2"
title = "Retry limit"
role = "implementer"
files = ["config.ini"]

[[task.verify]]
phase = "structural"
command = "grep -q '^retries' config.ini"
"""


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *GIT_CONFIG, *args], check=True, capture_output=True, text=True).stdout


def write(root: Path, files: dict[str, str]) -> None:
    for rel, text in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)


def commit(repo: Path, files: dict[str, str], message: str) -> str:
    write(repo, files)
    git(repo, "add", "--all")
    git(repo, "commit", "--quiet", "-m", message)
    return git(repo, "rev-parse", "HEAD").strip()


def make_repo(path: Path, files: dict[str, str]) -> Path:
    path.mkdir(parents=True)
    git(path, "init", "--quiet")
    commit(path, files, "base")
    return path


def repo_state(repo: Path) -> dict:
    """What a check must leave as it found it: worktrees, refs, the checkout, worktree admin dirs."""
    admin = Path(git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()) / "worktrees"
    return {
        "worktrees": git(repo, "worktree", "list", "--porcelain"),
        "refs": git(repo, "for-each-ref", "--format=%(refname) %(objectname)"),
        "status": git(repo, "status", "--porcelain", "--untracked-files=all"),
        "admin": sorted(p.name for p in admin.iterdir()) if admin.is_dir() else [],
    }


def lint_repo(repo: Path, **kwargs) -> dict[str, dict]:
    records, errors = dynamic.lint(speclint.discover([repo / "plans"]), repo, **kwargs)
    assert errors == []
    return {record["task_id"]: record for record in records}


@pytest.fixture(scope="module")
def fixture_records(tmp_path_factory) -> dict[str, dict]:
    """The fixture, checked once in fixture mode: {"<plan_path> <task_id>": record}."""
    scratch = tmp_path_factory.mktemp("scratch")
    counter = tmp_path_factory.mktemp("flaky") / "counter"
    with pytest.MonkeyPatch.context() as patch:
        patch.setenv("SPECLINT_FLAKY_COUNTER", str(counter))
        records, errors = dynamic.lint(speclint.discover([FIXTURE]), FIXTURE, fixture=True, scratch=scratch)
    assert errors == []
    assert list(scratch.iterdir()) == []  # the snapshot repo and its worktree are gone
    return {f"{record['plan_path']} {record['task_id']}": record for record in records}


@pytest.mark.parametrize("key", sorted(EXPECTED["tasks"]))
def test_fixture_outcomes(fixture_records, key):
    want = EXPECTED["tasks"][key]
    got = fixture_records[key]
    assert got["mode"] == "dynamic"
    assert got["red_on_base"] == want["red_on_base"]
    assert got["red_on_base_detail"]["outcome"] == want["outcome"], got["red_on_base_detail"]["reason"]
    assert got["rules"]["SQ06"] == want["SQ06"]
    assert ("HF3" in got["hard_fail"]) is want["HF3"]
    assert got["unknown"] == (["HF3", "SQ06"] if want["red_on_base"] == "unknown" else [])


def test_fixture_expectations_cover_every_task(fixture_records):
    assert sorted(fixture_records) == sorted(EXPECTED["tasks"])


def test_verify_passing_on_base_is_hf3(fixture_records):
    record = fixture_records["tasks.toml T2"]
    assert record["red_on_base"] == "pass"
    assert record["hard_fail"] == ["HF3"]
    assert record["rules"]["SQ06"] == 0
    assert record["unknown"] == []
    detail = record["red_on_base_detail"]
    assert detail["outcome"] == "pass"
    assert detail["base"] and detail["base_reason"] == "HEAD: the plan has not run"
    assert [[step["exit"] for step in run] for run in detail["runs"]] == [[0], [0]]


def test_verify_failing_on_base_scores_sq06(fixture_records):
    record = fixture_records["tasks.toml T1"]
    assert record["red_on_base"] == "fail"
    assert record["rules"]["SQ06"] == 1.0
    assert record["hard_fail"] == []
    assert record["unknown"] == []
    runs = record["red_on_base_detail"]["runs"]
    assert len(runs) == 2
    assert all(run[-1]["step"] == 1 and run[-1]["exit"] != 0 for run in runs)
    assert "greet.sh" in runs[0][-1]["tail"]  # bash's "No such file or directory"


def test_flaky_verify_is_unknown(fixture_records):
    record = fixture_records["tasks.toml T3"]
    assert record["red_on_base"] == "unknown"
    assert record["rules"]["SQ06"] == 0
    assert record["unknown"] == ["HF3", "SQ06"]
    assert "HF3" not in record["hard_fail"]
    detail = record["red_on_base_detail"]
    assert detail["outcome"] == "flaky"
    assert [run[-1]["exit"] for run in detail["runs"]] == [0, 1]
    assert detail["reason"] == "the runs disagree: run 1 passes, run 2 fails at step 1"


def test_timed_out_step_is_unknown_and_not_run_again(fixture_records):
    detail = fixture_records["tasks.toml T4"]["red_on_base_detail"]
    assert detail["outcome"] == "timeout"
    [[step]] = detail["runs"]
    assert step["timed_out"] and step["exit"] is None
    assert step["secs"] < 10  # killed at the step's 500 ms timeout_ms, not after the 30 s sleep


def test_only_implementer_tasks_run(fixture_records):
    detail = fixture_records["tasks.toml T5"]["red_on_base_detail"]
    assert detail["outcome"] == "not_run" and detail["runs"] == []


def test_failing_pass_on_base_step_is_unknown(fixture_records):
    detail = fixture_records["tasks.toml T6"]["red_on_base_detail"]
    assert detail["outcome"] == "base_broken"
    assert all(len(run) == 1 for run in detail["runs"])  # step 2 never ran


def test_each_run_starts_from_the_clean_base(fixture_records):
    """T7's step appends to config.ini and passes only when the previous run left nothing behind."""
    assert fixture_records["tasks.toml T7"]["red_on_base"] == "pass"
    assert (FIXTURE / "config.ini").read_text() == "retries = 3\n"


def test_archived_plan_without_base_is_unknown(fixture_records):
    detail = fixture_records["archive/old-plan/tasks.toml T1"]["red_on_base_detail"]
    assert detail["outcome"] == "no_base"
    assert detail["base"] is None and detail["runs"] == []
    assert "--base" in detail["reason"]


def test_removes_only_its_own_worktrees(tmp_path):
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n", "plans/p/tasks.toml": PLAN})
    # Someone else's worktrees: one on a branch, one detached with local work, and one whose
    # directory is gone (prunable). The check must leave all three as they are.
    git(repo, "worktree", "add", "--quiet", "-b", "other", str(tmp_path / "wt-branch"))
    git(repo, "worktree", "add", "--quiet", "--detach", str(tmp_path / "wt-detached"))
    (tmp_path / "wt-detached" / "notes.txt").write_text("keep\n")
    git(repo, "worktree", "add", "--quiet", "--detach", str(tmp_path / "wt-gone"))
    shutil.rmtree(tmp_path / "wt-gone")
    scratch = tmp_path / "scratch"
    scratch.mkdir()
    (scratch / "keep.txt").write_text("keep\n")
    before = repo_state(repo)
    assert "prunable" in before["worktrees"]

    got = lint_repo(repo, scratch=scratch)

    assert {task: record["red_on_base"] for task, record in got.items()} == {"T1": "fail", "T2": "pass"}
    assert repo_state(repo) == before  # no worktree, branch or admin entry left, none pruned
    assert not (repo / "ran.txt").exists()  # T1's step wrote it in the base checkout only
    assert sorted(p.name for p in scratch.iterdir()) == ["keep.txt"]
    assert (tmp_path / "wt-detached" / "notes.txt").read_text() == "keep\n"
    assert git(tmp_path / "wt-detached", "status", "--porcelain") == "?? notes.txt\n"


def test_interrupted_run_still_removes_its_worktrees(tmp_path, monkeypatch):
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n", "plans/p/tasks.toml": PLAN})
    scratch = tmp_path / "scratch"
    scratch.mkdir()
    before = repo_state(repo)
    seen = []

    def interrupt(command, cwd, timeout):
        seen.append(git(repo, "worktree", "list", "--porcelain").count("worktree "))
        raise KeyboardInterrupt

    monkeypatch.setattr(dynamic, "run_step", interrupt)
    with pytest.raises(KeyboardInterrupt):
        lint_repo(repo, scratch=scratch)
    assert seen == [2]  # the base checkout existed when the run was interrupted
    assert repo_state(repo) == before
    assert list(scratch.iterdir()) == []


def test_terminated_run_kills_its_step_and_removes_its_worktrees(tmp_path):
    """A SIGTERM mid-step (a CI timeout, say) still kills the step and removes the base checkout."""
    started = tmp_path / "started"
    # `sleep 60 & wait` keeps bash alive as the parent of a child, so killing bash alone would
    # leave the sleep behind.
    slow = PLAN.replace("grep -q '^retries' config.ini", 'echo $$ > \\"$SPECLINT_TEST_STARTED\\"; sleep 60 & wait')
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n", "plans/p/tasks.toml": slow})
    scratch = tmp_path / "scratch"
    scratch.mkdir()
    before = repo_state(repo)
    args = [str(repo / "plans"), "--root", str(repo), "--dynamic", "--scratch", str(scratch), "--out", "-"]
    proc = subprocess.Popen(
        [sys.executable, str(SPECLINT_DIR / "speclint.py"), *args],
        env={**os.environ, "SPECLINT_TEST_STARTED": str(started)},
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    deadline = time.monotonic() + 60
    while not (started.exists() and started.read_text().strip()):
        assert proc.poll() is None and time.monotonic() < deadline, "the sleeping step never started"
        time.sleep(0.05)

    proc.send_signal(signal.SIGTERM)
    _, stderr = proc.communicate(timeout=60)

    assert proc.returncode == 128 + signal.SIGTERM, stderr
    step_group = int(started.read_text())  # bash -c is its process group's leader
    deadline = time.monotonic() + 10
    while True:
        try:
            os.killpg(step_group, 0)
        except ProcessLookupError:
            break
        assert time.monotonic() < deadline, "the step's processes outlived the run"
        time.sleep(0.05)
    assert repo_state(repo) == before
    assert list(scratch.iterdir()) == []


def test_plan_whose_files_changed_after_it_was_added_needs_a_base(tmp_path):
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n"})
    planned = commit(repo, {"plans/p/tasks.toml": PLAN}, "plan p")
    commit(repo, {"greet.sh": "echo 'hello, world'\n"}, "T1 lands")

    got = lint_repo(repo, scratch=tmp_path)
    assert {record["red_on_base_detail"]["outcome"] for record in got.values()} == {"no_base"}
    assert "--base" in got["T1"]["red_on_base_detail"]["reason"]

    got = lint_repo(repo, base=planned, scratch=tmp_path)
    assert {task: record["red_on_base"] for task, record in got.items()} == {"T1": "fail", "T2": "pass"}
    assert {record["red_on_base_detail"]["base"] for record in got.values()} == {planned}


def test_uncommitted_plan_brings_its_own_directory(tmp_path):
    """The plan's harness is not in the base commit; the check copies the plan directory in."""
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n"})
    plan = PLAN.replace("grep -q '^retries' config.ini", "bash plans/new/check.sh")
    write(repo, {"plans/new/tasks.toml": plan, "plans/new/check.sh": "grep -q '^retries' config.ini\n"})

    got = lint_repo(repo, scratch=tmp_path)
    assert got["T2"]["red_on_base"] == "pass"  # without the copy, bash would exit 127
    assert got["T2"]["red_on_base_detail"]["base_reason"] == "HEAD: the plan has not run"
    assert git(repo, "status", "--porcelain", "--untracked-files=all") == "?? plans/new/check.sh\n?? plans/new/tasks.toml\n"


def test_refuses_a_scratch_directory_inside_a_worktree(tmp_path):
    repo = make_repo(tmp_path / "repo", {"config.ini": "retries = 3\n", "plans/p/tasks.toml": PLAN})
    git(repo, "worktree", "add", "--quiet", "--detach", str(tmp_path / "wt"))
    (repo / "scratch").mkdir()
    before = repo_state(repo)
    for scratch in (repo / "scratch", tmp_path / "wt"):
        with pytest.raises(dynamic.CheckError, match="inside the checkout"):
            lint_repo(repo, scratch=scratch)
    assert repo_state(repo) == before
    assert list((repo / "scratch").iterdir()) == []


def test_cli_dynamic_fixture_mode(tmp_path):
    out = tmp_path / "speclint.jsonl"
    env = {**os.environ, "SPECLINT_FLAKY_COUNTER": str(tmp_path / "counter")}
    args = ["--dynamic", "--fixture", "--scratch", str(tmp_path), "--out", str(out), "--strict"]
    proc = subprocess.run(
        [sys.executable, str(SPECLINT_DIR / "speclint.py"), str(FIXTURE), *args],
        env=env,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 1, proc.stderr  # HF3 is a hard fail under --strict
    assert "speclint sq-2 (dynamic): 2 files, 8 tasks" in proc.stdout
    assert "Red on base (tasks)" in proc.stdout
    records = [json.loads(line) for line in out.read_text().splitlines()]
    got = {f"{r['plan_path']} {r['task_id']}": r["red_on_base"] for r in records}
    assert got == {key: want["red_on_base"] for key, want in EXPECTED["tasks"].items()}
    assert all(r["mode"] == "dynamic" and r["ts"] for r in records)
    assert sorted(p.name for p in tmp_path.iterdir()) == ["counter", "speclint.jsonl"]


def test_dynamic_options_need_dynamic():
    proc = subprocess.run(
        [sys.executable, str(SPECLINT_DIR / "speclint.py"), str(FIXTURE), "--base", "HEAD", "--out", "-"],
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 2
    assert "need --dynamic" in proc.stderr
