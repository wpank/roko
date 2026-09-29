#!/usr/bin/env python3
"""Leak check: no canary GUID and no benchmark secret in a tree an agent works in (S08 SC4, §4.2 (4); gap-7ee7c2).

A canary in an agent's workdir means benchmark source reached it: a generator did not strip a marker line, or a
template or solution carried one where stripping cannot reach. The secret in a workdir would let the agent compute
the hidden cases. Either makes every verdict on that tree worthless. `scan` looks for any string of canary form
(`common.canary`, which also matches older releases' canaries and the canary line of the secret file) and for the
secret's exact bytes, in every file's name, content and symlink target. Like `canary.find_in_tree`, it follows no
link and skips `.git`.

verify_verifiers.py scans every materialized workdir and every solved tree. Transcripts and diffs come later, when
the driver's census uses the same scan (gap-a8a160).

Usage:
    leak_check.py --secret-file PATH DIR [DIR ...]

It prints {DIR: {relative path: [hits]}} for the directories with a hit, where a hit is a canary GUID or the words
"the secret" (never the secret itself). It exits 1 on any hit, 0 when every directory is clean, and 2 when it cannot
run, for example because group or others can read the secret file.

API:
    scan(root: Path, secret: bytes | None = None) -> dict[str, list[str]]   # relative path -> hits; {} when clean
    secret_bytes(secret: hmac_seed.Secret) -> bytes
    SECRET_HIT: str
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "families"))
from common import canary, hmac_seed  # noqa: E402

SECRET_HIT = "the secret"
CHUNK = 1 << 20


def scan(root: Path, secret: bytes | None = None) -> dict[str, list[str]]:
    """Relative path -> what leaked into that file's name, content or symlink target: canaries, then SECRET_HIT."""
    root = Path(root)
    if not root.is_dir() or root.is_symlink():
        raise NotADirectoryError(f"{root} is not a directory")
    hits = {path: list(found) for path, found in canary.find_in_tree(root).items()}
    if secret:
        for path in _secret_paths(root, secret):
            hits.setdefault(path, []).append(SECRET_HIT)
    return dict(sorted(hits.items()))


def secret_bytes(secret: hmac_seed.Secret) -> bytes:
    """The raw secret. The scan has to know it to find it; nothing here prints or stores it."""
    return secret._value  # noqa: SLF001


def _secret_paths(root: Path, secret: bytes) -> list[str]:
    found = []
    for directory, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if name.lower() != ".git")
        for name in sorted(filenames) + [name for name in dirnames if os.path.islink(os.path.join(directory, name))]:
            path = Path(directory) / name
            relpath = path.relative_to(root).as_posix()
            if path.is_symlink():
                leaked = secret in os.fsencode(os.readlink(path))
            else:
                leaked = path.is_file() and _file_contains(path, secret)
            if leaked or secret in os.fsencode(relpath):
                found.append(relpath)
    return found


def _file_contains(path: Path, needle: bytes) -> bool:
    """Whether a file of any size holds `needle`, read in chunks that overlap by one needle length."""
    keep = len(needle) - 1
    tail = b""
    with open(path, "rb") as handle:
        while chunk := handle.read(CHUNK):
            window = tail + chunk
            if needle in window:
                return True
            tail = window[-keep:] if keep else b""
    return False


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    hmac_seed.add_secret_file_argument(parser)
    parser.add_argument("dirs", type=Path, nargs="+", metavar="DIR", help="a tree to scan, such as a workdir")
    args = parser.parse_args(argv)
    try:
        secret = secret_bytes(hmac_seed.read_secret_file(args.secret_file))
        report = {str(directory): hits for directory in args.dirs if (hits := scan(directory, secret))}
    except (hmac_seed.SecretFileError, OSError) as err:
        print(f"leak_check.py: {err}", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=1, sort_keys=True))
    return 1 if report else 0


if __name__ == "__main__":
    sys.exit(main())
