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

What it cannot do: an agent under the same uid can still read a file it names by absolute path. Canaries catch reads
of benchmark files (`census`), and a container per task is the stronger option (S08 decision 4). The proof that the
secret file never reaches an agent is gap-a8a160's; it passes the secret and its canary as `forbidden_values`.

API:
    build(*, home: Path, extra: Mapping[str, str] | None = None, forbidden_values: Iterable[str] = ()) -> dict
    check(env: Mapping[str, str], forbidden_values: Iterable[str] = ()) -> None      # raises AgentEnvError
    FORBIDDEN_NAME, PASSTHROUGH, SYSTEM_PATH
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
    """Raise AgentEnvError if a name looks like a secret or a VB_ setting, or a value holds a forbidden string."""
    bad = sorted(name for name in env if FORBIDDEN_NAME.search(name))
    if bad:
        raise AgentEnvError(f"agent environment must not carry {', '.join(bad)}")
    for value in forbidden_values:
        if value and any(value in item for item in env.values()):
            raise AgentEnvError("agent environment carries a forbidden value")


def _base_python() -> Path:
    """The interpreter the driver's venv was made from, never a path inside the repository."""
    python = Path(getattr(sys, "_base_executable", None) or sys.executable).resolve()
    if layout.within(python, layout.REPO_ROOT) or not python.is_file():
        found = shutil.which("python3", path=os.pathsep.join(SYSTEM_PATH))
        if not found:
            raise AgentEnvError("no python3 outside the repository for agents to use")
        python = Path(found).resolve()
    return python
