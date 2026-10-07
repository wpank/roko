"""Offline tests of the Codex CLI runner (`run_codex.py`, arm fd_codex): a fake `codex`, the toy family, no spend.

No test can reach a real `codex`: PATH holds only the fake's directory and the system directories, the arm files the
runs use name the fake by its absolute path, and an offline run's egress proxy admits no host. The operator's login
is a fake `auth.json` under a temporary `CODEX_HOME`. Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_codex.py -q
"""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest

import agent_env
import caps
import harness
import layout
import ledger
import provider
import run_cli
import run_codex
import secret
import validate
import vb
from common import canary
from test_run_cli import CORRECT, LOOPBACK, TOY_STREAM, places, read_jsonl  # noqa: F401 (places is a fixture)

MODEL = "gpt-5.5"
VERSION = "0.152.0"
# One turn's usage in `codex exec --json`'s `turn.completed` shape (OpenAI's convention: the cached input is inside
# input_tokens, the reasoning inside output_tokens).
USAGE = {"input_tokens": 52_000, "cached_input_tokens": 31_000, "cache_write_input_tokens": 0, "output_tokens": 4_200,
         "reasoning_output_tokens": 2_600}
# U′ at the snapshot's gpt-5.5 row: 5.00 input, 0.50 cached input, 30.00 output (reasoning inside it).
U_PRIME = (21_000 * 5.00 + 31_000 * 0.50 + 4_200 * 30.00) / 1e6
WITHOUT_CACHE = (52_000 * 5.00 + 4_200 * 30.00) / 1e6
LOGIN = {"OPENAI_API_KEY": "sk-not-a-real-key-7c41", "tokens": {"id_token": "fake-id", "access_token": "fake-access",
                                                                "refresh_token": "fake-refresh", "account_id": "acct"},
         "last_refresh": "2026-10-03T09:00:00Z"}

FAKE_CODEX = r'''#!/usr/bin/env python3
"""A stand-in for `codex exec --json`: note what it was given, write a rollout, then play one scenario."""
import json
import os
import sys
import time

with open(__CONFIG__) as handle:
    CONFIG = json.load(handle)
if sys.argv[1:] == ["--version"]:
    print("codex-cli " + CONFIG["version"])
    sys.exit(0)
prompt = sys.stdin.read()
home = os.environ.get("CODEX_HOME", "")
auth = os.path.join(home, "auth.json")
names = sorted(os.path.normpath(os.path.join(base, name)) for base, dirs, files in os.walk(".")
               for name in dirs + files)
seen = {"argv": sys.argv[1:], "cwd": os.getcwd(), "env": dict(os.environ), "prompt": prompt, "names": names,
        "codex_home": sorted(os.listdir(home)) if os.path.isdir(home) else None,
        "auth": json.load(open(auth)) if os.path.isfile(auth) else None}
with open(CONFIG["log"], "a") as handle:
    handle.write(json.dumps(seen) + "\n")


def emit(event):
    print(json.dumps(event), flush=True)


scenario = CONFIG["scenario"]
model = sys.argv[sys.argv.index("--model") + 1]
emit({"type": "thread.started", "thread_id": "fake-thread"})
sessions = os.path.join(home, "sessions", "2026", "10", "03")
os.makedirs(sessions, exist_ok=True)
with open(os.path.join(sessions, "rollout-2026-10-03T09-00-00-fake-thread.jsonl"), "w") as handle:
    handle.write(json.dumps({"timestamp": "2026-10-03T09:00:00Z", "type": "session_meta",
                             "payload": {"id": "fake-thread", "cli_version": CONFIG["version"]}}) + "\n")
    handle.write(json.dumps({"timestamp": "2026-10-03T09:00:01Z", "type": "turn_context",
                             "payload": {"cwd": os.getcwd(), "model": CONFIG.get("served") or model,
                                         "effort": "high"}}) + "\n")
emit({"type": "turn.started"})
if scenario == "hang":
    time.sleep(120)
if scenario == "fail":
    emit({"type": "error", "message": "The model is not available on your plan."})
    emit({"type": "turn.failed", "error": {"message": "The model is not available on your plan."}})
    sys.exit(1)
if scenario == "web":
    item = {"id": "item_w", "type": "web_search", "query": "clamp hidden suite"}
    emit({"type": "item.started", "item": item})
    emit({"type": "item.completed", "item": item})
for path, text in CONFIG["files"].items():
    with open(path, "w") as handle:
        handle.write(text)
emit({"type": "item.completed", "item": {"id": "item_1", "type": "file_change", "status": "completed",
                                         "changes": [{"path": "calc/ops.py", "kind": "update"}]}})
emit({"type": "item.completed", "item": {"id": "item_2", "type": "agent_message", "text": "Done."}})
emit({"type": "turn.completed", "usage": CONFIG["usage"]})
'''


@pytest.fixture
def login(places, monkeypatch) -> Path:
    """The operator's Codex home, with a fake subscription login that also carries an API key."""
    home = places["tmp"] / "operator-codex"
    home.mkdir()
    (home / "auth.json").write_text(json.dumps(LOGIN))
    monkeypatch.setenv("CODEX_HOME", str(home))
    monkeypatch.setenv("CODEX_API_KEY", LOGIN["OPENAI_API_KEY"])  # the driver may hold it; codex must never see it
    return home


def fake_codex(places: dict[str, Path], scenario: str, **extra: object) -> tuple[Path, Path]:
    """Put the fake `codex` for `scenario` on PATH, with `extra` in its config; returns it and the log of what it was
    given."""
    log, config = places["tmp"] / f"codex-{scenario}.jsonl", places["tmp"] / f"codex-{scenario}.json"
    config.write_text(json.dumps({"scenario": scenario, "log": str(log), "version": VERSION, "usage": USAGE,
                                  "files": {"calc/ops.py": CORRECT}, **extra}))
    program = places["bin"] / "codex"
    program.write_text(FAKE_CODEX.replace("__CONFIG__", repr(str(config))))
    program.chmod(0o755)
    return program, log


def arm_file(places: dict[str, Path], program: Path, **overrides: object) -> str:
    """arms/fd_codex.toml with the fake as its program and some caps changed, written outside arms/."""
    text = (layout.ARMS_DIR / "fd_codex.toml").read_text()
    text = text.replace('program = "codex"', f'program = "{program}"', 1)
    assert f'program = "{program}"' in text
    for name, value in overrides.items():
        text, count = re.subn(rf"(?m)^{name} = \S+", f"{name} = {value}", text)
        assert count == 1, name
    path = places["tmp"] / "fd_codex.test.toml"
    path.write_text(text)
    return str(path)


def run_vb(places: dict[str, Path], arm: str, *extra: str) -> int:
    return vb.main(["run", "--experiment", "TEST-CODEX", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm", arm,
                    "--model", MODEL, "--seeds", "1", "--limit", "1", "--provider-url", LOOPBACK,
                    "--results", str(places["results"]), "--work", str(places["work"]),
                    "--secret-file", str(places["secret"]), *extra])


def record_of(places: dict[str, Path]) -> dict:
    [record] = read_jsonl(places["results"] / "TEST-CODEX" / "run-1" / "records.jsonl")
    assert validate.validate("run-record", record) == []
    return record


def test_codex_runner_prices_turn_completed_usage(places, login):
    program, log = fake_codex(places, "solve")
    arm = vb.load_arm("fd_codex")
    assert arm["arm"]["models_allow"] == [MODEL] and arm["arm"]["billed"] is False
    assert vb.load_runner(arm) is run_codex and run_codex.CodexConfig.from_table(arm["cli"]).egress_allow == (
        "chatgpt.com:443", "auth.openai.com:443")
    assert run_vb(places, arm_file(places, program), "--transcripts") == 0
    record = record_of(places)
    assert record["arm"] == "fd_codex" and record["execution"]["status"] == "completed"
    assert record["vs"]["label"] == 1  # the fake wrote a correct clamp, and the census re-ran it
    [attempt] = record["execution"]["attempts"]
    assert attempt["model_reported"] == MODEL and attempt["provider"] == "openai" and attempt["turns"] == 1
    assert attempt["usage"] == {"tokens_in": 21_000, "tokens_out": 4_200, "tokens_cache_read": 31_000,
                                "tokens_reasoning": 2_600}
    assert record["costs"] == {"api_equiv_usd": pytest.approx(U_PRIME), "billed_usd": 0.0,
                               "without_cache_usd": pytest.approx(WITHOUT_CACHE), "vendor_usd": None,
                               "source": "cli_usage", "meter_cross_check_usd": None, "by_class": None}
    cli = attempt["cli"]
    assert cli["u_prime_usd"] == pytest.approx(U_PRIME) and cli["r_usd"] is None
    assert cli["cli_version"] == VERSION == cli["rollout_cli_version"]
    assert cli["turn_models"] == [MODEL] and cli["model_source"] == "rollout" and cli["priced_at"] == [MODEL]
    assert cli["turn_usage"] == [USAGE] and cli["killed"] is None and cli["errors"] == []

    # The record holds the exact argv and environment codex got.
    [seen] = read_jsonl(log)
    assert cli["argv"] == [str(program), *seen["argv"]]
    flags = seen["argv"]
    assert flags[:2] == ["exec", "--json"] and flags[-1] == "-"
    for flag in ("--skip-git-repo-check", "--ignore-user-config", "--ignore-rules",
                 "--dangerously-bypass-approvals-and-sandbox"):
        assert flags.count(flag) == 1, flag
    assert flags[flags.index("--model") + 1] == MODEL
    assert Path(flags[flags.index("--cd") + 1]).resolve() == Path(seen["cwd"]).resolve()
    overrides = [flags[index + 1] for index, flag in enumerate(flags) if flag == "-c"]
    assert overrides == ['web_search="disabled"', 'model_reasoning_effort="high"']
    for banned in ("--full-auto", "--profile", "-p", "--oss", "--search", "resume", "--sandbox"):
        assert banned not in flags
    # A fresh Codex home under the session's own HOME, holding only the subscription login, without its API key.
    env = seen["env"]
    home = Path(env["HOME"])
    assert env["CODEX_HOME"] == str(home / ".codex") and home != Path.home() and seen["codex_home"] == ["auth.json"]
    assert seen["auth"] == {**LOGIN, "OPENAI_API_KEY": None}
    assert not [name for name in env if agent_env.FORBIDDEN_NAME.search(name)]  # no VB_*, key or token
    assert LOGIN["OPENAI_API_KEY"] not in "".join(env.values())
    for needle in secret.load(places["secret"]).needles:
        assert needle not in "".join(env.values())
    assert env["HTTPS_PROXY"].startswith("http://127.0.0.1:")  # its own egress proxy, which admits no host offline
    assert record["provenance"]["network_policy"]["egress"]["allow"] == []
    assert "Toy instance" in seen["prompt"] and canary.RELEASE_CANARY not in seen["prompt"]
    assert ".vb" not in seen["names"] and "calc/ops.py" in seen["names"]

    out = places["results"] / "TEST-CODEX" / "run-1"
    [row] = read_jsonl(out / "ledger.jsonl")
    assert validate.validate("ledger", row) == [] and row["source"] == "cli_usage" and row["billed_usd"] == 0.0
    assert row["api_equiv_usd"] == pytest.approx(U_PRIME) and row["model_reported"] == MODEL
    [reserved] = read_jsonl(out / "reservations.jsonl")  # reserved before codex started, $0 on the subscription
    assert (reserved["event"], reserved["attempt_key"], reserved["reserved_usd"]) == ("reserve", row["attempt_key"], 0)
    manifest = json.loads((out / "manifest.json").read_text())
    assert manifest["config"]["prompt"] == {"version": run_codex.PROMPT_VERSION, "sha256": run_codex.PROMPT_SHA256}
    assert not (places["work"] / "run-1").exists()  # the workdir and its Codex home are gone

    # The pricing rules on their own: per-turn models, one model for several turns, the pin, and unknowns.
    snapshot = ledger.load_snapshot()
    one = run_codex.turn_usage(USAGE)
    assert run_codex.turn_usage({"input_tokens": 10}) is None and run_codex.turn_usage(None) is None
    usage, cost, priced_at = run_codex.price_turns([one, one], [MODEL, "gpt-5.4"], snapshot, MODEL)
    gpt54 = (21_000 * 2.50 + 31_000 * 0.25 + 4_200 * 15.00) / 1e6
    assert cost.source == "cli_usage" and cost.api_equiv_usd == pytest.approx(U_PRIME + gpt54)
    assert priced_at == [MODEL, "gpt-5.4"] and usage["tokens_in"] == 42_000 and usage["tokens_reasoning"] == 5_200
    assert run_codex.price_turns([one, one], [], snapshot, MODEL)[1].api_equiv_usd == pytest.approx(2 * U_PRIME)
    assert run_codex.price_turns([one, one], ["gpt-5.4"], snapshot, MODEL)[2] == ["gpt-5.4", "gpt-5.4"]
    assert run_codex.price_turns([one, one, one], [MODEL, "gpt-5.4"], snapshot, MODEL)[1].source == "unknown"
    assert run_codex.price_turns([one], ["gpt-mystery-9"], snapshot, MODEL)[1].source == "unknown"
    assert run_codex.price_turns([one, None], [], snapshot, MODEL)[:2] == (None, ledger.Cost(None, None, "unknown"))
    meter = run_codex.Meter(snapshot, MODEL)
    for event in ({"type": "item.completed", "item": {"type": "agent_message"}}, {"type": "turn.completed",
                                                                                  "usage": USAGE}):
        meter.observe(event)
    assert meter.spent_usd() == pytest.approx(U_PRIME)


@pytest.mark.parametrize("scenario, extra, reason", [
    ("fail", {}, "codex reported turn.failed: The model is not available on your plan."),
    ("web", {}, "web_tools"),
    ("switch", {"served": "gpt-5.4"}, "ended"),
])
def test_a_failed_turn_a_web_search_or_a_model_switch_is_an_infra_error(places, login, scenario, extra, reason):
    program, _ = fake_codex(places, scenario, **extra)
    assert run_vb(places, arm_file(places, program)) == 0
    record = record_of(places)
    assert record["execution"]["status"] == "infra_error"  # the switch through `records.final_status`
    assert record["execution"]["reason"] == reason
    [attempt] = record["execution"]["attempts"]
    if scenario == "switch":
        assert attempt["model_reported"] == "gpt-5.4" and attempt["cli"]["priced_at"] == ["gpt-5.4"]
    if scenario == "fail":
        assert attempt["api_equiv_usd"] is None and attempt["cli"]["errors"] == [
            "The model is not available on your plan."] * 2


def test_the_wallclock_limit_kills_a_hung_session(places, login):
    program, _ = fake_codex(places, "hang")
    assert run_vb(places, arm_file(places, program, wallclock_s=3)) == 0
    record = record_of(places)
    assert record["execution"]["status"] == "timeout"
    [attempt] = record["execution"]["attempts"]
    assert attempt["cli"]["killed"] == "wallclock" and record["costs"]["source"] == "unknown"  # no turn completed


def test_setup_refuses_an_api_key_login_flaky_verify_and_a_missing_codex(places, login):
    program, log = fake_codex(places, "solve")
    arm = vb.load_arm(arm_file(places, program))
    cli = run_codex.CodexConfig.from_table(arm["cli"])
    with pytest.raises(run_cli.CliError, match="unknown"):
        run_codex.CodexConfig.from_table({"fallback_model": "gpt-5.4"})
    with pytest.raises(run_cli.CliError, match="effort"):
        run_codex.CodexConfig.from_table({"effort": "max"})

    def context(key: str, **fields: object) -> harness.TaskContext:
        snapshot = ledger.load_snapshot()
        results = places["results"] / "TEST" / "run-1"
        results.mkdir(parents=True, exist_ok=True)
        work = places["work"] / "run-1"
        (work / key).mkdir(parents=True)
        return harness.TaskContext(
            experiment_id="TEST", run_id="run-1", arm=arm, model=MODEL, provider=None, snapshot=snapshot,
            endpoint=provider.Endpoint("openai", LOOPBACK), caps=caps.Caps.from_table(arm["caps"]),
            billed=False, instance_id="F1-l1-0001", seed=1, key=key,
            ledger=ledger.Ledger(results / "ledger.jsonl", line="BL1", experiment_id="TEST", run_id="run-1",
                                 price_snapshot_id=snapshot.id),
            workdir=work / key, spec_text="Implement clamp.", agent_env=agent_env.build(home=work / "_home" / key),
            **fields)

    invocation = run_codex.build_invocation(context("plain"), cli)
    assert invocation.jail == () and invocation.network is None  # no proxy given: unconfined, as run_cli's
    assert [path.name for path in invocation.config_dir.iterdir()] == ["auth.json"]
    assert invocation.config_dir_sha256 == run_codex.home_digest(invocation.config_dir)
    denied = run_codex.denied(context("denied"))
    for path in (layout.REPO_ROOT, Path.home() / ".roko", login):
        assert path in denied

    (login / "auth.json").write_text(json.dumps({"OPENAI_API_KEY": "sk-not-a-real-key-7c41", "tokens": None}))
    with pytest.raises(run_cli.CliError, match="no ChatGPT login"):
        run_codex.build_invocation(context("api-key"), cli)
    (login / "auth.json").unlink()
    with pytest.raises(run_cli.CliError, match="needs the subscription login"):
        run_codex.build_invocation(context("no-login"), cli)
    with pytest.raises(run_cli.CliError, match="flaky_verify"):
        run_codex.build_invocation(context("flaky", verify_wrapper=places["tmp"] / "vb-verify"), cli)
    missing = run_codex.CodexConfig(program=str(places["tmp"] / "no-such-codex"))
    with pytest.raises(run_cli.CliError, match="not an executable"):
        run_codex.build_invocation(context("missing"), missing)
    assert not log.exists()  # nothing above ever started codex

    # npm's codex is a node script: the session gets a link to that one interpreter, not the driver's PATH.
    script = places["tmp"] / "npm-bin" / "codex"
    script.parent.mkdir()
    script.write_text("#!/usr/bin/env fakenode\n")
    (script.parent / "fakenode").write_text("#!/bin/sh\n")
    (script.parent / "fakenode").chmod(0o755)
    env = agent_env.build(home=places["tmp"] / "npm-home")
    link = run_codex.interpreter_link(script, env)
    assert link == Path(env["HOME"]) / ".vb-bin" / "fakenode"
    assert link.resolve() == (script.parent / "fakenode").resolve()
    assert run_codex.interpreter_link(program, env) is None  # python3 is on the session's PATH already


def test_rotated_refresh_token_prints_a_login_warning(places, login, capsys):
    """gap-ba5006: a session that refreshes its Codex login rotates the refresh token inside its own sandboxed
    copy, never the operator's real ~/.codex (the canonical home is sandboxed out, so it never moves to compare).
    A changed fingerprint between seeding and settlement prints a warning naming the cause, never the tokens
    themselves; unchanged or unreadable prints nothing -- that is "unknown", not "proof it did not rotate"."""
    codex_home = places["tmp"] / "session-codex"
    run_codex._seed_codex_home(codex_home)
    seeded = run_codex.login_fingerprint(codex_home)
    assert seeded is not None

    run_codex._warn_if_login_rotated(seeded, run_codex.login_fingerprint(codex_home))
    assert capsys.readouterr().err == ""  # unchanged: silent

    rotated = {**LOGIN, "OPENAI_API_KEY": None, "tokens": {**LOGIN["tokens"], "refresh_token": "fake-refresh-2"}}
    (codex_home / "auth.json").write_text(json.dumps(rotated))
    run_codex._warn_if_login_rotated(seeded, run_codex.login_fingerprint(codex_home))
    err = capsys.readouterr().err
    assert "run_codex:" in err and "codex login" in err
    for token in ("fake-refresh", "fake-refresh-2", LOGIN["tokens"]["access_token"]):
        assert token not in err  # the warning names the cause, never a token

    (codex_home / "auth.json").unlink()
    run_codex._warn_if_login_rotated(seeded, run_codex.login_fingerprint(codex_home))
    assert capsys.readouterr().err == ""  # unreadable afterward: silent, not a false "it rotated"
