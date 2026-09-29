"""Surface renames, so the instances of one family differ while the latent structure stays the same (B §3.1 (2)).

A family writes its template with real, working names (`billing`, `Invoice`, `issue_refund`) and lists, for each
name it wants varied, a pool of alternatives. `choose` draws one distinct alternative per name from the instance's
surface stream, and `rename_tree` applies the mapping to file contents and to file and directory names.

Renames act on whole identifiers only: `refund` does not touch `refund_total` or `Refund`, so a family maps every
spelling it uses (e.g. both `invoice` and `Invoice`). All renames happen in one pass, so a mapping may swap two
names. A new name that already occurs in the tree, and is not itself renamed away, is refused, because it would
silently merge two identifiers.

API:
    choose(stream: Stream, pools: Mapping[str, Sequence[str]]) -> dict[str, str]
    rename_text(text: str, mapping: Mapping[str, str]) -> str
    rename_tree(root: Path, mapping: Mapping[str, str]) -> list[str]    # the changed files, by their new paths
"""

from __future__ import annotations

import os
import re
from collections.abc import Mapping, Sequence
from pathlib import Path

from .hmac_seed import Stream

IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def choose(stream: Stream, pools: Mapping[str, Sequence[str]]) -> dict[str, str]:
    """For each name, in sorted order, one alternative from its pool that no earlier name has taken."""
    mapping: dict[str, str] = {}
    for name in sorted(pools):
        options = [option for option in pools[name] if option not in mapping.values()]
        if not options:
            raise ValueError(f"no unused alternative left for {name!r}")
        mapping[name] = stream.choice(options)
    _check(mapping)
    return mapping


def rename_text(text: str, mapping: Mapping[str, str]) -> str:
    """`text` with every whole-identifier occurrence of a key replaced by its value, in one pass."""
    _check(mapping)
    pattern = _pattern(mapping)
    if pattern is None:
        return text
    taken = [new for new in mapping.values() if new not in mapping and re.search(_word(new), text)]
    if taken:
        raise ValueError(f"the new name {taken[0]!r} already occurs; renaming would merge two identifiers")
    return pattern.sub(lambda match: mapping[match[0]], text)


def rename_tree(root: Path, mapping: Mapping[str, str]) -> list[str]:
    """Apply `mapping` to every UTF-8 text file's content and to every file and directory name under `root`.

    Binary files keep their content, but are renamed like any other. Symlinks are renamed, never followed; `.git`
    is skipped. Returns the relative paths, after renaming, of the files whose content or path changed.
    """
    root = Path(root)
    _check(mapping)
    files = _files(root)
    texts = {}
    for file in files:
        data = file.read_bytes() if not file.is_symlink() else None
        if data is not None and b"\0" not in data:
            try:
                texts[file] = data.decode("utf-8")
            except UnicodeDecodeError:
                pass
    for text in texts.values():  # check every file before changing any
        rename_text(text, mapping)
    changed = set()
    for file, text in texts.items():
        renamed = rename_text(text, mapping)
        if renamed != text:
            file.write_bytes(renamed.encode("utf-8"))
            changed.add(file)
    moves = {}
    for directory, dirnames, filenames in os.walk(root, topdown=False):
        for name in filenames + dirnames:
            if any(part.lower() == ".git" for part in Path(directory, name).relative_to(root).parts):
                continue
            new_name = rename_text(name, mapping)
            if new_name != name:
                source, target = Path(directory, name), Path(directory, new_name)
                if os.path.lexists(target):
                    raise ValueError(f"cannot rename {source.relative_to(root)}: {new_name} already exists")
                source.rename(target)
                moves[source] = target
    result = set()
    for file in changed | {path for path in files if _moved(path, moves, root) != path}:
        result.add(_moved(file, moves, root).relative_to(root).as_posix())
    return sorted(result)


def _check(mapping: Mapping[str, str]) -> None:
    for old, new in mapping.items():
        if not (IDENTIFIER.fullmatch(old) and IDENTIFIER.fullmatch(new)):
            raise ValueError(f"renames map identifiers to identifiers, not {old!r} -> {new!r}")
    if len(set(mapping.values())) != len(mapping):
        raise ValueError("two names cannot be renamed to the same identifier")


def _pattern(mapping: Mapping[str, str]) -> re.Pattern[str] | None:
    if not mapping:
        return None
    names = sorted(mapping, key=len, reverse=True)
    return re.compile(r"(?<![A-Za-z0-9_])(?:" + "|".join(map(re.escape, names)) + r")(?![A-Za-z0-9_])")


def _word(name: str) -> str:
    return r"(?<![A-Za-z0-9_])" + re.escape(name) + r"(?![A-Za-z0-9_])"


def _files(root: Path) -> list[Path]:
    files = []
    for directory, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if name.lower() != ".git")
        files.extend(Path(directory, name) for name in sorted(filenames))
        files.extend(Path(directory, name) for name in dirnames if Path(directory, name).is_symlink())
    return files


def _moved(path: Path, moves: Mapping[Path, Path], root: Path) -> Path:
    """Where `path` is after the renames. `moves` maps an original path to its new name in its original parent."""
    original = current = root
    for part in path.relative_to(root).parts:
        original = original / part
        target = moves.get(original)
        current = current / (target.name if target else part)
    return current
