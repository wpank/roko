#!/usr/bin/env python3
"""The driver-only files, the benchmark secret's and the provider keys', and the checks that keep them from every agent
(S08 §4.2 (4), SC4).

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

What mode 0600 does not do: an agent running as the driver's user can still open the file by its absolute path. This
design removes inheritance and accidental exposure. A deliberate read that reaches the transcript, the diff, the tree
or the census's own scratch trips the census, and the run becomes `leak_suspected`. A read that never shows the
secret or its canary is what the tripwire is for.

**The tripwire** (gap-308373). While `vb run`'s tasks run (`tripwire`), the file is at mode 000, so even its owner
must `chmod` it before reading it. A chmod sets the file's ctime, and no normal user can set a ctime back.
- **The stamp.** Each `Tripwire` keeps the file's device, inode, mode and ctime as of the driver's own last chmod.
- **Per task.** The census compares the file with that stamp after the agent's run, and makes the file readable
  (0400) only while `hidden.py` runs. Any change (`missing`, `replaced`, `mode`, `ctime`) marks the run
  `leak_suspected`, at place `secret-file:<change>` (`census:secret-file:<change>` during the census). A changed
  file is never handed to `hidden.py`.
- **At rest** the file is 0600, and `preflight` refuses it at any other mode. Another mode means a `vb run` is using
  it (one run per secret file at a time), or a run was killed with the file armed.
- **One run at a time.** `tripwire` also holds an exclusive lock on `<file>.lock` next to each file it arms. A second
  `vb run` on the same file is refused even when both passed `preflight` at once; the lock goes with the process.
- **Ending a run.** While `tripwire` holds the files, SIGTERM and SIGHUP (unless ignored, as under `nohup`) raise
  SystemExit, so the run unwinds through the `finally` blocks that put the files back at 0600; by default Python dies
  on either without running them. Only a SIGKILL leaves the files armed, and then `preflight` says so.
- **Limits.** It detects, it does not prevent. A read by agent code while `hidden.py` has the file open still goes
  unseen (gap-8c3752), as does a debugger reading the driver's memory, and an agent running as root needs no chmod.
  A container per task (S08 decision 4) prevents all three. It also needs sub-second ctimes (APFS, ext4 and the
  like): `Tripwire.arm` refuses a file whose filesystem keeps whole seconds.

After a `leak_suspected` run, rotate the secret: move the file aside and run `init`, which gives new hidden cases and a
new canary.

**Provider keys** (bug-979a06). An agent under the driver's user can read the environment the driver started with
(`ps -E -p $PPID` on macOS, `/proc/<ppid>/environ` on Linux); deleting a variable afterwards hides nothing. So no
provider key lives in the driver's environment. The keys live in a key file under the secret file's rules
(`--key-file`, else `$VB_KEY_FILE`, else `~/.config/viabilitybench/keys`), with `NAME=value` lines named by the arm
files' `api_key_env` and `#` lines ignored.
- **Who reads it.** `load_keys` reads it into the driver's memory. Only the driver's own code sends a key: its chat
  client (`provider.OpenAICompatible`) and the metering proxy (`faultproxy`). Roko, whose tools run agents'
  commands, gets the proxy's loopback URL and a placeholder key (`run_roko`).
- **Fail closed.** `preflight` refuses a run whose environment holds any arm file's `api_key_env` (`arm_key_names`)
  or a loaded key under any name, and registers the key values with `agent_env.forbid`.
- **The tripwire.** It holds the key file at mode 000 for the whole run as well, since nothing reads the file after
  startup (place `key-file:<change>`).
- **Escape hatch.** `KEYS_IN_ENV_OK` is for tests that keep a key in the driver's environment to show that agents
  never see it.

Usage (stdlib only):
    python3 benchmarks/viabilitybench/driver/secret.py init [--secret-file PATH]    # a new secret file
    python3 benchmarks/viabilitybench/driver/secret.py check [--secret-file PATH]   # its rules and exposures
    python3 benchmarks/viabilitybench/driver/secret.py keys [--key-file PATH]       # the key file's names and rules

API:
    DEFAULT_PATH, ENV_NAME, FILE_ENV, KEYS_DEFAULT_PATH, KEYS_FILE_ENV, KEYS_IN_ENV_OK
    resolve(value: str | Path | None = None) -> Path; resolve_keys(value=None) -> Path
    create(path: Path, *, value: str | None = None) -> Path             # never overwrites
    create_keys(path: Path, keys: Mapping[str, str]) -> Path             # never overwrites; tests and setup
    load(path: Path) -> DriverSecret                                     # raises SecretError
    load_keys(path: Path, *, need: Iterable[str] = ()) -> ProviderKeys   # raises SecretError
    preflight(path, *, work_root, results_root, workspaces=(), keys=None) -> DriverSecret   # raises SecretError
    exposures(loaded: DriverSecret, *, workspaces=()) -> list[str]; key_exposures(keys=None) -> list[str]
    arm_key_names() -> frozenset[str]
    tripwire(loaded: DriverSecret, keys: ProviderKeys | None = None)    # a context manager around the tasks
    armed(path) -> Tripwire | None; tripwires() -> tuple[Tripwire, ...]
    Tripwire: .place, .path, .secret, .arm(), .set(mode) -> bool, .check() -> list[str], .opened()
    DriverSecret: .path, .fingerprint, .canary, .needles, .find(text) -> list[str], .find_in_file(path) -> list[str],
                  .redact(text) -> str
    ProviderKeys: a Mapping of name -> key, with .path; its repr shows names only
    SecretError, SECRET_LABEL, TRIPWIRE_LABEL, ARMED_MODE, OPEN_MODE, REST_MODE
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import os
import re
import secrets
import signal
import stat
import sys
import threading
import tomllib
from collections.abc import Iterable, Iterator, Mapping
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
KEYS_DEFAULT_PATH = Path("~/.config/viabilitybench/keys")
KEYS_FILE_ENV = "VB_KEY_FILE"
KEYS_HEADER = "# ViabilityBench provider keys, NAME=value, read by the driver only. Never export them."
KEY_LINE = re.compile(r"^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$")
MIN_KEY_CHARS = 16  # a shorter value is no provider key, and would make `agent_env.forbid` refuse common strings
KEYS_IN_ENV_OK = False  # tests only: `preflight` then passes a driver environment that holds a provider key
TRIPWIRE_LABEL = "vb-tripwire"
ARMED_MODE, OPEN_MODE, REST_MODE = 0o000, 0o400, 0o600  # while agents run; while hidden.py reads it; between runs
_armed: dict[Path, Tripwire] = {}  # set by `tripwire` for the tasks of one `vb run`


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


class ProviderKeys(Mapping):
    """The key file's provider keys, by the variable name the arm files give them (`api_key_env`).

    Its repr shows the names only, so no key leaks through a log or a traceback.
    """

    __slots__ = ("path", "_keys")

    def __init__(self, path: Path | None, keys: Mapping[str, str]) -> None:
        self.path = path
        self._keys = dict(keys)

    def __getitem__(self, name: str) -> str:
        return self._keys[name]

    def __iter__(self) -> Iterator[str]:
        return iter(self._keys)

    def __len__(self) -> int:
        return len(self._keys)

    def __repr__(self) -> str:
        return f"ProviderKeys({self.path}, {', '.join(sorted(self._keys)) or 'no keys'})"


class Tripwire:
    """One driver-only file held at mode 000 while agents run, and the stamp that shows whether anything changed it.

    The stamp is the file's device, inode, mode and ctime as of the driver's own last chmod (see the module
    docstring). `place` names the file in the census's places: `secret-file` or `key-file`.
    """

    __slots__ = ("place", "path", "secret", "_stamp")

    def __init__(self, place: str, path: Path, secret: DriverSecret | None = None) -> None:
        self.place = place
        self.path = Path(path)
        self.secret = secret  # the loaded secret, for the census: the armed file cannot be read again
        self._stamp: tuple[int, int, int, int] | None = None

    def arm(self) -> None:
        """Take the file to mode 000 for the first time. Raises SecretError for a file that is not a regular file of
        the driver's user, or whose filesystem keeps whole-second ctimes, where a change could hide within the
        second of the driver's own chmod."""
        try:
            info = self.path.lstat()
        except OSError as err:
            raise SecretError(f"cannot arm the tripwire on {self.path}: {err.strerror}") from None
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
            raise SecretError(f"cannot arm the tripwire on {self.path}: not a regular file of the driver's user")
        self._stamp = _stamp(info)
        self.set(ARMED_MODE)
        if self._stamp[3] % 1_000_000_000 == 0:
            self.set(REST_MODE)
            raise SecretError(f"{self.path}'s filesystem keeps whole-second timestamps, so the tripwire cannot see a "
                              "change made in the same second as its own; keep the file on APFS, ext4 or the like")

    def set(self, mode: int) -> bool:
        """chmod the file to `mode` and stamp it again, unless it is no longer the file stamped (a link or another
        file never gets the driver's chmod). Returns whether it did."""
        try:
            info = self.path.lstat()
        except OSError:
            return False
        if self._stamp is None or (info.st_dev, info.st_ino) != self._stamp[:2] or not stat.S_ISREG(info.st_mode):
            return False
        os.chmod(self.path, mode)
        self._stamp = _stamp(self.path.lstat())
        return True

    def check(self) -> list[str]:
        """What changed since the last stamp: `missing`, `replaced`, or `mode` and `ctime`; [] when nothing did."""
        try:
            now = _stamp(self.path.lstat())
        except OSError:
            return ["missing"]
        if self._stamp is None or now[:2] != self._stamp[:2]:
            return ["replaced"]
        return [name for name, index in (("mode", 2), ("ctime", 3)) if now[index] != self._stamp[index]]

    @contextlib.contextmanager
    def opened(self) -> Iterator[list[str]]:
        """Mode 0400 for the block (hidden.py reads the file), then 000 again. The list it yields receives what
        changed the file during the block."""
        changes: list[str] = []
        self.set(OPEN_MODE)
        try:
            yield changes
        finally:
            changes += self.check()
            self.set(ARMED_MODE)

    def __repr__(self) -> str:
        return f"Tripwire({self.place}, {self.path})"


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
    _check_not_armed(path)
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


def preflight(path: Path, *, work_root: Path, results_root: Path, workspaces: Iterable[Path] = (),
              keys: ProviderKeys | None = None) -> DriverSecret:
    """Check the secret file, the key file (`keys`, from `load_keys`) and every place `roko` or an agent could inherit
    the secret or a provider key from, before any task runs.

    Raises SecretError with every reason, so `vb run` exits before it creates a directory or makes a request. On
    success, registers the secret, its canary and the keys with `agent_env.forbid` and returns the loaded secret.
    """
    loaded = load(path)
    problems = []
    mode = stat.S_IMODE(loaded.path.lstat().st_mode)
    if mode != REST_MODE:
        problems.append(f"the secret file is at mode {mode:04o}, not {REST_MODE:04o}: a vb run is using it (one run "
                        f"per secret file at a time), or its mode was changed by hand; chmod {REST_MODE:o} it once no "
                        "run is using it")
    files = [("secret file", loaded.path), *([("key file", keys.path)] if keys is not None and keys.path else [])]
    for what, file in files:
        for root, flag in ((work_root, "--work"), (results_root, "--results")):
            if layout.within(file, root):
                problems.append(f"the {what} must live outside {flag} {root}, where agents work or records live")
            if layout.within(root, file.parent):
                problems.append(f"{flag} {root} must not lie inside the {what}'s directory")
    problems += exposures(loaded, workspaces=workspaces)
    problems += key_exposures(keys)
    if problems:
        raise SecretError("refusing to run, the benchmark secret or a provider key could reach an agent: "
                          + "; ".join(problems))
    agent_env.forbid([*loaded.needles, *(keys.values() if keys is not None else ())])
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


def resolve_keys(value: str | Path | None = None) -> Path:
    """The key file's path: `value` (the `--key-file` flag), else `$VB_KEY_FILE`, else KEYS_DEFAULT_PATH."""
    return Path(value or os.environ.get(KEYS_FILE_ENV) or KEYS_DEFAULT_PATH).expanduser().absolute()


def create_keys(path: Path, keys: Mapping[str, str]) -> Path:
    """Write a new key file at `path` holding `keys` (tests and first-time setup), with the secret file's rules: the
    directory is created with mode 0700 if missing, the file with mode 0600, and an existing file is never replaced.
    The result is read back with `load_keys`."""
    path = Path(path).expanduser().absolute()
    for name, value in keys.items():
        if not KEY_LINE.fullmatch(f"{name}=x") or len(value) < MIN_KEY_CHARS or _has_space(value):
            raise SecretError(f"a key file holds NAME=value lines with keys of at least {MIN_KEY_CHARS} characters "
                              "and no spaces")
    _check_location(path, "key file")
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.parent.stat().st_mode & 0o077:
        raise SecretError(f"{path.parent} is open to group or others; pick a private directory, or chmod 700 it")
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        raise SecretError(f"{path} already exists; edit it, or move it aside first") from None
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        handle.write("\n".join([KEYS_HEADER, *(f"{name}={value}" for name, value in keys.items())]) + "\n")
    os.chmod(path, 0o600)
    return load_keys(path, need=keys).path


def load_keys(path: Path, *, need: Iterable[str] = ()) -> ProviderKeys:
    """The key file at `path`, after checking the secret file's rules and its format; `need` names the keys it must
    hold. Raises SecretError, whose messages never hold a key."""
    path = Path(path).expanduser().absolute()
    _check_location(path, "key file")
    try:
        info, parent = path.lstat(), path.parent.stat()
    except OSError as err:
        raise SecretError(f"cannot read the key file {path}: {err.strerror}") from None
    if not stat.S_ISREG(info.st_mode):
        raise SecretError(f"the key file {path} must be a regular file, not a link or a directory")
    if info.st_uid != os.getuid() or parent.st_uid != os.getuid():
        raise SecretError(f"the key file {path} and its directory must belong to the driver's user")
    if info.st_mode & 0o077:
        raise SecretError(f"the key file {path} is accessible to group or others; run: chmod 600 {path}")
    if parent.st_mode & 0o077:
        raise SecretError(f"the key file's directory {path.parent} is open to group or others; run: chmod 700 "
                          f"{path.parent}")
    _check_not_armed(path, "key file")
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as err:
        raise SecretError(f"cannot read the key file {path}: {type(err).__name__}") from None
    keys: dict[str, str] = {}
    for number, line in enumerate(text.splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        match = KEY_LINE.fullmatch(line)
        value = _unquote(match.group(2).strip()) if match else ""
        if not match or len(value) < MIN_KEY_CHARS or _has_space(value):
            raise SecretError(f"{path} line {number} is not NAME=value with a key of at least {MIN_KEY_CHARS} "
                              "characters")
        if match.group(1) in keys:
            raise SecretError(f"{path} line {number} sets {match.group(1)} a second time")
        keys[match.group(1)] = value
    missing = sorted(set(need) - set(keys))
    if missing:
        raise SecretError(f"the key file {path} has no {', '.join(missing)}")
    return ProviderKeys(path, keys)


def arm_key_names() -> frozenset[str]:
    """The provider key variables the arm files name (`[providers.*] api_key_env`)."""
    names = set()
    for path in sorted(layout.ARMS_DIR.glob("*.toml")):
        try:
            with path.open("rb") as handle:
                doc = tomllib.load(handle)
        except (OSError, tomllib.TOMLDecodeError):
            continue  # vb.load_arm reports a broken arm file
        for table in doc.get("providers", {}).values():
            if isinstance(table, dict) and isinstance(table.get("api_key_env"), str):
                names.add(table["api_key_env"])
    return frozenset(names)


def key_exposures(keys: Mapping[str, str] | None = None) -> list[str]:
    """Why a provider key could reach an agent: the driver's environment sets an arm file's `api_key_env` or a name in
    `keys`, or holds one of `keys` under any name. Every agent under the driver's user can read that environment (the
    one it started with: `ps -E`, `/proc/<pid>/environ`). Empty while `KEYS_IN_ENV_OK` (tests only)."""
    if KEYS_IN_ENV_OK:
        return []
    keys = keys or {}
    named = [name for name in sorted(arm_key_names() | set(keys)) if name in os.environ]
    found = [f"{name} is set in the driver's environment, which any agent can read (ps -E, /proc/<pid>/environ); "
             f"keep the key only in the key file (--key-file, ${KEYS_FILE_ENV} or {KEYS_DEFAULT_PATH}) and unset it"
             for name in named]
    found += [f"the driver's environment variable {name} holds a provider key" for name, value in
              sorted(os.environ.items()) if name not in named and any(key in value for key in keys.values())]
    return found


@contextlib.contextmanager
def tripwire(loaded: DriverSecret, keys: ProviderKeys | None = None) -> Iterator[tuple[Tripwire, ...]]:
    """Hold the secret file, and the key file when `keys` came from one, at mode 000 for the block: `vb run`'s
    tasks. The census checks them (`armed`, `tripwires`), and both are back at 0600 afterwards, also after a SIGTERM
    or SIGHUP. Raises SecretError when a file cannot be armed (`Tripwire.arm`), or another run holds it."""
    wires = [Tripwire("secret-file", loaded.path, loaded),
             *([Tripwire("key-file", keys.path)] if keys is not None and keys.path is not None else [])]
    done: list[Tripwire] = []
    with contextlib.ExitStack() as stack:
        stack.enter_context(_signals_exit())
        try:
            for wire in wires:
                if wire.path in _armed:
                    raise SecretError(f"{wire.path} is armed already: one vb run per file at a time")
                stack.enter_context(_run_lock(wire.path))
                wire.arm()
                _armed[wire.path] = wire
                done.append(wire)
            yield tuple(wires)
        finally:
            for wire in done:
                del _armed[wire.path]
                wire.set(REST_MODE)


@contextlib.contextmanager
def _run_lock(path: Path) -> Iterator[None]:
    """An exclusive lock on `<path>.lock` for the block: one `vb run` per armed file at a time. The lock is released
    when its descriptor closes, which the process's death does too."""
    lock = path.with_name(path.name + ".lock")
    fd = os.open(lock, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SecretError(f"another vb run holds {lock}: one run per file at a time") from None
        yield
    finally:
        os.close(fd)


@contextlib.contextmanager
def _signals_exit() -> Iterator[None]:
    """For the block, SIGTERM and SIGHUP raise SystemExit, unless they are ignored (`nohup`) or handled already, so
    the run unwinds through its `finally` blocks. Signal handlers belong to the main thread; elsewhere, nothing."""
    previous = {}
    if threading.current_thread() is threading.main_thread():
        previous = {sig: signal.signal(sig, _raise_exit) for sig in (signal.SIGTERM, signal.SIGHUP)
                    if signal.getsignal(sig) is signal.SIG_DFL}
    try:
        yield
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def _raise_exit(signum: int, frame: object) -> None:
    raise SystemExit(128 + signum)


def armed(path: Path) -> Tripwire | None:
    """The tripwire on `path` while `tripwire` holds it, else None."""
    return _armed.get(Path(path).expanduser().absolute())


def tripwires() -> tuple[Tripwire, ...]:
    """Every file `tripwire` holds armed now."""
    return tuple(_armed.values())


def _stamp(info: os.stat_result) -> tuple[int, int, int, int]:
    return info.st_dev, info.st_ino, stat.S_IMODE(info.st_mode), info.st_ctime_ns


def _check_not_armed(path: Path, what: str = "secret file") -> None:
    """Refuse a file its owner cannot read: mode 000 is the tripwire's, so a run is using the file, or a killed run
    left it armed."""
    try:
        info = path.lstat()
    except OSError:
        return  # the caller reports a missing file
    if stat.S_ISREG(info.st_mode) and not info.st_mode & 0o400:
        raise SecretError(f"the {what} {path} is at mode {stat.S_IMODE(info.st_mode):04o}, the tripwire of a vb run: "
                          "wait for that run to end, or if a run was killed, check that nothing else changed the "
                          f"file (`ls -lc` shows its ctime) and chmod {REST_MODE:o} it")


def _unquote(value: str) -> str:
    return value[1:-1] if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"" else value


def _has_space(value: str) -> bool:
    return any(char.isspace() for char in value)


def _check_location(path: Path, what: str = "secret file") -> None:
    """Refuse a secret or key file inside the repository, where agents may read, or in `~/.roko`, which roko reads."""
    for root, name in ((layout.REPO_ROOT, "the repository"), (_roko_home(), "~/.roko")):
        if layout.within(path, root):
            raise SecretError(f"the {what} must live outside {name}, not at {path}")


def _roko_home() -> Path:
    """`$HOME/.roko`, the directory whose `.env` roko loads at startup."""
    return Path("~/.roko").expanduser().absolute()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="secret.py", description=__doc__.split("\n\n")[0], allow_abbrev=False)
    parser.add_argument("command", choices=("init", "check", "keys"))
    parser.add_argument("--secret-file", type=Path, metavar="PATH",
                        help=f"default: ${FILE_ENV}, then {DEFAULT_PATH}")
    parser.add_argument("--key-file", type=Path, metavar="PATH",
                        help=f"for `keys`; default: ${KEYS_FILE_ENV}, then {KEYS_DEFAULT_PATH}")
    args = parser.parse_args(argv)
    if args.command == "keys":
        return _check_keys(resolve_keys(args.key_file))
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


def _check_keys(path: Path) -> int:
    """`keys`: the key file's rules and the names it holds (never a key), and any key the environment exposes."""
    try:
        keys = load_keys(path)
    except SecretError as err:
        print(f"secret.py: {err}", file=sys.stderr)
        return 1
    problems = key_exposures(keys)
    for problem in problems:
        print(f"secret.py: {problem}", file=sys.stderr)
    unused = sorted(set(keys) - arm_key_names())
    if unused:
        print(f"secret.py: no arm file names {', '.join(unused)}", file=sys.stderr)
    mode = stat.S_IMODE(path.stat().st_mode)
    print(f"{path} keys={','.join(sorted(keys)) or '-'} mode={mode:04o}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
