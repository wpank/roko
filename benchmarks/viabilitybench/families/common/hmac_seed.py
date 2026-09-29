"""Deterministic per-instance random streams, and the benchmark secret that keys the hidden ones.

Two kinds of stream, both HMAC-SHA256 in counter mode, so the same inputs give the same bytes on every platform
and every Python version (`random.Random` promises that only for `random()`):

- `surface_stream(family, instance_id)` is public. `gen.py` draws the visible instance from it: names, data,
  module layout and knob values. Anyone can regenerate an instance from its id, which is fine: the agent sees it.
- `hidden_stream(secret, family, instance_id)` is keyed by the benchmark secret. `hidden.py` draws its hidden cases
  from it at audit time, so they are never on disk while an agent runs. The generators are tracked in git, where
  an agent may read them, but without the secret it cannot compute the cases.

Both are HMAC-SHA256(key, fields), with the fields length-prefixed so no two inputs collide: the key is the
secret, or the public constant `SURFACE_KEY`; the fields are a domain tag, the family and the instance id.
`Stream.child(label)` derives an independent sub-stream, e.g. one per check.

**The secret arrives only as a file path** (`--secret-file`), never on a command line, where concurrent agents can
read it with `ps`, and never in the environment, which agents inherit. This departs from S08 §5.2's
`--secret $VB_SECRET`. Where the real file lives, and how the driver keeps it from agents, is gap-a8a160's. The
file is UTF-8 text: blank lines and lines starting with `#` are ignored (so the file can carry a canary line of its
own), and exactly one line remains, the secret, at least `MIN_SECRET_CHARS` characters long. The file must be a
regular file that neither group nor others can access (mode 0600, like an SSH key).

API:
    read_secret_file(path: Path) -> Secret                    # raises SecretFileError
    write_secret_file(path: Path, value: str | None = None) -> Path   # a new 0600 file; tests and setup only
    add_secret_file_argument(parser: argparse.ArgumentParser) -> None  # the one CLI spelling: --secret-file PATH
    hidden_stream(secret: Secret, family: str, instance_id: str) -> Stream
    surface_stream(family: str, instance_id: str) -> Stream
    Stream: bytes(n), randbelow(n), randint(a, b), random(), choice(seq), sample(seq, k), shuffled(seq),
            token_hex(nbytes), child(label)
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import os
import secrets
import stat
from collections.abc import Sequence
from pathlib import Path
from typing import TypeVar

T = TypeVar("T")

MIN_SECRET_CHARS = 32
SURFACE_KEY = b"vb.surface/1"
HIDDEN_TAG = "vb.hidden/1"
SURFACE_TAG = "vb.surface/1"


class SecretFileError(ValueError):
    """The secret file is missing, readable by others, or not in the expected format."""


class Secret:
    """The benchmark secret. Its repr never shows the value, so it cannot leak through a log or a traceback."""

    __slots__ = ("_value",)

    def __init__(self, value: bytes) -> None:
        self._value = value

    @property
    def fingerprint(self) -> str:
        """A short, one-way id of the secret, safe to record next to verdicts it produced."""
        return "sha256:" + hashlib.sha256(b"vb.secret-fingerprint/1\x00" + self._value).hexdigest()[:16]

    def __repr__(self) -> str:
        return f"Secret(<redacted> {self.fingerprint})"


class Stream:
    """A deterministic random stream: HMAC-SHA256 of a 32-byte seed over a block counter.

    Every method consumes the stream in a fixed, documented way, so a draw sequence reproduces exactly.
    """

    __slots__ = ("_seed", "_counter", "_buffer")

    def __init__(self, seed: bytes) -> None:
        if len(seed) != 32:
            raise ValueError("a stream seed is 32 bytes")
        self._seed = seed
        self._counter = 0
        self._buffer = b""

    def bytes(self, n: int) -> bytes:
        """The next `n` bytes."""
        while len(self._buffer) < n:
            block = hmac.digest(self._seed, b"block\x00" + self._counter.to_bytes(8, "big"), "sha256")
            self._buffer += block
            self._counter += 1
        out, self._buffer = self._buffer[:n], self._buffer[n:]
        return out

    def randbelow(self, n: int) -> int:
        """A uniform integer in [0, n), by rejection sampling over the fewest whole bytes that hold n - 1."""
        if n < 1:
            raise ValueError("randbelow needs n >= 1")
        bits = n.bit_length()
        width = (bits + 7) // 8
        while True:
            value = int.from_bytes(self.bytes(width), "big") >> (width * 8 - bits)
            if value < n:
                return value

    def randint(self, a: int, b: int) -> int:
        """A uniform integer in [a, b], both ends included."""
        if b < a:
            raise ValueError(f"empty range [{a}, {b}]")
        return a + self.randbelow(b - a + 1)

    def random(self) -> float:
        """A uniform float in [0, 1) with 53 random bits."""
        return (int.from_bytes(self.bytes(7), "big") >> 3) / (1 << 53)

    def choice(self, seq: Sequence[T]) -> T:
        """One element of a non-empty sequence."""
        if not seq:
            raise IndexError("cannot choose from an empty sequence")
        return seq[self.randbelow(len(seq))]

    def sample(self, seq: Sequence[T], k: int) -> list[T]:
        """`k` distinct positions of `seq`, in draw order (a partial Fisher-Yates shuffle)."""
        items = list(seq)
        if not 0 <= k <= len(items):
            raise ValueError(f"cannot sample {k} of {len(items)} items")
        for i in range(k):
            j = i + self.randbelow(len(items) - i)
            items[i], items[j] = items[j], items[i]
        return items[:k]

    def shuffled(self, seq: Sequence[T]) -> list[T]:
        """A shuffled copy of `seq`."""
        return self.sample(seq, len(seq))

    def token_hex(self, nbytes: int) -> str:
        """`nbytes` bytes as lowercase hex."""
        return self.bytes(nbytes).hex()

    def child(self, label: str) -> Stream:
        """An independent stream named `label`; it does not advance this one."""
        return Stream(_derive(self._seed, "child", label))

    def __repr__(self) -> str:
        return "Stream(<seeded>)"


def read_secret_file(path: Path) -> Secret:
    """The secret in `path`. Raises SecretFileError on any problem, and never includes the secret in the error."""
    path = Path(path)
    try:
        info = path.lstat()
    except OSError as err:
        raise SecretFileError(f"cannot read the secret file {path}: {err.strerror}") from None
    if not stat.S_ISREG(info.st_mode):
        raise SecretFileError(f"the secret file {path} must be a regular file, not a link or a directory")
    if info.st_mode & 0o077:
        raise SecretFileError(f"the secret file {path} is accessible to group or others; run: chmod 600 {path}")
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as err:
        raise SecretFileError(f"cannot read the secret file {path}: {type(err).__name__}") from None
    lines = [line.strip() for line in text.splitlines()]
    values = [line for line in lines if line and not line.startswith("#")]
    if len(values) != 1:
        raise SecretFileError(f"the secret file {path} must hold exactly one secret line, not {len(values)}")
    if len(values[0]) < MIN_SECRET_CHARS:
        raise SecretFileError(f"the secret in {path} is shorter than {MIN_SECRET_CHARS} characters")
    return Secret(values[0].encode("utf-8"))


def write_secret_file(path: Path, value: str | None = None) -> Path:
    """Create `path` (it must not exist) with mode 0600, holding `value` or a fresh 256-bit secret.

    For tests, which use throwaway secrets, and for first-time setup. Parent directories it creates get mode 0700.
    """
    path = Path(path)
    value = secrets.token_hex(32) if value is None else value
    if len(value) < MIN_SECRET_CHARS or "\n" in value or value.strip() != value or value.startswith("#"):
        raise ValueError(f"a secret is one line of at least {MIN_SECRET_CHARS} characters, not starting with #")
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as handle:
        handle.write("# ViabilityBench secret: keys the hidden test cases. Never pass it on argv or in the env.\n")
        handle.write(value + "\n")
    os.chmod(path, 0o600)
    return path


def add_secret_file_argument(parser: argparse.ArgumentParser) -> None:
    """Add the required `--secret-file PATH` option; read it with `read_secret_file(args.secret_file)`.

    It also turns off the parser's option abbreviations: otherwise `--secret VALUE` would be accepted as an
    abbreviation of `--secret-file`, with the secret on the command line.
    """
    parser.allow_abbrev = False
    parser.add_argument("--secret-file", type=Path, required=True, metavar="PATH",
                        help="file holding the benchmark secret (mode 0600); the secret itself is never an argument")


def hidden_stream(secret: Secret, family: str, instance_id: str) -> Stream:
    """The stream hidden cases are drawn from: HMAC-SHA256(secret, (tag, family, instance_id))."""
    if not isinstance(secret, Secret):
        raise TypeError("hidden_stream takes the Secret from read_secret_file, never a raw value")
    return Stream(_derive(secret._value, HIDDEN_TAG, family, instance_id))


def surface_stream(family: str, instance_id: str) -> Stream:
    """The public stream the visible instance is drawn from; it does not depend on the secret."""
    return Stream(_derive(SURFACE_KEY, SURFACE_TAG, family, instance_id))


def _derive(key: bytes, *fields: str) -> bytes:
    message = b"".join(len(data).to_bytes(4, "big") + data for data in (field.encode("utf-8") for field in fields))
    return hmac.digest(key, message, "sha256")
