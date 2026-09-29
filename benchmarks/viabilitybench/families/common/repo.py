"""Task repos with a pristine snapshot of their base commit, and clean exports for the census (S08 §4.2, §4.10).

The lifecycle of one task:

1. `gen.py` renders an instance into DIR. `init_task_repo(DIR, bundle)` makes DIR a git repo with one commit that
   holds every file, and saves that commit to `bundle`, a git bundle outside DIR where the agent cannot reach it.
   The commit has a fixed identity and date, so the same files always give the same commit. The returned
   `Pristine` (bundle path, commit, tree hash) is what the driver records.
2. The agent works in DIR.
3. For the census, `export_tree(DIR, dest)` copies the files as they are on disk, or `export_tree(repo, dest,
   rev=c_i)` copies a commit's tree, into `dest`: a new directory that is a fresh git repo with one commit. The
   census works there, never in the agent's repo.
4. `restore_paths(dest, pristine, paths)` makes each path, a file or a directory, identical to the pristine base,
   and removes whatever the agent added under it. With `paths=None` it restores the whole tree.
5. `tree_hash(dir)` hashes a directory's files as they are on disk. After a full restore it equals `pristine.tree`.

What counts as the tree, everywhere in this module: every regular file and symlink, byte for byte (no line-ending
or attribute filters), with git's modes (100755 when the owner may execute the file, else 100644; 120000 for a
symlink, which is never followed). Paths with a `.git` component (in any letter case), empty directories and
special files (fifos, sockets, devices) do not count. `tree_hash` computes git's SHA-1 tree id in Python, so it
needs no repository and agrees with `git write-tree`. Submodule entries (gitlinks) have no content and are left out
of an export.

Safety. The agent controls its repo's `.git`: hooks, config (fsmonitor, filters) and attributes. Nothing here runs
porcelain in it. `export_tree(rev=…)` reads the commit with plumbing only (`rev-parse`, `ls-tree`, `cat-file`);
every git call ignores global and system config and runs with hooks disabled; and writes never follow a symlink out
of the target directory: a symlink or file the agent put where a pristine directory belongs is replaced by a real
directory.

API:
    init_task_repo(workdir: Path, bundle: Path) -> Pristine
    Pristine(bundle: Path, commit: str, tree: str); .as_json() -> dict[str, str]; Pristine.from_json(doc) -> Pristine
    export_tree(source: Path, dest: Path, *, rev: str | None = None) -> str      # the exported tree hash
    restore_paths(workdir: Path, pristine: Pristine, paths: Sequence[str] | None = None) -> None
    tree_hash(path: Path) -> str
"""

from __future__ import annotations

import hashlib
import os
import re
import shutil
import stat
import subprocess
import tempfile
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

BRANCH = "main"
PRISTINE_REF = "refs/vb/pristine"
SYMLINK = "120000"
HEX40 = re.compile(r"[0-9a-f]{40}")
GIT_TIMEOUT_S = 120.0
# One identity and date for every commit made here, so equal trees give equal commits.
GIT_IDENTITY = {
    "GIT_AUTHOR_NAME": "ViabilityBench", "GIT_AUTHOR_EMAIL": "vb@localhost",
    "GIT_COMMITTER_NAME": "ViabilityBench", "GIT_COMMITTER_EMAIL": "vb@localhost",
    "GIT_AUTHOR_DATE": "2026-09-28T00:00:00+00:00", "GIT_COMMITTER_DATE": "2026-09-28T00:00:00+00:00",
}


class RepoError(RuntimeError):
    """A git step failed, a path is unsafe, or a snapshot does not match its record."""


@dataclass(frozen=True)
class Pristine:
    """The pristine base of a task repo: the bundle that holds it, its commit and its tree hash."""

    bundle: Path
    commit: str
    tree: str

    def as_json(self) -> dict[str, str]:
        return {"bundle": str(self.bundle), "commit": self.commit, "tree": self.tree}

    @classmethod
    def from_json(cls, doc: dict) -> Pristine:
        if not (HEX40.fullmatch(str(doc.get("commit"))) and HEX40.fullmatch(str(doc.get("tree")))):
            raise RepoError("a pristine record needs a 40-hex commit and tree")
        return cls(bundle=Path(doc["bundle"]), commit=doc["commit"], tree=doc["tree"])


@dataclass(frozen=True)
class _Entry:
    path: str  # relative, "/"-separated
    mode: str  # 100644, 100755 or 120000
    data: bytes  # file content, or a symlink's target


def init_task_repo(workdir: Path, bundle: Path) -> Pristine:
    """Commit every file in `workdir` as the pristine base, and save that commit to `bundle` (a new file)."""
    workdir = Path(workdir).resolve()
    bundle = Path(bundle).absolute()
    if not workdir.is_dir():
        raise RepoError(f"{workdir} is not a directory")
    if os.path.lexists(workdir / ".git"):
        raise RepoError(f"{workdir} already has a .git")
    if _within(bundle.parent.resolve() / bundle.name, workdir):
        raise RepoError("the bundle must live outside the workdir, where the agent cannot reach it")
    if os.path.lexists(bundle):
        raise RepoError(f"refusing to overwrite {bundle}")
    _git(workdir, "init", "-q", "-b", BRANCH)
    commit, tree = _commit_disk(workdir, "vb: pristine base")
    bundle.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    _git(workdir, "update-ref", PRISTINE_REF, commit)
    _git(workdir, "bundle", "create", str(bundle), PRISTINE_REF)
    _git(workdir, "update-ref", "-d", PRISTINE_REF)
    return Pristine(bundle=bundle, commit=commit, tree=tree)


def export_tree(source: Path, dest: Path, *, rev: str | None = None) -> str:
    """Copy `source`'s files on disk (rev=None), or the tree of commit `rev` in repo `source`, into `dest`.

    `dest` must not exist. It becomes a fresh git repo whose one commit holds exactly the exported files; the
    return value is that commit's tree hash, which equals `tree_hash(dest)`.
    """
    source = Path(source).resolve()
    dest = Path(dest).absolute()
    if os.path.lexists(dest):
        raise RepoError(f"refusing to overwrite {dest}")
    if _within(dest.parent.resolve() / dest.name, source) and rev is None:
        raise RepoError("cannot export a directory into itself")
    entries = _scan(source) if rev is None else _read_commit(source, rev)
    dest.mkdir(mode=0o700, parents=True)
    _git(dest, "init", "-q", "-b", BRANCH)
    _write_entries(dest, entries)
    _, tree = _commit_disk(dest, "vb: export of " + ("the working tree" if rev is None else rev))
    return tree


def restore_paths(workdir: Path, pristine: Pristine, paths: Sequence[str] | None = None) -> None:
    """Make each of `paths` in `workdir` identical to the pristine base; `None` restores the whole tree.

    Run it on an export, never in the agent's own repo. A path that the base lacks raises RepoError.
    """
    workdir = Path(workdir).resolve()
    wanted = None if paths is None else [_safe_relpath(raw) for raw in paths]
    entries = _pristine_entries(pristine)
    if wanted is None:
        for child in workdir.iterdir():
            if child.name.lower() != ".git":
                _remove(child)
        _write_entries(workdir, entries)
        return
    chosen = {path: [entry for entry in entries if entry.path == path or entry.path.startswith(path + "/")]
              for path in wanted}
    missing = [path for path, selected in chosen.items() if not selected]
    if missing:
        raise RepoError(f"{missing[0]!r} is not in the pristine base")
    for path, selected in chosen.items():
        target = _real_dir(workdir, PurePosixPath(path).parent) / PurePosixPath(path).name
        _remove(target)
        _write_entries(workdir, selected)


def tree_hash(path: Path) -> str:
    """Git's tree id for the files under `path` as they are on disk (see the module docstring for the rules)."""
    path = Path(path)
    if not path.is_dir():
        raise RepoError(f"{path} is not a directory")
    return _tree_id(_scan(path))


def _pristine_entries(pristine: Pristine) -> list[_Entry]:
    if not Path(pristine.bundle).is_file():
        raise RepoError(f"the pristine bundle {pristine.bundle} is missing")
    with tempfile.TemporaryDirectory(prefix="vb-pristine-") as tmp:
        store = Path(tmp)
        _git(store, "init", "-q", "--bare")
        _git(store, "bundle", "unbundle", str(Path(pristine.bundle).absolute()))
        tree = _git(store, "rev-parse", "--verify", "-q", f"{pristine.commit}^{{tree}}").decode().strip()
        if tree != pristine.tree:
            raise RepoError(f"the bundle {pristine.bundle} does not hold the recorded pristine commit and tree")
        return _read_commit(store, pristine.commit)


def _read_commit(repo: Path, rev: str) -> list[_Entry]:
    """The blobs and symlinks of `rev`'s tree, read with plumbing only."""
    if not rev or rev.startswith("-"):
        raise RepoError(f"not a revision: {rev!r}")
    tree = _git(repo, "rev-parse", "--verify", "-q", f"{rev}^{{tree}}").decode().strip()
    listing = []
    for record in _git(repo, "ls-tree", "-r", "-z", "--full-tree", tree).split(b"\0"):
        if not record:
            continue
        meta, _, raw_path = record.partition(b"\t")
        mode, kind, sha = meta.decode().split()
        if kind == "blob":  # gitlinks ("commit") have no content here
            path = _safe_relpath(os.fsdecode(raw_path))
            listing.append((path, SYMLINK if mode == SYMLINK else ("100755" if mode == "100755" else "100644"), sha))
    contents = _cat_blobs(repo, sorted({sha for _, _, sha in listing}))
    return [_Entry(path, mode, contents[sha]) for path, mode, sha in listing]


def _cat_blobs(repo: Path, shas: list[str]) -> dict[str, bytes]:
    if not shas:
        return {}
    out = _git(repo, "cat-file", "--batch", input="".join(sha + "\n" for sha in shas).encode())
    contents, position = {}, 0
    for sha in shas:
        end = out.index(b"\n", position)
        header = out[position:end].decode().split()
        if len(header) != 3 or header[0] != sha or header[1] != "blob":
            raise RepoError(f"cat-file could not read blob {sha}: {' '.join(header)}")
        size = int(header[2])
        contents[sha] = out[end + 1:end + 1 + size]
        position = end + 1 + size + 1
    return contents


def _scan(root: Path) -> list[_Entry]:
    entries: list[_Entry] = []

    def walk(directory: str, prefix: str) -> None:
        with os.scandir(directory) as listing:
            children = sorted(listing, key=lambda child: child.name)
        for child in children:
            if child.name.lower() == ".git":
                continue
            relpath = prefix + child.name
            if child.is_symlink():
                entries.append(_Entry(relpath, SYMLINK, os.fsencode(os.readlink(child.path))))
            elif child.is_dir(follow_symlinks=False):
                walk(child.path, relpath + "/")
            elif child.is_file(follow_symlinks=False):
                executable = child.stat(follow_symlinks=False).st_mode & stat.S_IXUSR
                with open(child.path, "rb") as handle:
                    entries.append(_Entry(relpath, "100755" if executable else "100644", handle.read()))

    walk(str(root), "")
    return entries


def _write_entries(root: Path, entries: list[_Entry]) -> None:
    for entry in entries:
        path = PurePosixPath(_safe_relpath(entry.path))
        target = _real_dir(root, path.parent) / path.name
        _remove(target)
        if entry.mode == SYMLINK:
            os.symlink(os.fsdecode(entry.data), target)
            continue
        fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "wb") as handle:
            handle.write(entry.data)
        os.chmod(target, 0o755 if entry.mode == "100755" else 0o644)


def _real_dir(root: Path, relpath: PurePosixPath) -> Path:
    """root/relpath as a real directory, replacing any symlink or file on the way instead of following it."""
    current = root
    for part in relpath.parts:
        current = current / part
        if current.is_symlink() or (current.exists() and not current.is_dir()):
            current.unlink()
        if not current.exists():
            current.mkdir()
    return current


def _remove(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)
    elif os.path.lexists(path):
        path.unlink()


def _commit_disk(root: Path, message: str) -> tuple[str, str]:
    """Commit the files under `root` (its own fresh repo) exactly as `_scan` sees them; (commit, tree)."""
    entries = _scan(root)
    written: dict[str, str] = {}
    # --stdin-paths reads one path per line, so a name with a line break goes through --stdin instead.
    plain = [entry for entry in entries if entry.mode != SYMLINK and not {"\n", "\r"} & set(entry.path)]
    if plain:
        out = _git(root, "hash-object", "-w", "--no-filters", "--stdin-paths",
                   input=b"".join(os.fsencode(entry.path) + b"\n" for entry in plain))
        written.update(zip((entry.path for entry in plain), out.decode().split()))
    for entry in entries:
        if entry.path not in written:
            written[entry.path] = _git(root, "hash-object", "-w", "--no-filters", "--stdin",
                                       input=entry.data).decode().strip()
        if written[entry.path] != _object_id(b"blob", entry.data).hex():
            raise RepoError(f"{entry.path} changed while it was being committed")
    index = b"".join(f"{entry.mode} {written[entry.path]}\t".encode() + os.fsencode(entry.path) + b"\0"
                     for entry in entries)
    _git(root, "update-index", "-z", "--index-info", input=index)
    tree = _git(root, "write-tree").decode().strip()
    if tree != _tree_id(entries):
        raise RepoError(f"git's tree {tree} differs from the computed tree hash; this is a bug")
    commit = _git(root, "commit-tree", tree, "-m", message).decode().strip()
    _git(root, "update-ref", f"refs/heads/{BRANCH}", commit)
    return commit, tree


def _tree_id(entries: list[_Entry]) -> str:
    root: dict = {}
    for entry in entries:
        *parents, name = entry.path.split("/")
        node = root
        for part in parents:
            node = node.setdefault(part, {})
        node[name] = entry
    return _hash_tree(root).hex()


def _hash_tree(node: dict) -> bytes:
    items = []
    for name, child in node.items():
        raw = os.fsencode(name)
        if isinstance(child, dict):  # git sorts a subtree as if its name ended in "/"
            items.append((raw + b"/", b"40000", raw, _hash_tree(child)))
        else:
            items.append((raw, child.mode.encode(), raw, _object_id(b"blob", child.data)))
    items.sort(key=lambda item: item[0])
    return _object_id(b"tree", b"".join(mode + b" " + raw + b"\0" + sha for _, mode, raw, sha in items))


def _object_id(kind: bytes, data: bytes) -> bytes:
    return hashlib.sha1(kind + b" " + str(len(data)).encode() + b"\0" + data, usedforsecurity=False).digest()


def _safe_relpath(raw: str) -> str:
    path = PurePosixPath(raw)
    parts = path.parts
    if path.is_absolute() or not parts or any(part in (".", "..") or part.lower() == ".git" for part in parts):
        raise RepoError(f"unsafe or empty path: {raw!r}")
    return "/".join(parts)


def _within(path: Path, root: Path) -> bool:
    return path == root or root in path.parents


def _git(cwd: Path, *args: str, input: bytes | None = None) -> bytes:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_IDENTITY, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0",
               GIT_CEILING_DIRECTORIES=str(Path(cwd).resolve().parent), LC_ALL="C")
    argv = ["git", "-c", f"core.hooksPath={os.devnull}", "-c", "core.fsmonitor=false", *args]
    try:
        result = subprocess.run(argv, cwd=cwd, input=input, capture_output=True, env=env, timeout=GIT_TIMEOUT_S,
                                check=False)
    except (OSError, subprocess.TimeoutExpired) as err:
        raise RepoError(f"git {args[0]} could not run in {cwd}: {err}") from None
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()[:500]
        raise RepoError(f"git {' '.join(args[:2])} failed in {cwd}: {detail}")
    return result.stdout
