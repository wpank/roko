#!/usr/bin/env python3
"""The benchmark secret's driver-only file, and the checks that keep it from every agent (S08 §4.2 (4), SC4).

S08 §9 decision 8 put `VB_SECRET` in `~/.roko/.env`. That file is not private to the driver: `roko` loads it into its
own environment at startup (`load_startup_env_files`, `crates/roko-cli/src/main.rs`), and agents and verify commands
inherit roko's environment (bug-7d7200). So the secret lives in a file of its own (gap-a8a160):

- **Where.** `--secret-file`, else `$VB_SECRET_FILE`, else `~/.config/viabilitybench/secret` (`resolve`): a regular
  file owned by the driver's user that neither group nor others can access (0600), in a directory with the same rule
  (0700). It lies outside the repository, `~/.roko`, the workdirs (`$VB_WORK`) and the results (`$VB_RESULTS`, whose
  `private/` holds the task manifests), and neither of those two lies inside its directory.
- **Format** (`common.hmac_seed` reads it): UTF-8, one secret line, `#` lines ignored. `create` also writes a canary
  line of its own (`# canary vb-canary-…`), so an agent that prints the file trips the census even where the secret
  line is cut from the output.
- **Who reads it.** `vb run` reads it once before the first task (`preflight`), only to learn what to look for: the
  secret and its canary go to `agent_env.forbid`, so every agent environment is checked against them, and the census
  looks for them wherever an agent could have put them (`census`). The secret itself reaches only `hidden.py`, as
  `--secret-file PATH`, after the agent's processes have exited: never in an environment variable, never on a
  command line.
- **Fail closed.** `preflight` refuses the run when `VB_SECRET` is set in the driver's environment or a variable
  there holds the secret or its canary, or when a `.roko/.env` that `roko` would load defines `VB_SECRET` or holds
  the secret or its canary: `$HOME/.roko/.env`, `./.roko/.env`, the repository's, and those of the workspaces the
  caller names.

What mode 0600 does not do: an agent running as the driver's user can still read the file by its absolute path. This
design removes inheritance and accidental exposure. A deliberate read that reaches the transcript, the diff, the tree
or the census's own scratch trips the census, and the run becomes `leak_suspected`; a read that never shows the
secret or its canary cannot be seen this way. A container per task (S08 decision 4) is the stronger option. After a
`leak_suspected` run, rotate the secret: move the file aside and run `init`, which gives new hidden cases and a new
canary.

Usage (stdlib only):
    python3 benchmarks/viabilitybench/driver/secret.py init [--secret-file PATH]    # a new secret file
    python3 benchmarks/viabilitybench/driver/secret.py check [--secret-file PATH]   # its rules and exposures

API:
    DEFAULT_PATH, ENV_NAME, FILE_ENV
    resolve(value: str | Path | None = None) -> Path
    create(path: Path, *, value: str | None = None) -> Path             # never overwrites
    load(path: Path) -> DriverSecret                                     # raises SecretError
    preflight(path, *, work_root, results_root, workspaces=()) -> DriverSecret   # raises SecretError
    exposures(loaded: DriverSecret, *, workspaces=()) -> list[str]
    DriverSecret: .path, .fingerprint, .canary, .needles, .find(text) -> list[str], .find_in_file(path) -> list[str],
                  .redact(text) -> str
    SecretError, SECRET_LABEL
"""

from __future__ import annotations

import argparse
import os
import re
import secrets
import stat
import sys
from collections.abc import Iterable
from pathlib import Path

import agent_env
import layout
from common import canary, hmac_seed

DEFAULT_PATH = Path("~/.config/viabilitybench/secret")
ENV_NAME = "VB_SECRET"  # S08 decision 8's variable: set anywhere roko or the driver would read it, it fails closed
FILE_ENV = "VB_SECRET_FILE"
SECRET_LABEL = "vb-secret"
CANARY_LINE = re.compile(r"^#\s*canary\s+(" + canary.CANARY_RE.pattern + r")\s*$", re.IGNORECASE)
DEFINES_SECRET = re.compile(r"^\s*(?:export\s+)?" + ENV_NAME + r"\s*=", re.MULTILINE)
HEADER = "# ViabilityBench secret: keys the hidden test cases. Never pass it on argv or in the env."


class SecretError(ValueError):
    """The secret file breaks a rule, or the secret could reach an agent. Messages never hold the secret."""


class DriverSecret:
    """What the driver knows of the secret file: its path, and the strings no agent may see.

    Its repr never shows the secret, so it cannot leak through a log or a traceback.
    """

    __slots__ = ("path", "fingerprint", "canary", "_value")

    def __init__(self, path: Path, value: str, canary_value: str | None, fingerprint: str) -> None:
        self.path = path
        self.fingerprint = fingerprint
        self.canary = canary_value
        self._value = value

    @property
    def needles(self) -> tuple[str, ...]:
        """The secret and the file's canary: no agent environment, command line, prompt, log or record may hold one."""
        return tuple(item for item in (self._value, self.canary) if item)

    def find(self, text: str) -> list[str]:
        """`[SECRET_LABEL]` if `text` holds the secret, else `[]`: a label, never the secret.

        The canary is not reported here: it has canary form, so `canary.find` already finds it.
        """
        return [SECRET_LABEL] if self._value in text else []

    def find_in_file(self, path: Path) -> list[str]:
        """`find` over a file of any size, read in chunks that overlap by the secret's length. Follows links, so
        callers pass regular files only."""
        needle = self._value.encode("utf-8")
        tail = b""
        with open(path, "rb") as handle:
            while chunk := handle.read(1 << 20):
                window = tail + chunk
                if needle in window:
                    return [SECRET_LABEL]
                tail = window[1 - len(needle):]
        return []

    def redact(self, text: str) -> str:
        """`text` with the secret and the canary replaced by labels, for strings the driver writes down."""
        text = text.replace(self._value, f"<{SECRET_LABEL}>")
        return text.replace(self.canary, f"<{SECRET_LABEL}-canary>") if self.canary else text

    def __repr__(self) -> str:
        return f"DriverSecret({self.path}, <redacted> {self.fingerprint})"


def resolve(value: str | Path | None = None) -> Path:
    """The secret file's path: `value` (the `--secret-file` flag), else `$VB_SECRET_FILE`, else DEFAULT_PATH."""
    return Path(value or os.environ.get(FILE_ENV) or DEFAULT_PATH).expanduser().absolute()


def create(path: Path, *, value: str | None = None) -> Path:
    """Write a new secret file at `path`: a fresh 256-bit secret (or `value`, for tests) and a fresh canary line.

    The directory is created with mode 0700 if missing, and the file with mode 0600; an existing file is never
    replaced, since a new secret changes every hidden case. The result is read back with `load`.
    """
    path = Path(path).expanduser().absolute()
    value = secrets.token_hex(32) if value is None else value
    if len(value) < hmac_seed.MIN_SECRET_CHARS or value.strip() != value or "\n" in value or value.startswith("#"):
        raise SecretError(f"a secret is one line of at least {hmac_seed.MIN_SECRET_CHARS} characters, not starting "
                          "with #")
    _check_location(path)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.parent.stat().st_mode & 0o077:
        raise SecretError(f"{path.parent} is open to group or others; pick a private directory, or chmod 700 it")
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        raise SecretError(f"{path} already exists; move it aside first to rotate the secret") from None
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        handle.write(f"{HEADER}\n# canary {canary.new_canary()}\n{value}\n")
    os.chmod(path, 0o600)
    load(path)
    return path


def load(path: Path) -> DriverSecret:
    """The secret file at `path`, after checking its rules (see the module docstring). Raises SecretError."""
    path = Path(path).expanduser().absolute()
    try:
        checked = hmac_seed.read_secret_file(path)  # the format and the file's own mode, as hidden.py reads them
    except hmac_seed.SecretFileError as err:
        raise SecretError(str(err)) from None
    info, parent = path.lstat(), path.parent.stat()
    if info.st_uid != os.getuid() or parent.st_uid != os.getuid():
        raise SecretError(f"the secret file {path} and its directory must belong to the driver's user")
    if parent.st_mode & 0o077:
        raise SecretError(f"the secret file's directory {path.parent} is open to group or others; run: chmod 700 "
                          f"{path.parent}")
    _check_location(path)
    lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
    value = next(line for line in lines if line and not line.startswith("#"))
    canaries = [match.group(1).lower() for match in map(CANARY_LINE.match, lines) if match]
    return DriverSecret(path, value, canaries[0] if canaries else None, checked.fingerprint)


def preflight(path: Path, *, work_root: Path, results_root: Path, workspaces: Iterable[Path] = ()) -> DriverSecret:
    """Check the secret file and every place `roko` or an agent could inherit the secret from, before any task runs.

    Raises SecretError with every reason, so `vb run` exits before it creates a directory or makes a request. On
    success, registers the secret and its canary with `agent_env.forbid` and returns the loaded secret.
    """
    loaded = load(path)
    problems = []
    for root, flag in ((work_root, "--work"), (results_root, "--results")):
        if layout.within(loaded.path, root):
            problems.append(f"the secret file must live outside {flag} {root}, where agents work or records live")
        if layout.within(root, loaded.path.parent):
            problems.append(f"{flag} {root} must not lie inside the secret file's directory")
    problems += exposures(loaded, workspaces=workspaces)
    if problems:
        raise SecretError("refusing to run, the benchmark secret could reach an agent: " + "; ".join(problems))
    agent_env.forbid(loaded.needles)
    return loaded


def exposures(loaded: DriverSecret, *, workspaces: Iterable[Path] = ()) -> list[str]:
    """Why the secret could reach roko or an agent: the driver's environment, and the `.roko/.env` files roko loads."""
    found = []
    if ENV_NAME in os.environ:
        found.append(f"{ENV_NAME} is set in the driver's environment; unset it (the secret lives only in its file)")
    for name, value in sorted(os.environ.items()):
        if any(needle in value for needle in loaded.needles):
            found.append(f"the driver's environment variable {name} holds the secret or its canary")
    dotenvs = [_roko_home() / ".env", Path.cwd() / ".roko" / ".env", layout.REPO_ROOT / ".roko" / ".env",
               *(Path(workspace) / ".roko" / ".env" for workspace in workspaces)]
    for dotenv in dict.fromkeys(item.expanduser().absolute() for item in dotenvs):
        try:
            text = dotenv.read_text(encoding="utf-8", errors="replace")
        except FileNotFoundError:
            continue
        except OSError as err:
            found.append(f"cannot check {dotenv}: {err.strerror}")
            continue
        if DEFINES_SECRET.search(text):
            found.append(f"{dotenv} defines {ENV_NAME}, and roko loads that file into its environment; remove it")
        if any(needle in text for needle in loaded.needles):
            found.append(f"{dotenv} holds the secret or its canary, and roko loads that file into its environment")
    return found


def _check_location(path: Path) -> None:
    """Refuse a secret file inside the repository, where agents may read, or in `~/.roko`, which roko reads."""
    for root, name in ((layout.REPO_ROOT, "the repository"), (_roko_home(), "~/.roko")):
        if layout.within(path, root):
            raise SecretError(f"the secret file must live outside {name}, not at {path}")


def _roko_home() -> Path:
    """`$HOME/.roko`, the directory whose `.env` roko loads at startup."""
    return Path("~/.roko").expanduser().absolute()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="secret.py", description=__doc__.split("\n\n")[0], allow_abbrev=False)
    parser.add_argument("command", choices=("init", "check"))
    parser.add_argument("--secret-file", type=Path, metavar="PATH",
                        help=f"default: ${FILE_ENV}, then {DEFAULT_PATH}")
    args = parser.parse_args(argv)
    path = resolve(args.secret_file)
    try:
        loaded = load(create(path)) if args.command == "init" else load(path)
        problems = exposures(loaded)
    except SecretError as err:
        print(f"secret.py: {err}", file=sys.stderr)
        return 1
    for problem in problems:
        print(f"secret.py: {problem}", file=sys.stderr)
    if not loaded.canary:
        print(f"secret.py: {path} has no canary line; add `# canary {canary.new_canary()}` to it", file=sys.stderr)
    mode = stat.S_IMODE(path.stat().st_mode)
    print(f"{path} {loaded.fingerprint} mode={mode:04o} canary={'yes' if loaded.canary else 'no'}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
