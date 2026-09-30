"""Confine the agent's code while a truth suite runs it (gap-8c3752).

A truth suite (`hidden.py`) is trusted code. It reads the benchmark secret and the private task directory (DIR, which
holds `task.json`, the spec variants and the pristine bundle), draws the hidden cases, and then runs the agent's code
on them: F4 its migration script, F1 its converter in probe processes, the driver's toy family its `clamp`. That code
runs as the driver's user, and the census opens the secret file (mode 0400) for exactly as long as `hidden.py` runs.
So each suite starts the agent's code through `command`, which denies it every file operation on the paths it is
given: the secret file and DIR (`denied`). The census's visible re-run does the same.

- **macOS** (`KIND` "sandbox-exec"): `sandbox-exec` with a profile that allows everything else. Apple marks the tool
  deprecated, and it still works (checked on macOS 26.4). The sandbox matches resolved paths, so `profile` resolves
  them first. A read through a symlink is denied, and a hard link into a denied path cannot be made from inside. Code
  in the sandbox cannot start a setuid program either, `ps` among them. A hard link made earlier, in the agent's own
  run, changes the secret file's ctime, so the tripwire (`driver/secret.py`) has already kept the file from
  `hidden.py`.
- **Elsewhere** (`KIND` "none"): no confinement. Linux would need bubblewrap and unprivileged user namespaces, which
  this module does not use yet. The suites put the kind in their verdict, and the run record keeps it.

The network is left alone: whether agents get one at all is gap-0bd49a's decision.

API:
    KIND: str                                           # "sandbox-exec" or "none", on this host
    command(argv: Sequence[str], *, deny: Iterable[str | Path]) -> list[str]
    kind(deny: Sequence[str | Path]) -> str             # what `command` applies for `deny`: KIND, or "none"
    denied(secret_file: str | Path, task_file: str | Path) -> tuple[Path, Path]   # the secret file and DIR
    profile(deny: Iterable[str | Path]) -> str          # the macOS profile
"""

from __future__ import annotations

import os
import sys
from collections.abc import Iterable, Sequence
from pathlib import Path

SANDBOX_EXEC = "/usr/bin/sandbox-exec"
KIND = "sandbox-exec" if sys.platform == "darwin" and os.access(SANDBOX_EXEC, os.X_OK) else "none"


def command(argv: Sequence[str], *, deny: Iterable[str | Path]) -> list[str]:
    """`argv` run so that every file operation on a path in `deny` fails; `argv` itself when there is nothing to
    deny or no sandbox on this host (`KIND`)."""
    deny = list(deny)
    if KIND != "sandbox-exec" or not deny:
        return list(argv)
    return [SANDBOX_EXEC, "-p", profile(deny), *argv]


def kind(deny: Sequence[str | Path]) -> str:
    """The confinement `command` applies for `deny`: KIND, or "none" when there is nothing to deny."""
    return KIND if deny else "none"


def denied(secret_file: str | Path, task_file: str | Path) -> tuple[Path, Path]:
    """What a truth suite denies the agent's code: the secret file, and DIR, the directory of `task.json`."""
    return Path(secret_file), Path(task_file).parent


def profile(deny: Iterable[str | Path]) -> str:
    """A sandbox-exec profile that allows everything but file operations on the resolved `deny` paths."""
    paths = sorted({os.path.realpath(path) for path in deny})
    return "(version 1) (allow default)" + "".join(f' (deny file* (subpath "{_quoted(path)}"))' for path in paths)


def _quoted(path: str) -> str:
    return path.replace("\\", "\\\\").replace('"', '\\"')
