#!/usr/bin/env python3
"""The visible-verify wrapper, installed as `run-shell` (S08 §4.6 `flaky_verify`, gap-4e8795): each arm's visible
checks run through it, and under the disturbance some of them fail.

`vb run` installs it for every task of a run whose disturbance spec names `flaky_verify` (`disturb.py`): a copy of
this file, a launcher and a config file go into the task's `.local/bin/`, the directory the agent's PATH starts with
(`agent_env.BIN_DIR`). Each arm's runner reaches it through `TaskContext.verify_wrapper`:
- Roko's plan runs its visible check as `<launcher> '<command>'`, in its verify step and its gate rung (`planemit`);
- the direct loop runs each of the agent's commands as `<launcher> '<command>'` instead of `bash -c '<command>'`
  (`mini_loop`);
- Claude Code runs each of its shell commands through it, as its `CLAUDE_CODE_SHELL_PREFIX` (`run_cli`).

**What the agent finds** (bug-d68575). The launcher, its config, its log and the module's copy have neutral names
(`run-shell`, `LAUNCHER`, `CONFIG`, `LOG`, `MODULE`), and the copy holds this module without its docstrings and
comments (`stripped_source`), so nothing the agent can list or read in its HOME names the benchmark or explains the
disturbance in words. Its code still shows what it does, as any program's does; hiding that needs the wrapper out of
the agent's reach, which a same-uid sandbox cannot give (S08 decision 4).

`run-shell COMMAND [ARG...]` does what `bash -c COMMAND [ARG...]` does, and replaces itself with that bash, so the
output, the exit status, signals and the process group are bash's. A command is a visible check run when it holds one
of the task's visible commands (the manifest's `visible_verify`, as the spec states them) as whole words. Each such
run gets the next number n and a line in the task's log, and fails with probability p: the wrapper then runs nothing,
prints `Killed` on stderr and exits 137, as a check the host killed would. The draw is HMAC-SHA256 of the
disturbance's seed and "<key>:<n>". The key `vb run` installs is the task's alias, never its task key (bug-3c921e):
the config lies in the agent's HOME, and a key there would tell the agent its family and level. So the seed and the
record's `provenance.alias` give a run's draws again, and another run of the same spec, whose aliases are new, draws
its own. p is the spec's
`params.p`, which S08 calls `VB_FLAKE_P`; the wrapper reads it from its config, since no `VB_*` variable may reach an
agent (`agent_env`). The agent is not told: no task text names the wrapper (Roko's prompt shows it only as the prefix
of the verify command, A4), and a flake looks like a crashed check. Outside the positions the disturbance covers, p
is 0 and the wrapper only counts.

After the task the driver reads the log (`read_log`) into the run record: `visible.verify_runs` counts the visible
check runs, `visible.flakes` lists the injected failures, and `visible.flake_injected` says whether there was one. The
census runs the visible checks itself, never through the wrapper, so no flake reaches a VS label.

It fails open: when its config or its log cannot be used, it runs the command and injects nothing. Limits: a check
run in another form (another interpreter, reordered arguments) is not seen, and a command that only names the check
(`echo "sh tests/visible/run.sh"`) counts as a run. An agent under the driver's uid can read or edit the wrapper's
files, and so learn p, as it can read anything else of its own (`agent_env`). The installed copy runs as
`python3 -I`, so it uses the standard library only.

API:
    install(bin_dir, *, key, visible, p, seed, shell, python=None) -> Path     # the launcher's path
    stripped_source() -> str                          # this module without its docstrings and comments: the copy
    read_log(wrapper: Path) -> list[dict] | None      # the task's visible check runs; None if the log is gone
    visible_run(command: str, visible) -> str | None  # the visible command that `command` runs, if any
    flaky(seed: int, key: str, number: int, p: float) -> bool
    main(argv: list[str]) -> int                      # returns only for a flake or a usage error
"""

from __future__ import annotations

import ast
import datetime as dt
import fcntl
import hashlib
import hmac
import json
import os
import shlex
import shutil
import sys
from collections.abc import Sequence
from pathlib import Path

LAUNCHER = "run-shell"  # the installed names, in the agent's HOME: none names the benchmark (bug-d68575)
MODULE = "run_shell.py"
CONFIG = "run-shell.json"
LOG = "run-shell.jsonl"
FLAKE_STATUS = 137  # the status of a check the host killed (SIGKILL), which is what a flake looks like
BOUNDARY = frozenset(" \t\n;&|()<>'\"`{}")  # what may stand next to a visible command for it to be whole words


def install(bin_dir: Path, *, key: str, visible: Sequence[str], p: float, seed: int, shell: str,
            python: Path | None = None) -> Path:
    """Put the wrapper into `bin_dir`, with an empty log, its draws keyed by `key` (`vb run` passes the task's alias,
    which the config shows the agent); returns the launcher's path."""
    bin_dir = Path(bin_dir).absolute()
    if python is None:  # agent_env links its bin directory's python3 to the driver's base interpreter
        python = bin_dir / "python3" if (bin_dir / "python3").exists() else Path(sys.executable)
    (bin_dir / MODULE).write_text(stripped_source(), encoding="utf-8")
    config = {"key": key, "visible": [command.strip() for command in visible if command.strip()], "p": p,
              "seed": seed, "shell": shell, "log": str(bin_dir / LOG)}
    (bin_dir / CONFIG).write_text(json.dumps(config) + "\n", encoding="utf-8")
    (bin_dir / LOG).write_text("", encoding="utf-8")
    launcher = bin_dir / LAUNCHER
    launcher.write_text(f'#!/bin/sh\nexec {shlex.quote(str(python))} -I {shlex.quote(str(bin_dir / MODULE))} "$@"\n',
                        encoding="utf-8")
    launcher.chmod(0o755)
    return launcher


def stripped_source() -> str:
    """This module's source without its docstrings and comments, which `install` copies into the agent's HOME: the
    agent may read the copy, and the docstrings would tell it what the wrapper is for (module docstring). An AST round
    trip, so the copy runs the same code."""
    tree = ast.parse(Path(__file__).read_text(encoding="utf-8"))
    for node in ast.walk(tree):
        body = getattr(node, "body", None)
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)) and body and \
                isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) and \
                isinstance(body[0].value.value, str):
            node.body = body[1:] or [ast.Pass()]
    return ast.unparse(tree) + "\n"


def read_log(wrapper: Path) -> list[dict] | None:
    """The visible check runs that the wrapper at `wrapper` logged, in order. Lines that are not runs are skipped."""
    try:
        lines = Path(wrapper).with_name(LOG).read_text(encoding="utf-8").splitlines()
    except OSError:
        return None
    rows = []
    for line in lines:
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict) and type(row.get("run")) is int and isinstance(row.get("flake"), bool):
            rows.append(row)
    return rows


def visible_run(command: str, visible: Sequence[str]) -> str | None:
    """The first of `visible` that `command` holds as whole words, or None."""
    for check in visible:
        start = command.find(check) if check else -1
        while start >= 0:
            end = start + len(check)
            if (start == 0 or command[start - 1] in BOUNDARY) and (end == len(command) or command[end] in BOUNDARY):
                return check
            start = command.find(check, start + 1)
    return None


def flaky(seed: int, key: str, number: int, p: float) -> bool:
    """Whether visible check run `number` of task `key` fails: a draw below p, from HMAC-SHA256(seed, key:number)."""
    if p <= 0:
        return False
    digest = hmac.new(str(seed).encode(), f"{key}:{number}".encode(), hashlib.sha256).digest()
    return int.from_bytes(digest[:8], "big") / 2**64 < p


def main(argv: list[str]) -> int:
    if not argv:
        print(f"usage: {LAUNCHER} COMMAND [ARG...]", file=sys.stderr)
        return 2
    try:
        config = json.loads(Path(__file__).with_name(CONFIG).read_text(encoding="utf-8"))
        shell = str(config["shell"])
    except (OSError, ValueError, KeyError, TypeError):
        config, shell = None, shutil.which("bash") or "/bin/sh"
    if config is not None:
        check = visible_run(argv[0], config.get("visible") or [])
        if check is not None and _flakes(config, check):
            print("Killed", file=sys.stderr, flush=True)
            return FLAKE_STATUS
    try:
        os.execv(shell, [shell, "-c", *argv])
    except OSError as err:
        print(f"{LAUNCHER}: cannot run {shell}: {err.strerror}", file=sys.stderr)
        return 127


def _flakes(config: dict, check: str) -> bool:
    """Number this visible check run, log it, and say whether it fails. A run that cannot be logged never fails."""
    try:
        with open(config["log"], "a+", encoding="utf-8") as log:
            fcntl.flock(log, fcntl.LOCK_EX)
            log.seek(0)
            number = 1 + sum(1 for line in log if line.strip())
            flake = flaky(int(config["seed"]), str(config["key"]), number, float(config["p"]))
            at = dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
            log.write(json.dumps({"run": number, "flake": flake, "check": check, "at": at}) + "\n")
            log.flush()
        return flake
    except (OSError, ValueError, KeyError, TypeError):
        return False


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
