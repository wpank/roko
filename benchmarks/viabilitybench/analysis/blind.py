"""Blinding: arm labels hashed with a secret salt until the lock allows unblinding (S09 §4.1 "Blinding", E4; task
3341).

The analysis scripts are written and checked on arm-hashed data, and unblinded only after the pre-registration lock
(§4.1). A `Blinder` holds a salt, 32 random bytes in a 0600 file outside the repository (`new_salt`; by default
`~/.config/viabilitybench/blind-salt`, beside the benchmark secret), and labels each arm id "arm-" + the first 12 hex
digits of HMAC-SHA256(salt, arm id): the same label for one arm under one salt, and nothing about the arm without the
salt. `replay.load(blind=blinder.label)` reads a campaign blinded.

Unblinding needs the lock: `unblind` and `key` map labels back to the arm ids they hash from only once `lock.require`
accepts the lock (it exists, is committed, and checks clean); before that they raise BlindError. A label maps back by
hashing the candidate arm ids (S09 §4.2's registry, `ARMS`, unless the caller names others), since HMAC cannot be
inverted.

API:
    ARMS, DEFAULT_SALT, PREFIX
    new_salt(path=DEFAULT_SALT) -> Path               # refuses to overwrite
    Blinder(salt); Blinder.from_file(path=DEFAULT_SALT) -> Blinder
    .label(arm) -> str
    .key(arms=ARMS, *, lock_path, spec) -> dict[label, arm]       # raises BlindError before the lock
    .unblind(label, arms=ARMS, *, lock_path, spec) -> str         # raises BlindError before the lock
    BlindError
"""

from __future__ import annotations

import hashlib
import hmac
import os
import secrets
import stat
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

import lock

SALT_BYTES = 32
DEFAULT_SALT = Path("~/.config/viabilitybench/blind-salt")
PREFIX = "arm-"
# S09 §4.2's arm registry: the only arm names (v1.2 adds roko_plan; 3302 roko_ladder).
ARMS = ("cheap_direct", "roko_fixed", "roko_full", "fd_claude", "fr_claude", "fd_claude_lite", "fd_codex", "fd_api",
        "hybrid", "roko_plan", "roko_ladder")


class BlindError(RuntimeError):
    """A blinding salt cannot be made or read, or unblinding was asked for before the lock allows it."""


def new_salt(path: Path = DEFAULT_SALT) -> Path:
    """A fresh salt file, mode 0600, its directory 0700; never over an existing one."""
    path = Path(path).expanduser()
    if path.exists() or path.is_symlink():
        raise BlindError(f"{path} exists; a salt is never replaced, or every blinded label would change")
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w", encoding="ascii") as handle:
        handle.write(secrets.token_hex(SALT_BYTES) + "\n")
    return path


@dataclass(frozen=True)
class Blinder:
    salt: bytes

    def __post_init__(self) -> None:
        if len(self.salt) < 16:
            raise BlindError("a blinding salt has at least 16 bytes")

    def __repr__(self) -> str:  # never print the salt
        return f"Blinder(salt=<{len(self.salt)} bytes>)"

    @classmethod
    def from_file(cls, path: Path = DEFAULT_SALT) -> Blinder:
        """The salt in `path`, which must be a regular file readable by its owner alone."""
        path = Path(path).expanduser()
        try:
            info = path.lstat()
        except OSError as err:
            raise BlindError(f"no blinding salt at {path}: {err}") from None
        if not stat.S_ISREG(info.st_mode) or info.st_mode & (stat.S_IRWXG | stat.S_IRWXO):
            raise BlindError(f"{path} must be a regular file of mode 0600 (no group or other access)")
        try:
            return cls(bytes.fromhex(path.read_text(encoding="ascii").strip()))
        except ValueError:
            raise BlindError(f"{path} does not hold a hex salt") from None

    def label(self, arm: str) -> str:
        """The blinded label of `arm`."""
        return PREFIX + hmac.new(self.salt, arm.encode("utf-8"), hashlib.sha256).hexdigest()[:12]

    def key(self, arms: Iterable[str] = ARMS, *, lock_path: Path = lock.DEFAULT_LOCK,
            spec: Path = lock.DEFAULT_SPEC) -> dict[str, str]:
        """label -> arm for `arms`, once the lock allows unblinding (module docstring)."""
        try:
            lock.require(lock_path, spec)
        except lock.LockError as err:
            raise BlindError(f"unblinding waits for the pre-registration lock: {err}") from None
        found = {self.label(arm): arm for arm in arms}
        if len(found) != len(set(arms)):
            raise BlindError("two arms share a label under this salt")  # a 48-bit collision: make a new salt
        return found

    def unblind(self, label: str, arms: Iterable[str] = ARMS, *, lock_path: Path = lock.DEFAULT_LOCK,
                spec: Path = lock.DEFAULT_SPEC) -> str:
        """The arm `label` hashes from, once the lock allows unblinding."""
        found = self.key(arms, lock_path=lock_path, spec=spec)
        if label not in found:
            raise BlindError(f"{label} is the label of none of the arms {', '.join(sorted(set(arms)))}")
        return found[label]
