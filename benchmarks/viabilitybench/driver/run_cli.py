"""The Claude Code runner, harness `claude-code`: the runner of the fd_claude arm (S08 §4.9, §4.10; S09 §4.1).

Claude Code runs non-interactively (`claude -p`, stream-json) in the task's workdir, with the task as its only prompt
and no Roko or system prompt. The flags are `ClaudeCliAgent::build_command`'s (crates/roko-agent/src/
claude_cli_agent.rs) without Roko's system prompt, and never `--fallback-model`, which switches the model silently.
One session is the task's one attempt, and it ends when the agent stops (status `completed`).

**Isolation** (the flags and variables gap-8be530 settled against Claude Code 2.1.282). Started from the author's
machine, `claude -p` would also load the author's CLAUDE.md, memory, hooks, plugins and MCP servers, and the arm would
measure that setup rather than Claude Code (W10). Every session gets:
- a fresh config directory, `CLAUDE_CONFIG_DIR=$HOME/.claude` under the task's own HOME (`agent_env`), holding at most
  the login (`[cli] credentials`, below);
- `--setting-sources ""`: only `--settings` and managed policy apply, so no user, project or local settings, hooks,
  plugins, skills, agents or discovered CLAUDE.md load;
- `--add-dir <workdir>` with `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`: the workdir's own CLAUDE.md,
  `.claude/CLAUDE.md` and `.claude/rules` load, and nothing above it. Roko's runs use the same pair, so both arms see
  the task repository's instructions and no one else's (full isolation would drop it). The workdir is the working
  directory, so the flag grants no file access. The runner also refuses a workdir with a CLAUDE.md above it;
- `--strict-mcp-config` with an empty `--mcp-config`: no MCP server;
- `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`, since auto-memory ignores setting sources, and `DISABLE_AUTOUPDATER=1`, so the
  version cannot change mid-run (each record keeps the version its `init` event reports);
- `--settings` with no hooks, and deny rules for the repository, `~/.roko`, the run's results and the default secret
  directory (S08 §4.9, "where the CLI supports them": they bind Claude's file tools, not its shell, and canaries
  catch the rest);
- no web tools (gap-f253cf): `--disallowed-tools WebFetch,WebSearch` takes them out of the model's context, and
  deny rules for both in `--settings` also bind subagents. The repository is public, so a web fetch could return a
  truth suite, and WebFetch hands back a model-written digest in which the canary may never appear. The tasks need
  no web (S08 §4.2 (3): every requirement is in the spec or the repo), so this is the arm's one departure from
  Claude Code's default tools. A session whose `init` event still offers a web tool (any tool named `Web…`) is
  killed before its first turn and ends `infra_error` (reason `web_tools`). Any web request that reaches the
  transcript anyway makes the census mark the run `leak_suspected` (`census`, place `web`);
- `--no-session-persistence`, and `--dangerously-skip-permissions`, since the agent may edit and run anything in its
  workdir, as in the direct loop.
Its environment is the task's agent environment plus those variables, and `ANTHROPIC_BASE_URL` for a loopback
`--provider-url`, so an offline run cannot reach the API. No `VB_*` variable and no provider key reach it.

**Credentials** (`[cli] credentials`). With `CLAUDE_CONFIG_DIR` set, Claude Code looks for its macOS keychain entry
under a name suffixed with a hash of that directory, and misses the subscription login. `keychain`, the default, sets
`CLAUDE_SECURESTORAGE_CONFIG_DIR=` (empty) to keep the default entry name; the variable is undocumented, so the probe
confirms it. `credentials_file` copies the login's `.credentials.json` (Linux, or a file-based login) into the fresh
directory instead. A token in the environment (`CLAUDE_CODE_OAUTH_TOKEN`) is not offered: Claude Code hands its
environment to the agent's shell.

**Caps** (S08 §4.10, subscription arms: native behaviour with safety limits). `[caps] turns_per_task` goes to
`--max-turns` and `usd_per_task` to `--max-budget-usd`, where Claude Code stops itself and still reports its usage.
The runner also kills the session once the live API-equivalent spend of its streamed messages passes `usd_per_task`,
and at `wallclock_s`. The direct loop's other caps do not apply. A kill ends Claude Code's process group; a command it
started in a session of its own can outlive it. Before claude starts, the attempt is reserved on the run's budget
line (`Ledger.reserve`, $0 on the subscription), and its ledger row releases the reservation. If the line refuses,
the task ends `aborted_cap` (reason `budget`) and claude never starts.

**Cost** (S09 §4.1), from the last `result` event, whose figures are cumulative:
- U′, the headline (`api_equiv_usd`, source `cli_usage`): Σ over `modelUsage`'s models of their tokens × the snapshot
  row of the model (its `canonicalModel` when given), with cache writes at `[cli] cache_write_ttl` (1-hour for
  subscription turns). Background turns on a small model (claude-haiku-4-5) count; a model without a row makes U′
  unknown;
- R, `total_cost_usd`, the CLI's own figure, kept as `vendor_usd`. The attempt records both and |U′ − R|/R.
A session killed before its `result` event is priced from the usage its assistant messages carried
(`cli.cost_basis = "stream"`), a partial total that misses background calls, so its source is `estimated` in the
ledger row and the run record alike (bug-f62293, bug-a49003).

**Model.** `model_reported` is the model that served the main thread's messages (the `init` event's when none did). If
any other model served it, the attempt reports that model, and `records.final_status` makes the run an `infra_error`
that the report excludes. Subagents' and background models are recorded and allowed.

Each attempt record carries `cli`: the exact argv and environment, the config directory's digest at launch, the `init`
event, the usage per model, U′, R and their gap. `PROMPT_SHA256` covers the task message and all that is fixed about
the invocation (flags, settings, environment, the config directory's seed), so `vb run`'s `config_hash`, which holds
it and the arm file, covers every flag.

**Probe** (gap-c4f364, step 2): `run_cli.py probe --arm fd_claude --allow-network` runs one throwaway session with the
arm's invocation and a trivial prompt, and saves its argv, environment, `init` and `result` events as JSON. It passes
when the `init` event shows the pinned model and no MCP server, plugin, memory or web tool, and the session signed in.

API:
    run_task(ctx: harness.TaskContext) -> harness.TaskOutcome
    CliConfig.from_table(table) -> CliConfig                    # raises CliError
    build_invocation(ctx, cli) -> Invocation                    # raises CliError
    run_session(invocation, prompt, *, cwd, wallclock_s, usd_cap, meter) -> Session
    parse_result(event, snapshot, *, cache_write_ttl) -> ResultCost
    Meter(snapshot, *, cache_write_ttl); CliAttempt; settings(run_dir) -> dict; web_tools(event) -> list[str]
    ancestor_instructions(workdir) -> list[Path]; config_dir_digest(path) -> str
    main(argv) -> int                                           # `probe`
    PROMPT_VERSION, PROMPT_SHA256
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import queue
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field, fields
from pathlib import Path
from typing import IO

import agent_env
import archive
import caps
import harness
import layout
import ledger
import records
from common import repo

PROMPT_VERSION = "claude-code-1"
EFFORTS = ("low", "medium", "high", "xhigh", "max")
CREDENTIALS = ("keychain", "credentials_file")
CACHE_WRITE_TTLS = ("5m", "1h")
CREDENTIAL_FILE = ".credentials.json"
WEB_TOOLS = ("WebFetch", "WebSearch")  # disallowed, and denied in the settings (gap-f253cf)
WEB_TOOL_PREFIX = "Web"  # a tool the init event offers under this prefix kills the session
FIXED_FLAGS = ("--print", "--verbose", "--output-format", "stream-json", "--setting-sources", "",
               "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}', "--disallowed-tools", ",".join(WEB_TOOLS),
               "--no-session-persistence", "--dangerously-skip-permissions")
WORKDIR_FLAG = "--add-dir"  # followed by the workdir, whose own CLAUDE.md files then load
FIXED_ENV = {"CLAUDE_CODE_DISABLE_AUTO_MEMORY": "1", "CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD": "1",
             "DISABLE_AUTOUPDATER": "1"}
CLI_ENV = ("CLAUDE_CONFIG_DIR", "CLAUDE_SECURESTORAGE_CONFIG_DIR", "ANTHROPIC_BASE_URL", *FIXED_ENV)
SETTINGS = {"disableAllHooks": True}  # plus deny rules, which name this host's paths
DENY_TOOLS = ("Read", "Edit")
INSTRUCTION_FILES = ("CLAUDE.md", "CLAUDE.local.md")
CAPPED = {"error_max_turns": "max_turns", "error_max_budget_usd": "usd"}  # result subtypes of Claude Code's own caps
SYNTHETIC = "<synthetic>"  # the model of messages Claude Code makes up itself (API errors); they cost nothing
CONTEXT_TAG = re.compile(r"\[[^\]]*\]$")  # "claude-opus-5-5[1m]" names a context window, not another model
EXIT_GRACE_S = 2.0
STDERR_CHARS = 4000
PROBE_WALLCLOCK_S = 300.0

TASK_MESSAGE = """\
<task>
{spec}
</task>

The repository is your current working directory. Complete the task, then stop.
"""

PROBE_PROMPT = "Reply with the single word READY. Do not use any tools."

PROMPT_SHA256 = hashlib.sha256(json.dumps([TASK_MESSAGE, FIXED_FLAGS, WORKDIR_FLAG, FIXED_ENV, SETTINGS, DENY_TOOLS,
                                           WEB_TOOLS, CREDENTIAL_FILE], sort_keys=True).encode()).hexdigest()


class CliError(RuntimeError):
    """The arm's [cli] table is invalid, or a session cannot be set up; nothing was started."""


@dataclass(frozen=True)
class CliConfig:
    program: str = "claude"
    effort: str = "high"
    credentials: str = "keychain"
    cache_write_ttl: str = "1h"

    @classmethod
    def from_table(cls, table: Mapping) -> CliConfig:
        if "fallback_model" in table:
            raise CliError("[cli] fallback_model: a fallback model switches the model silently, so the arm has none")
        unknown = sorted(set(table) - {item.name for item in fields(cls)})
        if unknown:
            raise CliError(f"unknown [cli] key(s): {', '.join(unknown)}")
        config = cls(**table)
        if not isinstance(config.program, str) or not config.program:
            raise CliError("[cli] program must name the claude executable")
        for name, allowed in (("effort", EFFORTS), ("credentials", CREDENTIALS),
                              ("cache_write_ttl", CACHE_WRITE_TTLS)):
            if getattr(config, name) not in allowed:
                raise CliError(f"[cli] {name} must be one of {', '.join(allowed)}, not {getattr(config, name)!r}")
        return config


@dataclass(frozen=True)
class Invocation:
    argv: list[str]
    env: dict[str, str]
    config_dir: Path
    config_dir_sha256: str  # at launch


@dataclass(frozen=True)
class ResultCost:
    """U′ and R of one `result` event (S09 §4.1)."""

    models: dict[str, dict]  # modelUsage key -> {"slug", "usage", "api_equiv_usd"}
    usage: dict | None  # summed over the models; None when an entry had no token counts
    cost: ledger.Cost  # U′, source cli_usage; unknown when a model has no row or an entry no counts
    r_usd: float | None  # total_cost_usd
    web_search_requests: int

    @property
    def gap(self) -> float | None:
        """|U′ − R| / R."""
        u_prime = self.cost.api_equiv_usd
        return abs(u_prime - self.r_usd) / self.r_usd if u_prime is not None and self.r_usd else None


class Meter:
    """Live API-equivalent spend of one session, from the usage its assistant messages carry (S08 §4.10's kill).

    Claude Code emits one assistant event per content block, each repeating its message's usage so far, so the latest
    usage per message id counts. Background calls never reach the stream, so the total is a lower bound.
    """

    def __init__(self, snapshot: ledger.Snapshot, *, cache_write_ttl: str) -> None:
        self.snapshot = snapshot
        self.cache_write_ttl = cache_write_ttl
        self.messages: dict[str, tuple[str | None, dict | None]] = {}  # message id -> (model, usage)

    def observe(self, event: Mapping) -> None:
        message = event.get("message")
        if event.get("type") != "assistant" or not isinstance(message, dict) or message.get("model") == SYNTHETIC:
            return
        key = message.get("id") if isinstance(message.get("id"), str) else f"unnamed-{len(self.messages)}"
        model = message.get("model") if isinstance(message.get("model"), str) else None
        self.messages[key] = (model, _message_usage(message.get("usage"), self.cache_write_ttl))

    def spent_usd(self) -> float:
        """The priced part of the spend so far; a message on a model without a row adds nothing."""
        return sum(cost.api_equiv_usd for cost in self._costs() if cost.api_equiv_usd is not None)

    def cost(self) -> ledger.Cost:
        """The streamed messages' cost, `estimated`: a lower bound, not the CLI's own report (bug-a49003)."""
        costs = self._costs()
        if not costs or any(cost.source == "unknown" for cost in costs):
            return ledger.Cost(None, None, "unknown")
        return ledger.Cost(sum(cost.api_equiv_usd for cost in costs), sum(cost.without_cache_usd for cost in costs),
                           "estimated")

    def usage(self) -> dict | None:
        usages = [usage for _, usage in self.messages.values()]
        if not usages or None in usages:
            return None
        total: dict = {}
        for usage in usages:
            total = ledger.add_usage(total, usage)
        return total

    def _costs(self) -> list[ledger.Cost]:
        return [ledger.price(usage, self.snapshot.row(_slug(model))) for model, usage in self.messages.values()]


@dataclass
class CliAttempt(harness.Attempt):
    """An attempt of this runner: `vendor_usd` is R, and `cli` the invocation and what the CLI reported."""

    vendor_usd: float | None = None
    cli: dict = field(default_factory=dict)

    def as_record(self) -> dict:
        return {**super().as_record(), "vendor_usd": self.vendor_usd, "cli": self.cli}


@dataclass
class Session:
    started: bool = False
    events: list[dict] = field(default_factory=list)  # stream-json events in order; other lines as {"type": "raw"}
    stop: caps.Stop | None = None  # the runner's own kill
    exit_code: int | None = None
    stderr: str = ""
    error: str | None = None  # why claude could not start

    @property
    def init(self) -> dict | None:
        return next((event for event in self.events
                     if event.get("type") == "system" and event.get("subtype") == "init"), None)

    @property
    def result(self) -> dict | None:
        """The last result event: its figures are cumulative."""
        return next((event for event in reversed(self.events) if event.get("type") == "result"), None)


def run_task(ctx: harness.TaskContext) -> harness.TaskOutcome:
    started = harness.utc_now()
    prompt = TASK_MESSAGE.format(spec=ctx.spec_text.strip())
    transcript: list[dict] = [{"attempt": 1, "role": "user", "content": prompt}]
    try:
        cli = CliConfig.from_table(ctx.arm.get("cli", {}))
        above = ancestor_instructions(ctx.workdir)
        if above:
            raise CliError(f"Claude Code would read {above[0]}, above the workdir")
        invocation = build_invocation(ctx, cli)
    except (CliError, OSError, agent_env.AgentEnvError) as err:
        return harness.TaskOutcome("infra_error", f"claude setup: {err}", [], transcript, started, harness.utc_now())
    attempt = CliAttempt(number=1, attempt_key=f"{ctx.chain_key}:1", model_requested=ctx.model,
                         provider=ctx.endpoint.provider,
                         reserved_usd=caps.worst_task_usd(ctx.caps, ctx.price_row) or ctx.caps.usd_per_task)
    try:  # a subscription bills nothing, so it reserves $0 against its budget line
        ctx.ledger.reserve(attempt.attempt_key, attempt.reserved_usd if ctx.billed else 0.0)
    except ledger.BudgetError as err:  # the line has no room for the session, which never starts
        transcript.append({"attempt": 1, "event": "stop", "kind": "aborted_cap", "reason": str(err)})
        return harness.TaskOutcome("aborted_cap", "budget", [], transcript, started, harness.utc_now())
    meter = Meter(ctx.snapshot, cache_write_ttl=cli.cache_write_ttl)
    session = run_session(invocation, prompt, cwd=ctx.workdir, wallclock_s=ctx.caps.wallclock_s,
                          usd_cap=ctx.caps.usd_per_task, meter=meter)
    status, reason = _status(session)
    transcript += [{"attempt": 1, "stream": event} for event in session.events]
    if session.stop:
        transcript.append({"attempt": 1, "event": "stop", "kind": session.stop.kind, "reason": session.stop.reason})
    if session.stderr:
        transcript.append({"attempt": 1, "stderr": session.stderr})
    if not session.started:  # no process, so no model call: nothing to book
        ctx.ledger.release(attempt.attempt_key)
        return harness.TaskOutcome(status, reason, [], transcript, started, harness.utc_now())
    attempt.ended_by = reason if status != "infra_error" else status
    _settle(ctx, attempt, session, meter, invocation, cli)  # its ledger row releases the reservation
    return harness.TaskOutcome(status=status, reason=reason, attempts=[attempt], transcript=transcript,
                               started_at=started, finished_at=harness.utc_now())


def build_invocation(ctx: harness.TaskContext, cli: CliConfig) -> Invocation:
    """The argv and environment of one session, with its fresh config directory (made here)."""
    program = shutil.which(cli.program)
    if program is None:
        raise CliError(f"{cli.program} is not an executable on the driver's PATH")
    config_dir = Path(ctx.agent_env["HOME"]) / ".claude"
    _seed_config_dir(config_dir, cli.credentials)
    argv = [str(Path(program).absolute()), *FIXED_FLAGS, WORKDIR_FLAG, str(ctx.workdir), "--model", ctx.model,
            "--effort", cli.effort, "--max-turns", str(ctx.caps.turns_per_task),
            "--max-budget-usd", f"{ctx.caps.usd_per_task:g}",
            "--settings", json.dumps(settings(ctx.ledger.path.parent), sort_keys=True, separators=(",", ":"))]
    env = {**ctx.agent_env, **FIXED_ENV, "CLAUDE_CONFIG_DIR": str(config_dir)}
    if cli.credentials == "keychain":
        env["CLAUDE_SECURESTORAGE_CONFIG_DIR"] = ""
    if ctx.endpoint.offline:  # a loopback --provider-url: a real claude must not reach the API either
        env["ANTHROPIC_BASE_URL"] = ctx.endpoint.base_url
    agent_env.check({name: value for name, value in env.items() if name not in CLI_ENV})
    return Invocation(argv=argv, env=env, config_dir=config_dir, config_dir_sha256=config_dir_digest(config_dir))


def settings(run_dir: Path) -> dict:
    """`--settings`: no hooks, deny rules for the web tools, and deny rules (absolute `//path` patterns) for what the
    agent must not read or edit."""
    home = Path.home()
    denied = (layout.REPO_ROOT, home / ".roko", Path(run_dir), home / ".config" / "viabilitybench")
    return {**SETTINGS, "permissions": {"deny": [*WEB_TOOLS, *(f"{tool}(/{Path(path).resolve()}/**)"
                                                               for path in denied for tool in DENY_TOOLS)]}}


def web_tools(event: Mapping) -> list[str]:
    """The web tools an `init` event offers (tools named `Web…`); [] for any other event."""
    if event.get("type") != "system" or event.get("subtype") != "init" or not isinstance(event.get("tools"), list):
        return []
    return [tool for tool in event["tools"] if isinstance(tool, str) and tool.startswith(WEB_TOOL_PREFIX)]


def ancestor_instructions(workdir: Path) -> list[Path]:
    """The CLAUDE.md files in the directories above `workdir`."""
    return [parent / name for parent in Path(workdir).resolve().parents for name in INSTRUCTION_FILES
            if (parent / name).is_file()]


def config_dir_digest(path: Path) -> str:
    """sha256 over a config directory's paths and file contents; the login counts by name only, since it rotates."""
    entries = {}
    for item in sorted(Path(path).rglob("*")):
        name = item.relative_to(path).as_posix()
        if item.is_symlink():
            entries[name] = "link:" + os.readlink(item)
        elif item.is_dir():
            entries[name] = "dir"
        else:
            entries[name] = "login" if item.name == CREDENTIAL_FILE else hashlib.sha256(item.read_bytes()).hexdigest()
    return records.canonical_hash(entries)


def run_session(invocation: Invocation, prompt: str, *, cwd: Path, wallclock_s: float, usd_cap: float,
                meter: Meter) -> Session:
    """Run claude once in a session of its own, with `prompt` on stdin; kill it at the wall clock or the dollar cap."""
    session = Session()
    try:
        process = subprocess.Popen(invocation.argv, cwd=cwd, env=invocation.env, stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    except OSError as err:
        session.error = f"could not start {invocation.argv[0]}: {err}"
        return session
    session.started = True
    lines: queue.Queue[bytes | None] = queue.Queue()
    tail: list[bytes] = []
    threads = [threading.Thread(target=_feed, args=(process.stdin, prompt.encode("utf-8")), daemon=True),
               threading.Thread(target=_pump, args=(process.stdout, lines.put), daemon=True),
               threading.Thread(target=_drain, args=(process.stderr, tail), daemon=True)]
    for thread in threads:
        thread.start()
    deadline = time.monotonic() + wallclock_s
    ended = False
    exited_at: float | None = None
    try:
        while session.stop is None:
            left = deadline - time.monotonic()
            if left <= 0:
                session.stop = caps.Stop("timeout", "wallclock")
                break
            try:
                raw = lines.get(timeout=min(left, 0.5))
            except queue.Empty:
                if process.poll() is not None:  # it exited, but a leftover child may still hold its stdout
                    exited_at = exited_at or time.monotonic()
                    if time.monotonic() - exited_at > EXIT_GRACE_S:
                        break
                continue
            if raw is None:
                break
            event = _take(session, meter, raw)
            if event is not None and web_tools(event):  # the init event offers a web tool: stop before any turn
                session.stop = caps.Stop("infra_error", "web_tools")
            elif meter.spent_usd() > usd_cap:
                session.stop = caps.Stop("aborted_cap", "usd")
        ended = True
    finally:
        if session.stop is not None or not ended:
            _kill_session(process.pid)
        try:
            session.exit_code = process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            _kill_session(process.pid)
            session.exit_code = process.wait()
        _kill_session(process.pid)  # background jobs its commands left behind
        for thread in threads:
            thread.join(timeout=5)
    while True:  # lines that arrived before the end
        try:
            raw = lines.get_nowait()
        except queue.Empty:
            break
        if raw is not None:
            _take(session, meter, raw)
    session.stderr = b"".join(tail).decode("utf-8", "replace")[-STDERR_CHARS:]
    return session


def parse_result(event: Mapping, snapshot: ledger.Snapshot, *, cache_write_ttl: str) -> ResultCost:
    """U′ and R from a `result` event (the module docstring has the rules)."""
    entries = event.get("modelUsage") if isinstance(event.get("modelUsage"), dict) else {}
    models: dict[str, dict] = {}
    costs: list[ledger.Cost] = []
    usage: dict | None = {}
    searches = 0
    for key, entry in entries.items():
        entry = entry if isinstance(entry, dict) else {}
        canonical = entry.get("canonicalModel")
        slug = _slug(canonical if isinstance(canonical, str) and canonical else key)
        counts = _model_usage(entry, cache_write_ttl)
        cost = ledger.price(counts, snapshot.row(slug))
        models[key] = {"slug": slug, "usage": counts, "api_equiv_usd": cost.api_equiv_usd}
        costs.append(cost)
        usage = None if usage is None or counts is None else ledger.add_usage(usage, counts)
        searches += _count(entry.get("webSearchRequests")) or 0
    if costs and all(cost.source != "unknown" for cost in costs):
        total = ledger.Cost(sum(cost.api_equiv_usd for cost in costs),
                            sum(cost.without_cache_usd for cost in costs), "cli_usage")
    else:
        total = ledger.Cost(None, None, "unknown")
    reported = event.get("total_cost_usd")
    r_usd = float(reported) if isinstance(reported, (int, float)) and not isinstance(reported, bool) \
        and reported >= 0 else None
    return ResultCost(models=models, usage=usage or None, cost=total, r_usd=r_usd, web_search_requests=searches)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="run_cli.py", description="The fd_claude arm's isolation probe.",
                                     allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)
    probe = commands.add_parser("probe", help="run one throwaway session and save its init event",
                                allow_abbrev=False)
    probe.add_argument("--arm", default="fd_claude", help="an arm name in arms/, or a path to one")
    probe.add_argument("--allow-network", action="store_true",
                       help="required: the probe runs claude for real, on the subscription")
    probe.add_argument("--work", type=Path, help="default: $VB_WORK, then ~/vb-work")
    probe.add_argument("--out", type=Path, help="where the probe's JSON goes (default: its directory under --work)")
    args = parser.parse_args(argv)
    import vb  # here rather than at the top: `vb run` imports this module as a runner

    try:
        return _probe(args, vb)
    except (CliError, vb.DriverError, caps.CapError, ledger.PriceError, agent_env.AgentEnvError, OSError) as err:
        print(f"run_cli: {err}", file=sys.stderr)
        return 2


def _probe(args: argparse.Namespace, vb) -> int:
    if not args.allow_network:
        raise CliError("the probe runs claude for real, on the subscription; pass --allow-network")
    arm = vb.load_arm(args.arm)
    model = arm["arm"]["models_allow"][0]
    cli = CliConfig.from_table(arm.get("cli", {}))
    snapshot = ledger.load_snapshot()
    stamp = dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ")
    work_root = vb._outside_repo(args.work or os.environ.get("VB_WORK") or vb.DEFAULT_WORK, "--work")
    root = work_root / f"claude-probe-{stamp}"
    workdir = root / "work"
    workdir.mkdir(mode=0o700, parents=True)
    ctx = harness.TaskContext(
        experiment_id="PROBE", run_id=root.name, arm=arm, model=model, provider=None, snapshot=snapshot,
        endpoint=vb._endpoint(arm, snapshot.row(model), None), caps=caps.Caps.from_table(arm.get("caps", {})),
        ledger=ledger.Ledger(root / "ledger.jsonl", line="probe", experiment_id="PROBE", run_id=root.name,
                             price_snapshot_id=snapshot.id),
        billed=False, instance_id="probe", seed=0, key="probe", workdir=workdir, spec_text=PROBE_PROMPT,
        agent_env=agent_env.build(home=root / "_home"))
    try:
        invocation = build_invocation(ctx, cli)
        session = run_session(invocation, PROBE_PROMPT, cwd=workdir,
                              wallclock_s=min(ctx.caps.wallclock_s, PROBE_WALLCLOCK_S), usd_cap=ctx.caps.usd_per_task,
                              meter=Meter(snapshot, cache_write_ttl=cli.cache_write_ttl))
    finally:
        archive.remove_tree(root / "_home")  # the config directory, with any copied login
    status, reason = _status(session)
    init = session.init or {}
    tools = init.get("tools") if isinstance(init.get("tools"), list) else None
    parsed = parse_result(session.result, snapshot, cache_write_ttl=cli.cache_write_ttl) if session.result else None
    checks = {
        "model_pinned": isinstance(init.get("model"), str) and records.same_model(model, _slug(init["model"]))
                        and all(records.same_model(model, served) for served in _served_models(session)),
        "no_mcp_servers": init.get("mcp_servers") == [],
        "no_mcp_tools": tools is not None and not any(str(tool).startswith("mcp__") for tool in tools),
        "no_web_tools": tools is not None and not web_tools(init),
        "no_plugins": not init.get("plugins"),
        "no_memory": not init.get("memory_paths"),
        "signed_in": status == "completed",
    }
    report = {"schema_version": "vb.claude_probe/1", "at": harness.utc_now(), "arm": arm["arm"]["id"], "model": model,
              "credentials": cli.credentials, "passed": all(checks.values()), "checks": checks, "status": status,
              "reason": reason, "argv": invocation.argv, "env": invocation.env,
              "config_dir_sha256": invocation.config_dir_sha256, "init": session.init, "result": session.result,
              "u_prime_usd": parsed.cost.api_equiv_usd if parsed else None, "r_usd": parsed.r_usd if parsed else None,
              "stderr": session.stderr}
    out = Path(args.out).absolute() if args.out else root
    out.mkdir(mode=0o700, parents=True, exist_ok=True)
    path = out / f"claude-probe-{stamp}.json"
    path.write_text(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    failed = [name for name, ok in checks.items() if not ok]
    print(f"run_cli: probe {'passed' if not failed else 'failed: ' + ', '.join(failed)} ({reason}); {path}",
          file=sys.stderr)
    return 0 if not failed else 1


def _settle(ctx: harness.TaskContext, attempt: CliAttempt, session: Session, meter: Meter, invocation: Invocation,
            cli: CliConfig) -> None:
    """Price the attempt from what the CLI reported, fill in its record, and append its ledger row."""
    result = session.result
    parsed = parse_result(result, ctx.snapshot, cache_write_ttl=cli.cache_write_ttl) if result else None
    if parsed is not None:
        usage, cost, basis = parsed.usage, parsed.cost, "model_usage"
    else:
        usage, cost, basis = meter.usage(), meter.cost(), "stream" if meter.messages else "none"
    messages = [event for event in session.events if event.get("type") == "assistant"
                and isinstance(event.get("message"), dict) and event["message"].get("model") != SYNTHETIC]
    main_ids = {str(event["message"].get("id")) for event in messages if event.get("parent_tool_use_id") is None}
    served = _served_models(session)
    attempt.model_reported = next((model for model in served if not records.same_model(ctx.model, model)),
                                  served[0] if served else None)
    turns = _count(result.get("num_turns")) if result else None
    attempt.turns = len(main_ids) if turns is None else turns
    attempt.calls = len({str(event["message"].get("id")) for event in messages})
    attempt.usage_unknown = usage is None
    attempt.usage = usage or attempt.usage
    attempt.cost = cost
    attempt.vendor_usd = parsed.r_usd if parsed else None
    try:
        attempt.tree = repo.tree_hash(ctx.workdir)
    except (OSError, repo.RepoError):
        attempt.tree = None
    attempt.cli = {
        "argv": invocation.argv, "env": invocation.env, "config_dir_sha256": invocation.config_dir_sha256,
        "credentials": cli.credentials, "cache_write_ttl": cli.cache_write_ttl, "init": session.init,
        "result": {key: result.get(key) for key in ("subtype", "is_error", "num_turns", "duration_ms",
                                                    "duration_api_ms", "session_id")} if result else None,
        "served_models": served, "models": parsed.models if parsed else None, "cost_basis": basis,
        "u_prime_usd": parsed.cost.api_equiv_usd if parsed else None, "r_usd": attempt.vendor_usd,
        "u_r_gap": parsed.gap if parsed else None,
        "web_search_requests": parsed.web_search_requests if parsed else None,
        "exit_code": session.exit_code, "killed": session.stop.reason if session.stop else None}
    ctx.ledger.append(attempt_key=attempt.attempt_key, provider=attempt.provider, model_reported=attempt.model_reported,
                      usage=attempt.reported_usage(), cost=cost, billed=ctx.billed, reserved_usd=attempt.reserved_usd)


def _status(session: Session) -> tuple[str, str]:
    if session.error:
        return "infra_error", session.error
    if session.stop:
        return session.stop.kind, session.stop.reason
    result = session.result
    if result is None:
        detail = session.stderr.strip()[-300:]
        return "infra_error", f"claude exited {session.exit_code} without a result event" + (
            f": {detail}" if detail else "")
    subtype = result.get("subtype")
    if subtype in CAPPED:
        return "aborted_cap", CAPPED[subtype]
    if subtype == "success" and not result.get("is_error"):
        return "completed", "ended"
    detail = str(result.get("result") or result.get("errors") or "")[:200]
    return "infra_error", f"claude reported {subtype}{' (is_error)' if result.get('is_error') else ''}: {detail}"


def _served_models(session: Session) -> list[str]:
    """The models that served the main thread's messages, in order; the `init` event's model when none did."""
    served = [event["message"]["model"] for event in session.events
              if event.get("type") == "assistant" and event.get("parent_tool_use_id") is None
              and isinstance(event.get("message"), dict) and isinstance(event["message"].get("model"), str)
              and event["message"]["model"] != SYNTHETIC]
    init = session.init
    if not served and init and isinstance(init.get("model"), str):
        served = [_slug(init["model"])]
    return list(dict.fromkeys(served))


def _seed_config_dir(config_dir: Path, credentials: str) -> None:
    """A fresh config directory, holding the login only for `credentials_file`."""
    if config_dir.exists() and any(config_dir.iterdir()):
        raise CliError(f"{config_dir} is not empty; every session gets a fresh config directory")
    config_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    if credentials != "credentials_file":
        return
    source = Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude") / CREDENTIAL_FILE
    try:
        data = source.read_bytes()
    except OSError as err:
        raise CliError(f'credentials = "credentials_file" needs {source}: {err}') from None
    fd = os.open(config_dir / CREDENTIAL_FILE, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as handle:
        handle.write(data)


def _message_usage(raw: object, cache_write_ttl: str) -> dict | None:
    """An assistant message's API usage in the run-record shape (Anthropic's input_tokens exclude cache reads and
    writes); cache writes by their TTL when the message splits them. None without token counts."""
    if not isinstance(raw, dict):
        return None
    tokens_in, tokens_out = _count(raw.get("input_tokens")), _count(raw.get("output_tokens"))
    if tokens_in is None or tokens_out is None:
        return None
    usage = {"tokens_in": tokens_in, "tokens_out": tokens_out,
             "tokens_cache_read": _count(raw.get("cache_read_input_tokens")) or 0, "tokens_reasoning": 0}
    split = raw.get("cache_creation") if isinstance(raw.get("cache_creation"), dict) else {}
    five, hour = _count(split.get("ephemeral_5m_input_tokens")), _count(split.get("ephemeral_1h_input_tokens"))
    if five is not None and hour is not None:
        usage.update(tokens_cache_write_5m=five, tokens_cache_write_1h=hour)
    else:
        usage[f"tokens_cache_write_{cache_write_ttl}"] = _count(raw.get("cache_creation_input_tokens")) or 0
    return usage


def _model_usage(entry: Mapping, cache_write_ttl: str) -> dict | None:
    """One `modelUsage` entry in the run-record shape, with its cache writes at `cache_write_ttl`."""
    counts = [_count(entry.get(name)) for name in ("inputTokens", "outputTokens", "cacheReadInputTokens",
                                                     "cacheCreationInputTokens")]
    if None in counts:
        return None
    return {"tokens_in": counts[0], "tokens_out": counts[1], "tokens_cache_read": counts[2],
            f"tokens_cache_write_{cache_write_ttl}": counts[3],
            "tokens_reasoning": _count(entry.get("thinkingTokens")) or 0}  # already inside outputTokens


def _count(value: object) -> int | None:
    return value if isinstance(value, int) and not isinstance(value, bool) and value >= 0 else None


def _slug(model: str | None) -> str | None:
    return CONTEXT_TAG.sub("", model) if isinstance(model, str) else None


def _take(session: Session, meter: Meter, raw: bytes) -> dict | None:
    """Record one line of claude's stdout as an event; returns it, or None for a blank line."""
    text = raw.decode("utf-8", "replace").strip()
    if not text:
        return None
    try:
        event = json.loads(text)
    except ValueError:
        event = None
    if not isinstance(event, dict) or not isinstance(event.get("type"), str):
        event = {"type": "raw", "text": text[:2000]}
    session.events.append(event)
    meter.observe(event)
    return event


def _feed(stream: IO[bytes], data: bytes) -> None:
    try:
        stream.write(data)
        stream.close()
    except OSError:  # it exited before reading; its stderr says why
        pass


def _pump(stream: IO[bytes], put: Callable[[bytes | None], None]) -> None:
    try:
        for raw in iter(stream.readline, b""):
            put(raw)
    finally:
        put(None)


def _drain(stream: IO[bytes], tail: list[bytes]) -> None:
    for chunk in iter(lambda: stream.read1(65536), b""):
        tail.append(chunk)
        del tail[:-2]


def _kill_session(pid: int) -> None:
    try:
        os.killpg(pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


if __name__ == "__main__":
    sys.exit(main())
