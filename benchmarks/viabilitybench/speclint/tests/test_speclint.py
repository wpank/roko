"""Tests for speclint, the S07.1 static spec-quality linter.

Run from the repo root with the benchmark venv:
``benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint``.
"""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import tarfile
import tomllib
from pathlib import Path

import pytest

SPECLINT_DIR = Path(__file__).resolve().parents[1]
ROOT = SPECLINT_DIR.parents[2]
FIXTURES = SPECLINT_DIR / "fixtures"
FIXTURE_DIRS = sorted(p for p in FIXTURES.iterdir() if (p / "tasks.toml").is_file())

sys.path.insert(0, str(SPECLINT_DIR))
import speclint  # noqa: E402

# The prototype of S07 section 3.3 scored the plans tracked at this commit: 484 tasks, 323 archived.
PROTOTYPE_COMMIT = "725f21e05"


def lint(path: Path, root: Path, red_on_base: dict | None = None) -> dict[str, dict]:
    records, errors = speclint.lint_files([path], root, red_on_base)
    assert errors == []
    return {record["task_id"]: record for record in records}


@pytest.mark.parametrize("fixture", FIXTURE_DIRS, ids=[p.name for p in FIXTURE_DIRS])
def test_golden_fixture_per_rule(fixture):
    expected = json.loads((fixture / "expected.json").read_text())
    got = lint(fixture / "tasks.toml", fixture)
    assert sorted(got) == sorted(expected["tasks"])

    rule = expected["focus"]["rule"]
    for task_id, want in expected["focus"]["expect"].items():
        if rule.startswith("SQ"):
            assert got[task_id]["rules"][rule] == pytest.approx(want), task_id
        else:
            assert (rule in got[task_id]["hard_fail"]) is want, task_id

    for task_id, want in expected["tasks"].items():
        record = got[task_id]
        assert record["rules"] == pytest.approx(want["rules"]), task_id
        assert record["score"] == pytest.approx(want["score"]), task_id
        assert record["band"] == want["band"], task_id
        assert record["hard_fail"] == want["hard_fail"], task_id
        assert record["verify_classes"] == want["verify_classes"], task_id
        assert record["unknown"] == ["HF3", "SQ06"], task_id
        assert record["red_on_base"] == "unknown", task_id


def test_golden_fixtures_cover_every_rule_and_static_hard_fail():
    focused = [json.loads((p / "expected.json").read_text())["focus"]["rule"] for p in FIXTURE_DIRS]
    assert set(focused) == {*speclint.WEIGHTS, "HF1", "HF2", "HF4", "HF5"}


def test_accept_tests_count_as_verify_steps_and_acceptance():
    """A `[task.accept]` test is a scoped test-run verify step and an observable criterion (bug-019f02)."""
    fixture = FIXTURES / "accept-tests"
    got = lint(fixture / "tasks.toml", fixture)
    pinned, by_hand, with_own_step, malformed = got["T1"], got["T2"], got["T3"], got["T4"]
    assert pinned["verify_classes"] == ["test"]
    assert pinned["features"]["n_verify"] == pinned["features"]["n_accept"] == 1
    assert pinned["rules"]["SQ02"] == pinned["rules"]["SQ04"] == pinned["rules"]["SQ05"] == 1.0
    assert pinned["features"]["has_acceptance_fields"]
    assert pinned["hard_fail"] == []
    # The same test copied into a verify step by hand scores no better.
    assert pinned["score"] >= by_hand["score"]
    assert with_own_step["verify_classes"] == ["compile", "test"]
    # An entry the loader would reject (count = 0) compiles to no step.
    assert malformed["features"]["n_accept"] == 0
    assert malformed["hard_fail"] == ["HF1"]


def test_red_on_base_scores_sq06_and_hf3_when_supplied():
    fixture = FIXTURES / "sq06-red-on-base"
    got = lint(fixture / "tasks.toml", fixture, {("tasks.toml", "T1"): "fail", ("tasks.toml", "T2"): "pass"})
    assert got["T1"]["rules"]["SQ06"] == 1.0
    assert got["T1"]["unknown"] == []
    assert got["T2"]["hard_fail"] == ["HF3"]


def test_e2e_smoke_t02_is_band_d():
    got = lint(ROOT / "plans" / "e2e-smoke-test" / "tasks.toml", ROOT)
    t02 = got["T02"]
    assert t02["role"] == "scribe"
    assert t02["verify_classes"] == []
    assert t02["rules"]["SQ04"] == 0
    assert t02["band"] == "D"
    # HF1 is for implementers only.
    assert t02["hard_fail"] == []


def test_p11_cargo_test_no_run_steps_are_compile():
    data = tomllib.loads((ROOT / "plans/archive/P11-runner-v2-default/tasks.toml").read_text())
    commands = [
        step["command"]
        for task in data["task"]
        for step in task.get("verify", [])
        if "cargo test" in step["command"] and "--no-run" in step["command"]
    ]
    assert commands
    for command in commands:
        assert speclint.analyze_step(command).cls == "compile", command


def test_corpus_one_record_per_task_and_runs_differ_only_in_ts(tmp_path):
    runs = []
    for n in (1, 2):
        out = tmp_path / f"run{n}" / "speclint.jsonl"
        proc = subprocess.run(
            [sys.executable, str(SPECLINT_DIR / "speclint.py"), str(ROOT / "plans"), "--out", str(out)],
            capture_output=True,
            text=True,
            check=True,
        )
        assert ", 0 parse errors" in proc.stdout
        runs.append([json.loads(line) for line in out.read_text().splitlines()])

    files = speclint.discover([ROOT / "plans"])
    tasks = sum(len(tomllib.loads(path.read_text()).get("task", [])) for path in files)
    assert len(runs[0]) == tasks
    assert all(r["ev"] == "spec.quality" and r["linter"] == "sq-2" and r["ts"] for r in runs[0])

    def without_ts(records):
        return [{key: value for key, value in record.items() if key != "ts"} for record in records]

    assert without_ts(runs[0]) == without_ts(runs[1])


def test_default_output_goes_to_vb_results(tmp_path):
    env = {**os.environ, "VB_RESULTS": str(tmp_path)}
    fixture = FIXTURES / "sq01-goal"
    subprocess.run(
        [sys.executable, str(SPECLINT_DIR / "speclint.py"), str(fixture), "--root", str(fixture)],
        env=env,
        capture_output=True,
        check=True,
    )
    [out] = tmp_path.glob("speclint/*/speclint.jsonl")
    assert len(out.read_text().splitlines()) == 4


@pytest.mark.parametrize(
    "command, cls",
    [
        ("cargo test -p roko-cli --lib plan_validate", "test"),
        ("cargo test -p roko-serve --lib --no-run 2>&1 | tail -10", "compile"),
        ("cargo check -p roko-cli 2>&1 | tail -5", "compile"),
        ("grep -q 'cargo test --workspace' justfile && grep -q 'cargo clippy' justfile", "structural"),
        ("cd apps/portal && { [ -d node_modules ] || npm ci --silent; } && npm test", "test"),
        ("cd apps/portal && node scripts/vitest-min.mjs src/lib/runState.test.ts 56", "test"),
        ("cd apps/portal && ./node_modules/.bin/tsc --noEmit", "compile"),
        ("cargo run -p roko-cli --quiet -- plan queue show --help 2>&1 | grep -q 'show'", "run"),
        ("test \"$(sh demo/x/validate.sh)\" = 'demo: PASS'", "run"),
        ("cargo build -p roko-cli && bash plans/p/_harness/live-events-check.sh", "run"),
        ("python3 -m unittest tests.visible.test_migrate", "test"),
        ("python3 -c 'import json; json.load(open(\"a.json\"))'", "structural"),
        ("bash -n scripts/deploy.sh", "compile"),
        ("grep -c 'cmd_do' src/develop.rs | xargs -I{} test {} -le 4", "structural"),
        ("cargo test -p x || true", "vacuous"),
    ],
)
def test_step_classes(command, cls):
    assert speclint.analyze_step(command).cls == cls


def test_running_the_tasks_own_script_is_self_exit():
    step = speclint.analyze_step("bash scripts/migrate.sh", {"scripts/migrate.sh"})
    assert step.cls == "structural" and step.self_exit


@pytest.mark.parametrize(
    "command, vacuous",
    [
        ("true", True),
        (":", True),
        ("echo ok", True),
        ("  ", True),
        ("cargo test -p x || true", True),
        ("cargo test -p x || :", True),
        ("grep -q x f; exit 0", True),
        ("grep -q x f; echo done", True),
        ("grep -q x f && echo FAIL || echo PASS", True),
        ("test $(grep -c x f || true) -eq 3", False),
        ("cargo check -p x 2>&1 | tail -5", False),
        ("grep -q x f && echo ok", False),
        ('for h in a b; do grep -q "$h" f || exit 1; done', False),
        ("if [ -f x ]; then exit 1; else exit 0; fi", False),
        ("cd app && { [ -d node_modules ] || npm ci; } && npm test", False),
    ],
)
def test_vacuous_steps(command, vacuous):
    assert (speclint.vacuous_reason(command) is not None) is vacuous


def test_rule_rates_reproduce_the_prototype_on_its_corpus(tmp_path):
    """S07 section 7.1: the section 3.3 rule-level rates within 3 points (the mean is not a target)."""
    archive = subprocess.run(["git", "-C", str(ROOT), "archive", PROTOTYPE_COMMIT, "plans"], capture_output=True)
    if archive.returncode != 0:
        pytest.skip(f"commit {PROTOTYPE_COMMIT} is not in this checkout")
    with tarfile.open(fileobj=io.BytesIO(archive.stdout)) as tar:
        tar.extractall(tmp_path, filter="data")
    records, errors = speclint.lint_files(speclint.discover([tmp_path / "plans"]), tmp_path)
    assert errors == []
    assert len(records) == 484
    active = [r for r in records if not r["archived"]]
    assert len(active) == 161

    def pct(group, pred):
        return 100.0 * sum(1 for r in group if pred(r)) / len(group)

    rates = {
        "no acceptance criteria": (lambda r: not r["features"]["has_acceptance"], 81.6, 78.3),
        "strongest verify is structural": (lambda r: r["features"]["verify_max_class"] == "structural", 16.7, 21.7),
        "has a test-runner verify step": (lambda r: r["features"]["has_test_verify"], 27.7, 39.1),
        "no read_files": (lambda r: r["features"]["n_read_files"] == 0, 24.8, 11.2),
    }
    for name, (pred, all_rate, active_rate) in rates.items():
        assert pct(records, pred) == pytest.approx(all_rate, abs=3), name
        assert pct(active, pred) == pytest.approx(active_rate, abs=3), name
    assert not any(r["features"]["has_hidden_hook"] for r in records)
    no_run = [r for r in records if r["features"]["n_no_run"]]
    assert sum(r["features"]["n_no_run"] for r in no_run) == 33
