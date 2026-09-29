"""Offline tests of the Claude Code runner (`run_cli.py`, arm fd_claude): a fake `claude`, the toy family, no spend.

No test can reach a real `claude`: PATH holds only the fake's directory and the system directories, and the arm files
the runs use name the fake by its absolute path. Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -q
"""

from __future__ import annotations

import json
import os
import re
import time
from pathlib import Path

import pytest

import agent_env
import caps
import harness
import layout
import ledger
import provider
import records
import run_cli
import secret
import validate
import vb
from common import canary

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
MODEL = "claude-opus-5-5"
HAIKU = "claude-haiku-4-5-20251001"
LOOPBACK = "http://127.0.0.1:9/v1"
FAKE_KEY = "sk-ant-test-not-a-real-key-5b2e"
CORRECT = ('def clamp(value, low, high):\n    """Return value limited to the range [low, high]."""\n'
           "    return max(low, min(value, high))\n")
HIDDEN_URL = ("https://raw.githubusercontent.com/example/roko/main/benchmarks/viabilitybench/families/f1_pyconv/"
              "hidden.py")  # a truth suite, once the repository is public

# A `result` event in Claude Code 2.1.282's stream-json shape (the SDK's `modelUsage` fields, cumulative for the
# session), with background turns on the small model. The figures are made up; a live probe saves real ones.
RESULT = {
    "type": "result", "subtype": "success", "is_error": False, "duration_ms": 48211, "duration_api_ms": 45907,
    "num_turns": 7, "result": "Implemented clamp; the visible tests pass.", "session_id": "fake-session",
    "total_cost_usd": 0.2791,
    "usage": {"input_tokens": 38, "cache_creation_input_tokens": 21904, "cache_read_input_tokens": 186112,
              "output_tokens": 3120, "server_tool_use": {"web_search_requests": 0, "web_fetch_requests": 0},
              "service_tier": "standard",
              "cache_creation": {"ephemeral_1h_input_tokens": 21904, "ephemeral_5m_input_tokens": 0}},
    "modelUsage": {
        MODEL: {"inputTokens": 38, "outputTokens": 3120, "thinkingTokens": 1450, "cacheReadInputTokens": 186112,
                "cacheCreationInputTokens": 21904, "webSearchRequests": 0, "costUSD": 0.2756,
                "contextWindow": 200000, "maxOutputTokens": 64000, "canonicalModel": MODEL},
        HAIKU: {"inputTokens": 2513, "outputTokens": 196, "cacheReadInputTokens": 0, "cacheCreationInputTokens": 0,
                "webSearchRequests": 0, "costUSD": 0.0035, "contextWindow": 200000, "maxOutputTokens": 32000},
    },
    "permission_denials": [], "uuid": "fake-uuid",
}
# U′ at the snapshot's rates: claude-opus-5-5 4.00 in, 0.20 cache read, 8.00 1-hour cache write, 20.00 out;
# claude-haiku-4-5 1.00 in, 5.00 out.
U_PRIME = (38 * 4.00 + 186_112 * 0.20 + 21_904 * 8.00 + 3_120 * 20.00 + 2_513 * 1.00 + 196 * 5.00) / 1e6
R = 0.2791

FAKE_CLAUDE = r'''#!/usr/bin/env python3
"""A stand-in for `claude -p --output-format stream-json`: note what it was given, then play one scenario."""
import json
import os
import sys
import time

with open(__CONFIG__) as handle:
    CONFIG = json.load(handle)
prompt = sys.stdin.read()
names, file_hits = [], []
for base, dirs, files in os.walk("."):
    for name in dirs + files:
        path = os.path.normpath(os.path.join(base, name))
        names.append(path)
        if name in files and os.path.isfile(path) and not os.path.islink(path):
            with open(path, "rb") as handle:
                data = handle.read()
            file_hits += [path for needle in CONFIG["needles"] if needle.encode() in data]
config_dir = os.environ.get("CLAUDE_CONFIG_DIR", "")
seen = {"argv": sys.argv[1:], "cwd": os.getcwd(), "env": dict(os.environ), "prompt": prompt, "names": sorted(names),
        "file_hits": file_hits,
        "env_hits": [key for key, value in os.environ.items() for needle in CONFIG["needles"] if needle in value],
        "config_dir": sorted(os.listdir(config_dir)) if os.path.isdir(config_dir) else None}
with open(CONFIG["log"], "a") as handle:
    handle.write(json.dumps(seen) + "\n")


def emit(event):
    print(json.dumps(event), flush=True)


def assistant(number, model, block, input_tokens=1200, output_tokens=300):
    emit({"type": "assistant", "parent_tool_use_id": None, "session_id": "fake-session",
          "message": {"id": "msg_%d" % number, "model": model, "role": "assistant", "content": [block],
                      "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens,
                                "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}})


def fetch(parent):
    """A WebFetch of a truth suite: the tool returns a model-written digest, which carries no canary."""
    emit({"type": "assistant", "parent_tool_use_id": parent, "session_id": "fake-session",
          "message": {"id": "msg_fetch", "model": model, "role": "assistant", "content": [
              {"type": "tool_use", "id": "toolu_fetch", "name": "WebFetch",
               "input": {"url": CONFIG["hidden_url"], "prompt": "List the test cases."}}],
              "usage": {"input_tokens": 900, "output_tokens": 60, "cache_read_input_tokens": 0,
                        "cache_creation_input_tokens": 0}}})
    emit({"type": "user", "parent_tool_use_id": parent, "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "toolu_fetch", "content": "It checks clamp at both bounds."}]}})


scenario = CONFIG["scenario"]
if scenario == "crash":
    print("fake-claude: crashing before the first event", file=sys.stderr)
    sys.exit(3)
model = sys.argv[sys.argv.index("--model") + 1]
denied = []  # --disallowed-tools, which the "ignoring_flags" scenarios do not honour
for flag in ("--disallowed-tools", "--disallowedTools"):
    if flag in sys.argv and not scenario.endswith("ignoring_flags"):
        denied += sys.argv[sys.argv.index(flag) + 1].replace(",", " ").split()
tools = [tool for tool in ("Bash", "Edit", "Glob", "Grep", "Read", "WebFetch", "WebSearch", "Write")
         if tool not in denied]
emit({"type": "system", "subtype": "init", "cwd": os.getcwd(), "session_id": "fake-session", "model": model,
      "tools": tools, "mcp_servers": [], "plugins": [], "skills": [],
      "memory_paths": [], "permissionMode": "bypassPermissions", "apiKeySource": "none",
      "claude_code_version": "2.1.282"})
if scenario == "hang":
    time.sleep(120)
if scenario.startswith("fetch") and "WebFetch" in tools:  # a model that fetches the truth suite when it can
    time.sleep(5)  # the runner kills a session that offers a web tool before its first turn
    fetch(None)
if scenario == "subagent_fetch":  # a subagent's fetch, which the init event's tool list does not show
    fetch("toolu_task")
if scenario == "spend":
    for number in range(1, 1000):
        assistant(number, model, {"type": "text", "text": "Still reading."}, input_tokens=150000, output_tokens=5000)
        time.sleep(0.02)
served = "claude-sonnet-5" if scenario == "switch" else model
for path, text in CONFIG["files"].items():
    if os.path.isdir(os.path.dirname(path) or "."):
        with open(path, "w") as handle:
            handle.write(text)
assistant(1, served, {"type": "tool_use", "id": "toolu_1", "name": "Write", "input": {"file_path": "calc/ops.py"}})
emit({"type": "user", "parent_tool_use_id": None,
      "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "ok"}]}})
assistant(2, served, {"type": "text", "text": "Done."})
emit(CONFIG["result"])
'''


@pytest.fixture
def places(tmp_path: Path, monkeypatch) -> dict[str, Path]:
    secret_file = secret.create(tmp_path / "private-config" / "secret")  # a secret line and a canary line
    fake_bin = tmp_path / "fake-bin"
    fake_bin.mkdir()
    monkeypatch.setenv("PATH", os.pathsep.join([str(fake_bin), *agent_env.SYSTEM_PATH]))  # no real claude on it
    monkeypatch.delenv(secret.ENV_NAME, raising=False)  # `vb run` refuses a driver that holds VB_SECRET (preflight)
    for name, value in {"VB_SECRET_FILE": str(secret_file), "ANTHROPIC_API_KEY": FAKE_KEY,
                        "CLAUDE_CODE_OAUTH_TOKEN": FAKE_KEY}.items():
        monkeypatch.setenv(name, value)  # the driver may hold these; claude must never see them
    return {"tmp": tmp_path, "bin": fake_bin, "results": tmp_path / "results", "work": tmp_path / "work",
            "secret": secret_file}


def fake_claude(places: dict[str, Path], scenario: str) -> tuple[Path, Path]:
    """Put the fake `claude` for `scenario` on PATH; returns it and the log of what it was given."""
    log, config = places["tmp"] / f"claude-{scenario}.jsonl", places["tmp"] / f"claude-{scenario}.json"
    needles = [*secret.load(places["secret"]).needles, canary.RELEASE_CANARY, "vb.task/1"]
    config.write_text(json.dumps({"scenario": scenario, "log": str(log), "result": RESULT, "needles": needles,
                                  "files": {"calc/ops.py": CORRECT}, "hidden_url": HIDDEN_URL}))
    program = places["bin"] / "claude"
    program.write_text(FAKE_CLAUDE.replace("__CONFIG__", repr(str(config))))
    program.chmod(0o755)
    return program, log


def arm_file(places: dict[str, Path], program: Path, **overrides: object) -> str:
    """arms/fd_claude.toml with the fake as its program and some caps changed, written outside arms/."""
    text = (layout.ARMS_DIR / "fd_claude.toml").read_text()
    text = text.replace('program = "claude"', f'program = "{program}"', 1)
    assert f'program = "{program}"' in text
    for name, value in overrides.items():
        text, count = re.subn(rf"(?m)^{name} = \S+", f"{name} = {value}", text)
        assert count == 1, name
    path = places["tmp"] / "fd_claude.test.toml"
    path.write_text(text)
    return str(path)


def run_vb(places: dict[str, Path], arm: str, *extra: str, provider_url: str | None = LOOPBACK) -> int:
    url = ["--provider-url", provider_url] if provider_url else []
    return vb.main(["run", "--experiment", "TEST-CLI", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm", arm,
                    "--model", MODEL, "--seeds", "1", "--limit", "1", *url, "--results", str(places["results"]),
                    "--work", str(places["work"]), "--secret-file", str(places["secret"]), *extra])


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def run_dir(places: dict[str, Path]) -> Path:
    return places["results"] / "TEST-CLI" / "run-1"


def task_context(places: dict[str, Path], arm: dict, *, key: str = "F1-l1-0001.s1", line: str = "BL0",
                 endpoint: provider.Endpoint | None = None) -> harness.TaskContext:
    snapshot = ledger.load_snapshot()
    results = places["results"] / "TEST" / "run-1"
    results.mkdir(parents=True, exist_ok=True)
    work = places["work"] / "run-1"
    (work / key).mkdir(parents=True)
    return harness.TaskContext(
        experiment_id="TEST", run_id="run-1", arm=arm, model=MODEL, provider=None, snapshot=snapshot,
        endpoint=endpoint or provider.Endpoint("anthropic", "https://api.anthropic.com"),
        caps=caps.Caps.from_table(arm["caps"]), billed=False, instance_id="F1-l1-0001", seed=1, key=key,
        ledger=ledger.Ledger(results / "ledger.jsonl", line=line, experiment_id="TEST", run_id="run-1",
                             price_snapshot_id=snapshot.id),
        workdir=work / key, spec_text="Implement clamp.", agent_env=agent_env.build(home=work / "_home" / key))


def test_claude_arm_command_is_isolated_and_pinned(places, monkeypatch):
    program, _ = fake_claude(places, "solve")
    arm = vb.load_arm("fd_claude")
    assert arm["arm"]["models_allow"] == [MODEL] and arm["arm"]["runner"] == "run_cli"
    assert arm["arm"]["billed"] is False and vb.load_runner(arm) is run_cli
    cli = run_cli.CliConfig.from_table(arm["cli"])
    ctx = task_context(places, arm)
    invocation = run_cli.build_invocation(ctx, cli)

    argv = invocation.argv
    assert argv[0] == str(program)  # resolved on the driver's PATH, which holds only the fake
    flags = argv[1:]

    def value(flag: str) -> str:
        assert flags.count(flag) == 1, flag
        return flags[flags.index(flag) + 1]

    assert flags[:4] == ["--print", "--verbose", "--output-format", "stream-json"]
    assert value("--model") == MODEL and value("--effort") == "high" and value("--max-turns") == "50"
    assert value("--max-budget-usd") == "5" and value("--setting-sources") == ""
    assert value("--add-dir") == str(ctx.workdir)  # with the env below: the workdir's own CLAUDE.md, as in Roko's
    assert "--strict-mcp-config" in flags and json.loads(value("--mcp-config")) == {"mcpServers": {}}
    assert "--no-session-persistence" in flags and "--dangerously-skip-permissions" in flags
    for banned in ("--fallback-model", "--system-prompt", "--append-system-prompt", "--resume", "--continue",
                   "--bare", "--plugin-dir", "--agents", "--allowedTools", "--tools"):
        assert banned not in flags  # no Roko prompt, no fallback model, nothing the arm file did not pin
    task_settings = json.loads(value("--settings"))
    assert task_settings["disableAllHooks"] is True
    for path in (layout.REPO_ROOT, Path.home() / ".roko", ctx.ledger.path.parent):
        for tool in ("Read", "Edit"):
            assert f"{tool}(/{path.resolve()}/**)" in task_settings["permissions"]["deny"]

    env = invocation.env
    home = Path(env["HOME"])
    assert home == Path(ctx.agent_env["HOME"]) and home != Path.home()
    assert not layout.within(home, layout.REPO_ROOT) and not layout.within(home, ctx.workdir)
    assert not layout.within(ctx.workdir, home)
    assert env["CLAUDE_CONFIG_DIR"] == str(home / ".claude") == str(invocation.config_dir)
    assert list(invocation.config_dir.iterdir()) == [] and invocation.config_dir_sha256 == records.canonical_hash({})
    assert env["CLAUDE_CODE_DISABLE_AUTO_MEMORY"] == "1" and env["DISABLE_AUTOUPDATER"] == "1"
    assert env["CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD"] == "1"
    assert env["CLAUDE_SECURESTORAGE_CONFIG_DIR"] == ""  # keychain: the login's default entry name
    assert set(env) - set(ctx.agent_env) == {"CLAUDE_CONFIG_DIR", "CLAUDE_SECURESTORAGE_CONFIG_DIR",
                                             "CLAUDE_CODE_DISABLE_AUTO_MEMORY",
                                             "CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD", "DISABLE_AUTOUPDATER"}
    assert not [name for name in env if agent_env.FORBIDDEN_NAME.search(name)]  # no VB_*, key or token
    for needle in (FAKE_KEY, str(places["secret"]), *secret.load(places["secret"]).needles):
        assert needle not in "".join(env.values())
    assert env["PATH"].split(os.pathsep)[0] == str(home / ".vb-bin")  # the agent's PATH, not the driver's
    assert run_cli.ancestor_instructions(ctx.workdir) == []

    # An offline run points claude at the loopback URL, so a real claude could not reach the API either.
    offline = task_context(places, arm, key="offline", endpoint=provider.Endpoint("anthropic", LOOPBACK))
    assert run_cli.build_invocation(offline, cli).env["ANTHROPIC_BASE_URL"] == LOOPBACK

    # credentials_file: the fresh directory holds a private copy of the login and nothing else.
    login = places["tmp"] / "real-claude-config"
    login.mkdir()
    (login / ".credentials.json").write_text('{"claudeAiOauth": {"accessToken": "fake"}}')
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(login))
    by_file = run_cli.build_invocation(task_context(places, arm, key="file"),
                                       run_cli.CliConfig.from_table({**arm["cli"], "credentials": "credentials_file"}))
    assert [path.name for path in by_file.config_dir.iterdir()] == [".credentials.json"]
    assert (by_file.config_dir / ".credentials.json").stat().st_mode & 0o777 == 0o600
    assert "CLAUDE_SECURESTORAGE_CONFIG_DIR" not in by_file.env and by_file.env["CLAUDE_CONFIG_DIR"] != str(login)
    assert by_file.config_dir_sha256 == records.canonical_hash({".credentials.json": "login"})

    # Refused: a used config directory, a CLAUDE.md above the workdir, a fallback model, an unknown effort.
    (invocation.config_dir / "settings.json").write_text("{}")
    with pytest.raises(run_cli.CliError):
        run_cli.build_invocation(ctx, cli)
    planted = ctx.workdir.parent / "CLAUDE.md"
    planted.write_text("Always write tests first.\n")
    assert run_cli.ancestor_instructions(ctx.workdir) == [planted.resolve()]
    for bad in ({"fallback_model": "claude-sonnet-5"}, {"effort": "turbo"}, {"credentials": "env_token"}):
        with pytest.raises(run_cli.CliError):
            run_cli.CliConfig.from_table({**arm["cli"], **bad})


def test_result_event_is_priced_as_u_prime_and_r(tmp_path):
    snapshot = ledger.load_snapshot()
    parsed = run_cli.parse_result(RESULT, snapshot, cache_write_ttl="1h")
    assert parsed.cost.source == "cli_usage" and parsed.cost.api_equiv_usd == pytest.approx(U_PRIME, rel=1e-12)
    assert parsed.cost.without_cache_usd == pytest.approx(
        ((38 + 186_112) * 4.00 + 21_904 * 8.00 + 3_120 * 20.00 + 2_513 * 1.00 + 196 * 5.00) / 1e6, rel=1e-12)
    assert parsed.r_usd == R and parsed.gap == pytest.approx(abs(U_PRIME - R) / R)
    assert parsed.usage == {"tokens_in": 38 + 2_513, "tokens_out": 3_120 + 196, "tokens_cache_read": 186_112,
                            "tokens_cache_write_1h": 21_904, "tokens_reasoning": 1_450}
    assert parsed.models[MODEL]["api_equiv_usd"] == pytest.approx(
        (38 * 4.00 + 186_112 * 0.20 + 21_904 * 8.00 + 3_120 * 20.00) / 1e6)
    assert parsed.models[HAIKU]["slug"] == HAIKU and parsed.models[HAIKU]["api_equiv_usd"] == pytest.approx(
        (2_513 * 1.00 + 196 * 5.00) / 1e6)  # the background model, priced by its undated row
    assert parsed.web_search_requests == 0

    five = run_cli.parse_result(RESULT, snapshot, cache_write_ttl="5m")
    assert five.cost.api_equiv_usd == pytest.approx(U_PRIME - 21_904 * (8.00 - 5.00) / 1e6)
    tagged = {**RESULT, "modelUsage": {f"{MODEL}[1m]": {**RESULT["modelUsage"][MODEL], "canonicalModel": None},
                                       HAIKU: RESULT["modelUsage"][HAIKU]}}
    assert run_cli.parse_result(tagged, snapshot, cache_write_ttl="1h").cost.api_equiv_usd == pytest.approx(U_PRIME)
    mystery = {**RESULT, "modelUsage": {**RESULT["modelUsage"], "claude-mystery-9": RESULT["modelUsage"][HAIKU]}}
    unknown = run_cli.parse_result(mystery, snapshot, cache_write_ttl="1h")
    assert unknown.cost == ledger.Cost(None, None, "unknown") and unknown.r_usd == R and unknown.gap is None
    assert unknown.usage["tokens_in"] == 38 + 2 * 2_513  # tokens are still reported; only the price is unknown
    assert run_cli.parse_result({"type": "result"}, snapshot, cache_write_ttl="1h").cost.source == "unknown"

    # The subscription's ledger row: API-equivalent U′, $0 billed, source cli_usage.
    book = ledger.Ledger(tmp_path / "ledger.jsonl", line="BL0", experiment_id="T", run_id="r",
                         price_snapshot_id=snapshot.id)
    row = book.append(attempt_key="r/x:1", provider="anthropic", model_reported=MODEL, usage=parsed.usage,
                      cost=parsed.cost, billed=False, reserved_usd=5.0)
    assert validate.validate("ledger", row) == [] and row["billed_usd"] == 0.0 and row["source"] == "cli_usage"

    # The live meter keeps the latest usage per message id and prices cache writes by their TTL.
    meter = run_cli.Meter(snapshot, cache_write_ttl="1h")

    def block(number: int, output: int, model: str = MODEL) -> dict:
        return {"type": "assistant", "parent_tool_use_id": None,
                "message": {"id": f"msg_{number}", "model": model, "usage": {
                    "input_tokens": 10, "output_tokens": output, "cache_read_input_tokens": 1000,
                    "cache_creation_input_tokens": 500,
                    "cache_creation": {"ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 500}}}}

    for event in (block(1, 5), block(1, 40), block(2, 7), block(3, 0, model="<synthetic>")):
        meter.observe(event)
    assert meter.usage() == {"tokens_in": 20, "tokens_out": 47, "tokens_cache_read": 2000, "tokens_cache_write_5m": 0,
                             "tokens_cache_write_1h": 1000, "tokens_reasoning": 0}
    assert meter.spent_usd() == pytest.approx((20 * 4.00 + 47 * 20.00 + 2000 * 0.20 + 1000 * 8.00) / 1e6)
    assert meter.cost().source == "cli_usage"


def test_vb_run_with_a_fake_claude_labels_and_prices_the_run(places):
    program, log = fake_claude(places, "solve")
    assert run_vb(places, arm_file(places, program), "--transcripts") == 0
    out = run_dir(places)
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["arm"] == "fd_claude" and record["execution"]["status"] == "completed"
    assert record["vs"]["label"] == 1  # the fake wrote a correct clamp, and the census re-ran it
    [attempt] = record["execution"]["attempts"]
    assert attempt["model_reported"] == MODEL and attempt["provider"] == "anthropic" and attempt["turns"] == 7
    assert record["costs"] == {"api_equiv_usd": pytest.approx(U_PRIME), "billed_usd": 0.0,
                               "without_cache_usd": pytest.approx(0.985725), "vendor_usd": R,
                               "source": "cli_usage", "meter_cross_check_usd": None}
    cli = attempt["cli"]
    assert cli["cost_basis"] == "model_usage" and cli["u_prime_usd"] == pytest.approx(U_PRIME)
    assert cli["r_usd"] == R and cli["u_r_gap"] == pytest.approx(abs(U_PRIME - R) / R)
    assert cli["init"]["model"] == MODEL and cli["init"]["mcp_servers"] == []
    assert sorted(cli["models"]) == [HAIKU, MODEL]

    # The record holds the exact argv and environment claude got; --add-dir names its own working directory.
    [seen] = read_jsonl(log)
    assert cli["argv"] == [str(program), *seen["argv"]]
    assert Path(seen["argv"][seen["argv"].index("--add-dir") + 1]).resolve() == Path(seen["cwd"]).resolve()
    assert {name: seen["env"].get(name) for name in cli["env"] if not name.startswith("LC_")} == {
        name: value for name, value in cli["env"].items() if not name.startswith("LC_")}
    # What the agent could see: no manifest, canary or secret in its workdir; no VB_ variable, key or secret in its
    # environment; an empty config directory; the task's spec as its prompt.
    assert ".vb" not in seen["names"] and not [name for name in seen["names"] if name.endswith("task.json")]
    assert "calc/ops.py" in seen["names"] and seen["file_hits"] == [] and seen["env_hits"] == []
    assert not [name for name in seen["env"] if agent_env.FORBIDDEN_NAME.search(name)
                and name != "ANTHROPIC_BASE_URL"]
    assert seen["env"]["ANTHROPIC_BASE_URL"] == LOOPBACK and seen["config_dir"] == []
    assert "Toy instance" in seen["prompt"] and canary.RELEASE_CANARY not in seen["prompt"]
    key = f"{record['task']['instance_id']}.s1"
    assert json.loads((out / "private" / key / ".vb" / "task.json").read_text())["canary"] == canary.RELEASE_CANARY

    [row] = read_jsonl(out / "ledger.jsonl")
    assert validate.validate("ledger", row) == [] and row["source"] == "cli_usage" and row["billed_usd"] == 0.0
    assert row["api_equiv_usd"] == pytest.approx(U_PRIME) and row["model_reported"] == MODEL
    # The session was reserved before claude started ($0 on the subscription), and its row released it.
    [reserved] = read_jsonl(out / "reservations.jsonl")
    assert (reserved["event"], reserved["attempt_key"], reserved["reserved_usd"]) == ("reserve", row["attempt_key"], 0)
    assert ledger.read_books(places["results"]).reservations == []
    manifest = json.loads((out / "manifest.json").read_text())
    assert manifest["config"]["prompt"] == {"version": run_cli.PROMPT_VERSION, "sha256": run_cli.PROMPT_SHA256}
    assert manifest["config"]["arm"]["cli"]["effort"] == "high" and record["config_hash"] == manifest["config_hash"]
    transcript = json.loads((out / record["provenance"]["transcript_ref"]).read_text())
    assert transcript[1]["stream"]["subtype"] == "init" and transcript[-1]["stream"]["type"] == "result"
    assert not (places["work"] / "run-1").exists()  # the workdir and its config directory are gone


def test_vb_run_admits_the_claude_arm_only_with_both_network_flags(places, monkeypatch):
    program, log = fake_claude(places, "solve")
    arm = arm_file(places, program)
    assert run_vb(places, arm, provider_url=None) == 2
    assert run_vb(places, arm, "--allow-network", provider_url=None) == 2
    assert run_vb(places, arm, "--allow-network", "--max-cost-usd", "1", provider_url=None) == 2  # a task may cost $5
    assert not log.exists() and not places["results"].exists()
    monkeypatch.delenv("ANTHROPIC_API_KEY")  # claude signs in by itself; the driver holds no key for it
    assert run_vb(places, arm, "--allow-network", "--max-cost-usd", "20", provider_url=None) == 0
    [seen] = read_jsonl(log)
    assert "ANTHROPIC_BASE_URL" not in seen["env"]
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert record["execution"]["status"] == "completed"
    assert json.loads((run_dir(places) / "manifest.json").read_text())["offline"] is False


def test_live_spend_past_the_cap_kills_the_session(places):
    program, _ = fake_claude(places, "spend")
    assert run_vb(places, arm_file(places, program)) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "aborted_cap" and record["execution"]["reason"] == "usd"
    assert record["vs"]["label"] == 0
    per_message = (150_000 * 4.00 + 5_000 * 20.00) / 1e6
    spent = record["costs"]["api_equiv_usd"]
    assert 5.0 < spent < 5.0 + 4 * per_message and record["costs"]["source"] == "cli_usage"
    assert record["costs"]["vendor_usd"] is None  # killed before its result event: no R
    cli = record["execution"]["attempts"][0]["cli"]
    assert cli["cost_basis"] == "stream" and cli["killed"] == "usd"
    [row] = read_jsonl(run_dir(places) / "ledger.jsonl")
    assert validate.validate("ledger", row) == [] and row["api_equiv_usd"] == pytest.approx(spent)


def test_the_wallclock_limit_kills_a_hung_session(places):
    program, _ = fake_claude(places, "hang")
    started = time.monotonic()
    assert run_vb(places, arm_file(places, program, wallclock_s=2)) == 0
    assert time.monotonic() - started < 60
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "timeout" and record["execution"]["reason"] == "wallclock"
    assert record["costs"]["api_equiv_usd"] is None and record["costs"]["source"] == "unknown"  # nothing reported
    assert record["execution"]["attempts"][0]["cli"]["killed"] == "wallclock"
    [row] = read_jsonl(run_dir(places) / "ledger.jsonl")
    assert row["api_equiv_usd"] is None and row["source"] == "unknown"


@pytest.mark.parametrize("scenario", ["switch", "crash"])
def test_a_model_switch_or_a_crash_is_an_infra_error(places, scenario):
    program, _ = fake_claude(places, scenario)
    assert run_vb(places, arm_file(places, program)) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["execution"]["status"] == "infra_error"  # excluded from the report, and counted
    [attempt] = record["execution"]["attempts"]
    [row] = read_jsonl(run_dir(places) / "ledger.jsonl")
    if scenario == "switch":  # another model served the main thread
        assert attempt["model_reported"] == "claude-sonnet-5"
        assert attempt["cli"]["served_models"] == ["claude-sonnet-5"]
        assert record["costs"]["api_equiv_usd"] == pytest.approx(U_PRIME)
    else:  # no event at all: the cost is unknown, never $0
        assert "exited 3 without a result event" in record["execution"]["reason"]
        assert "crashing before the first event" in record["execution"]["reason"]
        assert record["costs"]["source"] == "unknown" and row["api_equiv_usd"] is None


@pytest.mark.parametrize("scenario", ["fetch", "fetch_ignoring_flags", "subagent_fetch"])
def test_fd_claude_cannot_fetch_the_hidden_suites(places, scenario):
    """gap-f253cf: the truth suites are in a public repository, and WebFetch returns a digest without the canary. The
    arm has no web tools; a claude that offers one anyway is killed before its first turn, and a web request that
    happens all the same (here a subagent's) makes the run leak_suspected."""
    program, log = fake_claude(places, scenario)
    arm = arm_file(places, program)
    assert run_vb(places, arm, "--transcripts") == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert validate.validate("run-record", record) == []
    [seen] = read_jsonl(log)
    flags = seen["argv"]
    assert flags.count("--disallowed-tools") == 1 and flags[flags.index("--disallowed-tools") + 1] == \
        "WebFetch,WebSearch"
    assert {"WebFetch", "WebSearch"} <= set(json.loads(flags[flags.index("--settings") + 1])["permissions"]["deny"])
    [attempt] = record["execution"]["attempts"]
    transcript = (run_dir(places) / record["provenance"]["transcript_ref"]).read_text()
    execution = (record["execution"]["status"], record["execution"]["reason"])
    if scenario == "fetch":  # the flags hold, so the model has only the repository to work from
        assert run_cli.web_tools(attempt["cli"]["init"]) == [] and "Bash" in attempt["cli"]["init"]["tools"]
        assert execution == ("completed", "ended") and record["vs"]["label"] == 1
        assert record["provenance"]["canary_places"] == [] and HIDDEN_URL not in transcript
    elif scenario == "fetch_ignoring_flags":  # web tools offered anyway: killed at the init event, before any fetch
        assert run_cli.web_tools(attempt["cli"]["init"]) == ["WebFetch", "WebSearch"]
        assert execution == ("infra_error", "web_tools") and attempt["cli"]["killed"] == "web_tools"
        assert HIDDEN_URL not in transcript and record["vs"]["label"] == 0
        out = places["tmp"] / "probe-out"  # the probe fails on the same claude
        assert run_cli.main(["probe", "--arm", arm, "--allow-network", "--work", str(places["work"]),
                             "--out", str(out)]) == 1
        [path] = out.glob("claude-probe-*.json")
        assert json.loads(path.read_text())["checks"]["no_web_tools"] is False
    else:  # the fetch reached the transcript without the canary; the census flags the request itself
        assert run_cli.web_tools(attempt["cli"]["init"]) == [] and HIDDEN_URL in transcript
        assert canary.RELEASE_CANARY not in transcript and "vb-canary-" not in transcript
        assert execution[0] == "leak_suspected" and record["provenance"]["canary_places"] == ["web"]
        assert record["provenance"]["canary_hits"] == 1


def test_a_budget_refusal_starts_no_session(places):
    program, log = fake_claude(places, "solve")
    ctx = task_context(places, vb.load_arm(arm_file(places, program)), line="BL99")  # the budget funds no BL99
    outcome = run_cli.run_task(ctx)
    assert (outcome.status, outcome.reason, outcome.attempts) == ("aborted_cap", "budget", [])
    assert "BL99" in outcome.transcript[-1]["reason"] and not log.exists()  # claude never started
    assert not ctx.ledger.path.exists() and not ctx.ledger.path.with_name(ledger.RESERVATIONS).exists()


def test_probe_saves_the_init_event(places):
    program, log = fake_claude(places, "solve")
    arm = arm_file(places, program)
    out = places["tmp"] / "probe-out"
    assert run_cli.main(["probe", "--arm", arm, "--work", str(places["work"]), "--out", str(out)]) == 2
    assert not log.exists()
    assert run_cli.main(["probe", "--arm", arm, "--allow-network", "--work", str(places["work"]),
                         "--out", str(out)]) == 0
    [path] = out.glob("claude-probe-*.json")
    report = json.loads(path.read_text())
    assert report["passed"] and all(report["checks"].values()) and report["status"] == "completed"
    assert report["init"]["model"] == MODEL and report["init"]["mcp_servers"] == []
    assert report["u_prime_usd"] == pytest.approx(U_PRIME) and report["r_usd"] == R
    [seen] = read_jsonl(log)
    assert report["argv"] == [str(program), *seen["argv"]] and seen["prompt"] == run_cli.PROBE_PROMPT
    task_flags = [*run_cli.FIXED_FLAGS, "--add-dir", "--model", "--effort", "--max-turns", "--max-budget-usd",
                  "--settings"]
    assert [arg for arg in report["argv"] if arg.startswith("--")] == [f for f in task_flags if f.startswith("--")]
    assert not list(places["work"].glob("claude-probe-*/_home"))  # the config directory is gone
