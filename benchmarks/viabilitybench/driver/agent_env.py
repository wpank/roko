"""The one environment builder for agent processes (S08 §4.2 (4), SC4).

Every process an agent controls (today the direct loop's shell commands; later `roko` and `claude`, and the census's
runs of agent code) gets its environment from `build`, which is an allowlist: nothing of the driver's environment
passes except a few locale settings. No `VB_*` variable (the secret file, workdir and results paths) and no provider
key can reach an agent, and `check` asserts that on the finished environment.

- HOME and TMPDIR point into a per-task directory outside the workdir, so `~` never reaches the real home and its
  `~/.roko/.env`.
- PATH starts with a per-task `.vb-bin/`, whose `python3` and `python` link to the driver's own base interpreter (the
  families need Python 3.11 or newer), followed by the system directories.
- PYTHONDONTWRITEBYTECODE keeps `__pycache__` out of the tree the census labels; git gets a fixed identity and no
  system config, so an agent's `git commit` behaves the same on every host.

The benchmark secret (gap-a8a160): `secret.preflight` reads the secret file once before the first task and passes the
secret and its canary to `forbid`, so every environment built or checked for the rest of the run is refused if any
value holds either. Runners never handle the secret. `check` also refuses an environment without a HOME of its own:
an agent whose HOME is the driver's would find the real `~/.roko/.env` and `~/.config/viabilitybench/`.

What it cannot do: an agent under the same uid can still read a file it names by absolute path, and see the driver's
own environment with `ps -E` or `/proc/<pid>/environ`. So `secret.preflight` keeps the secret and every provider key out
of the driver's environment (the keys live in a driver-only key file, bug-979a06), and registers both with `forbid`.
Canaries catch reads of benchmark files, and the tripwire catches a read of the secret file or the key file, which
must be chmod'ed first while agents run (`census`, gap-308373). A container per task is the stronger option (S08
decision 4).

**The driver's own environment** (bug-32eb77). An agent can read the environment the driver started with, so any
credential the operator's shell exports would reach every agent: `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, cloud keys.
- **Scrub.** A `vb run` process starts itself again once its checks have passed, with its environment cut to an
  allowlist (`driver_env`: DRIVER_PASSTHROUGH and `LC_*`). It keeps the same command line and process id
  (`exec_scrubbed`), so every agent, and every other child of the driver, sees only that. Deleting variables after
  start would not do: `ps -E` and `/proc` show the start-up environment.
- **Checks first.** The checks run before the restart, on the operator's environment, so a secret or a provider key
  in it is still refused rather than hidden (`secret.preflight`).
- **Limit.** Every other process of the operator's user stays readable: `ps -E -ax` shows the start-up environment of
  any of them that is not an Apple system binary (a shell started with a credential, an editor, another session).
  Only a separate user or a container per task hides those (S08 decision 4). Until then, run the benchmark from a
  session that exports no credential.

API:
    build(*, home: Path, extra: Mapping[str, str] | None = None, forbidden_values: Iterable[str] = ()) -> dict
    check(env: Mapping[str, str], forbidden_values: Iterable[str] = ()) -> None      # raises AgentEnvError
    forbid(values: Iterable[str]) -> None           # values refused in every later build and check (the secret)
    driver_env(env: Mapping[str, str] | None = None) -> dict        # the allowlisted driver environment
    exec_scrubbed() -> None                         # start again with `driver_env()`, once; returns if already done
    FORBIDDEN_NAME, PASSTHROUGH, SYSTEM_PATH, DRIVER_PASSTHROUGH, DRIVER_SCRUBBED
"""

from __future__ import annotations

import os
import re
import shutil
import sys
from collections.abc import Iterable, Mapping
from pathlib import Path

import layout

PASSTHROUGH = ("LANG", "LC_ALL", "LC_CTYPE", "TZ")
SYSTEM_PATH = ("/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin")
FORBIDDEN_NAME = re.compile(r"^VB_|API_?KEY|TOKEN|SECRET|PASSW|CREDENTIAL|^AWS_|^ANTHROPIC_|^OPENAI_|^CEREBRAS_",
                            re.IGNORECASE)
FIXED = {"SHELL": "/bin/bash", "TERM": "dumb", "NO_COLOR": "1", "PAGER": "cat", "GIT_PAGER": "cat",
         "PYTHONDONTWRITEBYTECODE": "1", "GIT_CONFIG_NOSYSTEM": "1", "GIT_TERMINAL_PROMPT": "0",
         "GIT_AUTHOR_NAME": "vb-agent", "GIT_AUTHOR_EMAIL": "agent@vb.invalid",
         "GIT_COMMITTER_NAME": "vb-agent", "GIT_COMMITTER_EMAIL": "agent@vb.invalid"}
_forbidden: tuple[str, ...] = ()  # set by `forbid`: the benchmark secret and its canary, once `vb run` has read them
# What the driver itself keeps of the operator's environment: paths, locale and its own settings, never a credential.
DRIVER_PASSTHROUGH = ("PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "TZ", "SSL_CERT_FILE", "SSL_CERT_DIR",
                      "CLAUDE_CONFIG_DIR", "VB_SECRET_FILE", "VB_KEY_FILE", "VB_RESULTS", "VB_WORK")
DRIVER_SCRUBBED = "VB_DRIVER_ENV"  # "scrubbed" in the environment `exec_scrubbed` starts the driver with


class AgentEnvError(ValueError):
    """An agent environment would carry a forbidden variable or value."""


def build(*, home: Path, extra: Mapping[str, str] | None = None, forbidden_values: Iterable[str] = ()) -> dict:
    home = Path(home).absolute()
    if layout.within(home, layout.REPO_ROOT):
        raise AgentEnvError(f"an agent's HOME must be outside the repository, not {home}")
    bin_dir, tmp = home / ".vb-bin", home / "tmp"
    for directory in (home, bin_dir, tmp):
        directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    python = _base_python()
    for name in ("python3", "python"):
        link = bin_dir / name
        if not link.is_symlink():
            link.symlink_to(python)
    env = {name: os.environ[name] for name in PASSTHROUGH if name in os.environ}
    env.update(FIXED)
    env.update(HOME=str(home), TMPDIR=str(tmp), PATH=os.pathsep.join([str(bin_dir), *SYSTEM_PATH]),
               USER=os.environ.get("USER", "vb-agent"), LOGNAME=os.environ.get("LOGNAME", "vb-agent"))
    env.update(extra or {})
    check(env, forbidden_values)
    return env


def check(env: Mapping[str, str], forbidden_values: Iterable[str] = ()) -> None:
    """Raise AgentEnvError if a name looks like a secret or a VB_ setting, a value holds a forbidden string (the
    given ones, and those registered with `forbid`), or HOME is missing, the driver's own, or in the repository."""
    bad = sorted(name for name in env if FORBIDDEN_NAME.search(name))
    if bad:
        raise AgentEnvError(f"agent environment must not carry {', '.join(bad)}")
    for value in (*forbidden_values, *_forbidden):
        if value and any(value in item for item in env.values()):
            raise AgentEnvError("agent environment carries a forbidden value")
    home = env.get("HOME")
    if not home:
        raise AgentEnvError("an agent environment needs a HOME of its own")
    driver_home = os.environ.get("HOME")
    if (driver_home and Path(home).resolve() == Path(driver_home).resolve()) or layout.within(home, layout.REPO_ROOT):
        raise AgentEnvError(f"an agent's HOME must be a directory of its own, not {home}")


def forbid(values: Iterable[str]) -> None:
    """Refuse `values` in every environment `build` or `check` sees from now on; each call replaces the last one.

    `secret.preflight` passes the benchmark secret and its canary here once per run.
    """
    global _forbidden
    _forbidden = tuple(value for value in values if value)


def driver_env(env: Mapping[str, str] | None = None) -> dict:
    """`env` (by default the driver's) cut to DRIVER_PASSTHROUGH and the `LC_*` locale settings, and marked scrubbed."""
    env = os.environ if env is None else env
    kept = {name: value for name, value in env.items() if name in DRIVER_PASSTHROUGH or name.startswith("LC_")}
    return {**kept, DRIVER_SCRUBBED: "scrubbed"}


def exec_scrubbed() -> None:
    """Start this process again with the same command line and `driver_env()`: it never returns then. It returns at
    once in a process that `exec_scrubbed` started, so the restart happens once."""
    if os.environ.get(DRIVER_SCRUBBED) == "scrubbed":
        return
    sys.stdout.flush()
    sys.stderr.flush()
    os.execve(sys.executable, sys.orig_argv, driver_env())


def _base_python() -> Path:
    """The interpreter the driver's venv was made from, never a path inside the repository."""
    python = Path(getattr(sys, "_base_executable", None) or sys.executable).resolve()
    if layout.within(python, layout.REPO_ROOT) or not python.is_file():
        found = shutil.which("python3", path=os.pathsep.join(SYSTEM_PATH))
        if not found:
            raise AgentEnvError("no python3 outside the repository for agents to use")
        python = Path(found).resolve()
    return python
