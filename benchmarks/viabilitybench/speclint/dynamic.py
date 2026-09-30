#!/usr/bin/env python3
"""speclint dynamic mode: the red-on-base check behind SQ06 and HF3 (S07.2).

``speclint.py plans/ --dynamic`` runs each implementer task's pinned acceptance tests and verify
steps twice on a clean checkout of the base commit and passes the task's ``red_on_base`` to
:func:`speclint.score_task`:

- ``fail``: the same step fails on the base in both runs. SQ06 = 1.
- ``pass``: every step passes on the base in both runs. HF3, unless the task pins no acceptance
  test and every step declares ``expect = "pass_on_base"``. No plan declares ``expect`` yet (S07.8
  adds it), so every implementer task is expected to fail on the base.
- ``unknown``: the runs disagree, a step times out or cannot start, a ``pass_on_base`` step
  fails, or the plan has no known base. SQ06 scores 0 and the record lists HF3 and SQ06 under
  ``unknown``.

Tasks that are not implementers, or have neither a pinned acceptance test nor a verify step, are
not run. A task's ``[task.accept]`` tests run first, as ``plan run`` runs them: each copies the
plan's ``src`` over its ``dest``, runs its ``runner`` and must report exactly ``count`` passing tests
(:func:`accept_steps`). A run stops at its first failing step, as ``plan run`` does. Each step runs
as ``bash -o pipefail -c <command>`` in the workspace root of the base checkout, for at most 120 s
(``--timeout``), or for its ``timeout_ms`` when that is shorter. Every record gets ``mode =
"dynamic"`` and a ``red_on_base_detail``: the base and why it was chosen, the outcome (a key of
:data:`OUTCOMES`), the reason, and each run's steps with their exit codes; a pinned test's step also
names its ``accept`` source.

The base. With ``--base REV`` every plan is checked against REV. Without it, a plan that has not
run is checked against HEAD, and the rest are ``unknown``: archived plans, and plans whose
``files`` changed in the commit that added the plan or later. Their work has probably landed, so
HEAD would pass their steps. No plan records its pre-change commit; pass it with ``--base``.

Safety. No step runs in the checkout. Each base commit gets one ``git worktree add --detach``
checkout, in a new scratch directory outside every worktree of the repo (``--scratch``; default:
the system temp directory). Before each run the checkout is reset to the base (``git reset
--hard``, ``git clean -ffdx``) and the plan's directory is copied in from the checkout, so an
uncommitted plan brings its own harness. When the run ends, the checker removes the worktrees and
the scratch directory it created, and nothing else. It creates no branch, never prunes, and never
touches another worktree.

``--fixture`` checks a plain directory instead: the workspace root is snapshotted into a
throwaway one-commit repo inside the scratch directory, and that commit is HEAD. The tests use it.
"""

from __future__ import annotations

import os
import posixpath
import shlex
import shutil
import signal
import subprocess
import tempfile
import threading
import time
import tomllib
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import IO

import speclint

STEP_TIMEOUT_S = 120.0
RUNS = 2

# red_on_base_detail.outcome, with its summary label.
OUTCOMES = {
    "fail": "fail: red on the base (SQ06 = 1)",
    "pass": "pass: every step passes (HF3)",
    "flaky": "unknown: the runs disagree",
    "timeout": "unknown: a step timed out",
    "error": "unknown: a step could not start",
    "base_broken": "unknown: a pass_on_base step fails",
    "no_base": "unknown: no known base",
    "not_run": "not run: no implementer verify",
}

# Variables that point git at another repository. Neither the steps nor the checker's own git
# commands see them, so nothing can reach the checkout through them.
_GIT_REDIRECTS = (
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
)
_TAIL_LINES = 5
_TAIL_CHARS = 400

# How a pinned acceptance step counts the passing tests its runner reported: a verbatim copy of
# task_accept.rs PASSED_COUNT_AWK (test_passed_count_awk_matches_task_accept keeps them equal).
PASSED_COUNT_AWK = r"""BEGIN { esc = sprintf("%c", 27); total = 0; seen = 0; ran = "" }
{ line = $0; gsub(esc "\\[[0-9;]*[A-Za-z]", "", line); gsub(/\r/, "", line) }
line ~ /^[ \t]*Test (Files|Suites):?[ \t]/ { next }
line ~ /test result: / || line ~ /^[ \t]*Tests:?[ \t]/ || line ~ /[0-9]+ passed.* in [0-9.]+m?s/ {
  n = split(line, w, /[^A-Za-z0-9]+/)
  for (i = 2; i <= n; i++) if (w[i] == "passed" && w[i - 1] ~ /^[0-9]+$/) { total += w[i - 1]; seen = 1 }
  next
}
line ~ /^[^A-Za-z0-9]*pass [0-9]+[ \t]*$/ { n = split(line, w, /[^A-Za-z0-9]+/); total += (w[n] == "" ? w[n - 1] : w[n]); seen = 1; next }
line ~ /^Ran [0-9]+ tests? in / { split(line, w, " "); ran = w[2]; next }
line ~ /^OK( |$)/ && ran != "" {
  skipped = 0
  if (match(line, /skipped=[0-9]+/)) skipped = substr(line, RSTART + 8, RLENGTH - 8)
  total += ran - skipped; seen = 1; ran = ""
}
END { if (seen) print total; else print "none" }"""


class CheckError(Exception):
    """The check cannot run: no git repository, an unknown base, or an unsafe scratch directory."""


def _env() -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if key not in _GIT_REDIRECTS}
    env.pop("CARGO_TARGET_DIR", None)  # builds stay inside the base checkout
    return env


def _terminate(signum: int, frame: object) -> None:
    raise SystemExit(128 + signum)


def _git(*args: str, cwd: Path, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        ["git", *args], cwd=cwd, env=_env(), stdin=subprocess.DEVNULL, capture_output=True, text=True
    )
    if check and proc.returncode != 0:
        raise CheckError(f"git {' '.join(args)} failed in {cwd}: {proc.stderr.strip() or proc.returncode}")
    return proc


# --------------------------------------------------------------------------------------------
# Base checkouts.


class BaseTrees:
    """Detached checkouts of base commits, in a scratch directory that only this run uses.

    Use it as a context manager: leaving the block removes the worktrees it added and the scratch
    directory, and nothing else. Inside the block a SIGTERM unwinds like Ctrl-C does, so a killed
    run still cleans up.
    """

    def __init__(self, root: Path, scratch_parent: Path | None = None, fixture: bool = False) -> None:
        self.root = root.resolve()
        self.fixture = fixture
        self.scratch_parent = scratch_parent
        self.scratch: Path | None = None
        self.repo = self.root
        self.prefix = Path()  # the workspace root, relative to the repo's top level
        self._admin = Path()  # the repo's worktrees/ directory
        self._trees: dict[str, tuple[Path, Path]] = {}  # base sha -> (checkout, its admin directory)
        self._on_sigterm: object = None  # the handler to restore, while ours is installed

    def __enter__(self) -> BaseTrees:
        parent = (self.scratch_parent or Path(tempfile.gettempdir())).resolve()
        if not parent.is_dir():
            raise CheckError(f"the scratch directory {parent} does not exist")
        if not self.root.is_dir():
            raise CheckError(f"the workspace root {self.root} is not a directory")
        if self.fixture:
            if (self.root / ".git").exists():
                raise CheckError(f"--fixture needs a plain directory, and {self.root} is a git checkout")
            checkouts = [self.root]
        else:
            top = _git("rev-parse", "--show-toplevel", cwd=self.root, check=False)
            if top.returncode != 0:
                raise CheckError(f"{self.root} is not in a git work tree (use --fixture for a plain directory)")
            self.repo = Path(top.stdout.strip()).resolve()
            if not self.root.is_relative_to(self.repo):
                raise CheckError(f"the workspace root {self.root} is outside its repo {self.repo}")
            self.prefix = self.root.relative_to(self.repo)
            listing = _git("worktree", "list", "--porcelain", cwd=self.repo).stdout
            checkouts = [Path(line[len("worktree ") :]).resolve() for line in listing.splitlines() if line.startswith("worktree ")]
        for checkout in checkouts:
            if parent.is_relative_to(checkout):
                raise CheckError(f"the scratch directory {parent} is inside the checkout {checkout}")
        if threading.current_thread() is threading.main_thread():
            self._on_sigterm = signal.signal(signal.SIGTERM, _terminate) or signal.SIG_DFL
        try:
            self.scratch = Path(tempfile.mkdtemp(prefix="speclint-dynamic-", dir=parent)).resolve()
            if self.fixture:
                self.repo = self._snapshot()
            common = _git("rev-parse", "--path-format=absolute", "--git-common-dir", cwd=self.repo).stdout.strip()
            self._admin = Path(common).resolve() / "worktrees"
        except BaseException:
            self.close()
            raise
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    def _snapshot(self) -> Path:
        assert self.scratch is not None
        repo = self.scratch / "snapshot"
        shutil.copytree(self.root, repo, symlinks=True, ignore=shutil.ignore_patterns(".git"))
        _git("init", "--quiet", cwd=repo)
        _git("add", "--all", cwd=repo)
        identity = ("-c", "user.name=speclint", "-c", "user.email=speclint@localhost", "-c", "commit.gpgsign=false")
        _git(*identity, "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "--no-verify", "-m", "speclint fixture", cwd=repo)
        return repo

    def resolve(self, rev: str) -> str:
        proc = _git("rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}", cwd=self.repo, check=False)
        if proc.returncode != 0:
            raise CheckError(f"unknown base {rev!r} in {self.repo}")
        return proc.stdout.strip()

    def plan_base(self, plan: Path, rel: str, tasks: list[dict], head: str) -> tuple[str | None, str]:
        """The commit to check a plan against when no --base is given, and why; None if unknown."""
        if "archive" in PurePosixPath(rel).parts:
            return None, "archived plan: its pre-change commit is not recorded; check it with --base"
        outputs: set[str] = set()
        for task in tasks:
            outputs |= speclint.task_outputs(task)
        landed = self._landed(plan, outputs)
        if landed:
            return None, (
                f"its files changed in {landed[:12]}, at or after the commit that added the plan, so its work "
                "has probably landed; check it with --base <pre-change commit>"
            )
        return head, "HEAD: the plan has not run"

    def _landed(self, plan: Path, outputs: set[str]) -> str | None:
        """The latest commit, from the one that added ``plan`` on, that touched the plan's files."""
        if not plan.is_relative_to(self.root):
            return None
        tracked = (self.prefix / plan.relative_to(self.root)).as_posix()
        added = _git("log", "--follow", "--diff-filter=A", "--format=%H", "--", tracked, cwd=self.repo).stdout.split()
        if not added:
            return None  # not committed yet, so it has not run
        first = added[-1]
        # A root commit has no state from before the plan to compare with: only later commits count.
        is_root = len(_git("rev-list", "--parents", "-n", "1", first, cwd=self.repo).stdout.split()) == 1
        span = f"{first}..HEAD" if is_root else f"{first}^..HEAD"
        paths = []
        for output in sorted(outputs):
            path = posixpath.normpath((self.prefix / output).as_posix())
            if not path.startswith(("/", "~", "$", "..")):
                paths.append(path)
        if not paths:
            return None
        return _git("log", "-1", "--format=%H", span, "--", *paths, cwd=self.repo).stdout.strip() or None

    def checkout(self, sha: str) -> tuple[Path, Path]:
        """The checkout of ``sha`` and its admin directory, added on first use."""
        if sha not in self._trees:
            assert self.scratch is not None
            tree = self.scratch / f"base-{sha[:12]}"
            _git("-c", "core.hooksPath=/dev/null", "worktree", "add", "--detach", "--quiet", str(tree), sha, cwd=self.repo)
            gitdir = (tree / ".git").read_text(encoding="utf-8").strip().removeprefix("gitdir:").strip()
            self._trees[sha] = (tree, (tree / gitdir).resolve())
        return self._trees[sha]

    def reset(self, sha: str, plan_dir: Path | None = None) -> Path:
        """The workspace root in the checkout of ``sha``, reset to the base, with ``plan_dir`` copied in."""
        tree, admin = self.checkout(sha)
        # An explicit git dir and work tree: even if a step broke the checkout's .git file, these
        # commands cannot reach another repository.
        where = (f"--git-dir={admin}", f"--work-tree={tree}")
        _git(*where, "reset", "--hard", "--quiet", sha, cwd=tree)
        _git(*where, "clean", "-ffdxq", cwd=tree)
        workspace = tree / self.prefix
        # A fixture snapshot already holds its plans; a plan at the workspace root has no directory
        # of its own to copy.
        if plan_dir is not None and not self.fixture and plan_dir != self.root and plan_dir.is_relative_to(self.root):
            shutil.copytree(
                plan_dir,
                workspace / plan_dir.relative_to(self.root),
                symlinks=True,
                ignore=shutil.ignore_patterns(".git"),
                dirs_exist_ok=True,
            )
        return workspace

    def close(self) -> None:
        if self._on_sigterm is not None:
            signal.signal(signal.SIGTERM, signal.SIG_IGN)  # finish cleaning up first
        for tree, admin in self._trees.values():
            removed = _git("worktree", "remove", "--force", "--force", str(tree), cwd=self.repo, check=False)
            if removed.returncode != 0:  # e.g. a step deleted its .git file: remove only what we added
                shutil.rmtree(tree, ignore_errors=True)
                if admin.parent == self._admin:
                    shutil.rmtree(admin, ignore_errors=True)
        self._trees.clear()
        if self.scratch is not None:
            shutil.rmtree(self.scratch, ignore_errors=True)
            self.scratch = None
        if self._on_sigterm is not None:
            signal.signal(signal.SIGTERM, self._on_sigterm)
            self._on_sigterm = None


# --------------------------------------------------------------------------------------------
# Running steps.


def step_timeout(step: dict, cap: float) -> float:
    ms = step.get("timeout_ms")
    if isinstance(ms, int) and not isinstance(ms, bool) and ms > 0:
        return min(cap, ms / 1000)
    return cap


def run_step(command: str, cwd: Path, timeout: float) -> dict:
    """Run one verify step as ``plan run`` does. ``exit`` is None when it timed out or could not start."""
    env = _env()
    env["PWD"] = str(cwd)
    with tempfile.TemporaryFile() as out:
        start = time.monotonic()
        try:
            proc = subprocess.Popen(
                ["bash", "-o", "pipefail", "-c", command],
                cwd=cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=out,
                stderr=subprocess.STDOUT,
                start_new_session=True,  # its own process group, so a timeout kills all it started
            )
        except OSError as err:
            return {"exit": None, "error": str(err), "secs": 0.0}
        timed_out = False
        try:
            proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
        finally:
            _kill_group(proc.pid)  # on a timeout or an interrupt, and whatever the step left running
            proc.wait()
        result: dict = {"exit": None if timed_out else proc.returncode, "secs": round(time.monotonic() - start, 2)}
        if timed_out:
            result["timed_out"] = True
        if timed_out or proc.returncode != 0:
            result["tail"] = _tail(out)
        return result


def _kill_group(pgid: int) -> None:
    try:
        os.killpg(pgid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def _tail(out: IO[bytes]) -> str:
    size = out.seek(0, os.SEEK_END)
    out.seek(max(0, size - 4096))
    lines = [line.rstrip() for line in out.read().decode("utf-8", "replace").splitlines() if line.strip()]
    return "\n".join(lines[-_TAIL_LINES:])[-_TAIL_CHARS:]


@dataclass
class Check:
    """One task's red-on-base result."""

    red_on_base: str  # fail | pass | unknown
    outcome: str  # a key of OUTCOMES
    reason: str
    base: str | None = None
    base_reason: str = ""
    runs: list[list[dict]] = field(default_factory=list)  # per run, the steps that ran

    def detail(self) -> dict:
        return {
            "base": self.base,
            "base_reason": self.base_reason,
            "outcome": self.outcome,
            "reason": self.reason,
            "runs": self.runs,
        }


def accept_steps(task: dict, plan_source: str) -> list[dict]:
    """The steps that run a task's `[task.accept]` tests, which ``plan run`` puts before its own.

    Each is the Graph path's pinned step (task_accept.rs ``pinned_verify_step``) with the plan's
    ``src`` in place of the pinned copy, since the base has no pin store and no hash to check. It
    copies ``src`` over ``dest``, runs ``runner`` from the workspace root and requires exactly
    ``count`` passing tests. ``plan_source`` is the plan's directory as the workspace root sees it.
    """
    steps = []
    for test in speclint.accept_tests(task):
        src = speclint._str(test.get("src")).strip()
        dest = speclint._str(test.get("dest")).strip()
        count = test["count"]
        runner = speclint._str(test.get("runner")).replace("{dest}", '"$roko_dest"').replace("{count}", str(count))
        lines = [
            f"# roko accept (speclint --dynamic): {src} -> {dest} (exactly {count} passing tests)",
            f"roko_src={shlex.quote(posixpath.normpath(posixpath.join(plan_source, src)))}",
            f'roko_dest="$PWD"/{shlex.quote(dest)}',
            'mkdir -p "$(dirname "$roko_dest")" && cp -f "$roko_src" "$roko_dest" || exit 1',
            "roko_log=$(mktemp) || exit 1",
            "{ " + runner,
            '} 2>&1 | tee "$roko_log"',
            "roko_rc=$?",
            f"roko_passed=$(awk '{PASSED_COUNT_AWK}' \"$roko_log\")",
            'rm -f "$roko_log"',
            'if [ "$roko_rc" -ne 0 ]; then',
            '  echo "roko accept: $roko_src failed: its runner exited with status $roko_rc" >&2',
            "  exit 1",
            "fi",
            f'if [ "$roko_passed" != "{count}" ]; then',
            f'  echo "roko accept: $roko_src must pass exactly {count} tests; the runner reported $roko_passed" >&2',
            "  exit 1",
            "fi",
        ]
        steps.append({"command": "\n".join(lines), "timeout_ms": test.get("timeout_ms"), "accept": src})
    return steps


def check_task(task: dict, trees: BaseTrees, sha: str, plan_dir: Path | None, cap: float) -> Check:
    """Run a task's pinned acceptance tests, then its verify steps, on the base, twice, and classify the result."""
    plan_dir = plan_dir or trees.root
    missing = [
        speclint._str(test.get("src")).strip()
        for test in speclint.accept_tests(task)
        if not (plan_dir / speclint._str(test.get("src")).strip()).is_file()
    ]
    if missing:
        return Check("unknown", "error", "pinned acceptance test missing from the plan: " + ", ".join(missing), base=sha)
    plan_source = plan_dir.relative_to(trees.root).as_posix() if plan_dir.is_relative_to(trees.root) else plan_dir.as_posix()
    steps = accept_steps(task, plan_source) + speclint._tables(task.get("verify"))
    runs: list[list[dict]] = []
    for _ in range(RUNS):
        cwd = trees.reset(sha, plan_dir)
        results: list[dict] = []
        for number, step in enumerate(steps, 1):
            result: dict = {"step": number}
            if "accept" in step:
                result["accept"] = step["accept"]
            result.update(run_step(speclint._str(step.get("command")), cwd, step_timeout(step, cap)))
            if "tail" in result:
                result["tail"] = result["tail"].replace(str(trees.scratch), "$SCRATCH")
            results.append(result)
            if result["exit"] != 0:
                break
        runs.append(results)
        if results[-1]["exit"] is None:
            break  # a timeout or a failed start: the task is unknown whatever a second run does
    red_on_base, outcome, reason = classify(steps, runs)
    return Check(red_on_base, outcome, reason, base=sha, runs=runs)


def classify(steps: list[dict], runs: list[list[dict]]) -> tuple[str, str, str]:
    """(red_on_base, outcome, reason) for the runs of one task's verify steps."""
    verdicts: list[tuple[str, int]] = []  # ("pass", 0) or ("fail", the failing step)
    for number, results in enumerate(runs, 1):
        last = results[-1]
        step = last["step"]
        if "error" in last:
            return "unknown", "error", f"step {step} could not start: {last['error']}"
        if last.get("timed_out"):
            return "unknown", "timeout", f"step {step} timed out after {last['secs']:g} s in run {number}"
        if last["exit"] == 0:
            verdicts.append(("pass", 0))
        elif speclint._str(steps[step - 1].get("expect")) == "pass_on_base":
            return "unknown", "base_broken", f"step {step} expects to pass on the base but exits {last['exit']} in run {number}"
        else:
            verdicts.append(("fail", step))
    if len(set(verdicts)) > 1:
        runs_said = [
            f"run {number} {'passes' if verdict == 'pass' else f'fails at step {step}'}"
            for number, (verdict, step) in enumerate(verdicts, 1)
        ]
        return "unknown", "flaky", "the runs disagree: " + ", ".join(runs_said)
    verdict, step = verdicts[0]
    if verdict == "pass":
        return "pass", "pass", "every step passes on the base in both runs"
    exits = ", ".join(str(results[-1]["exit"]) for results in runs)
    return "fail", "fail", f"step {step} fails on the base in both runs (exit {exits})"


# --------------------------------------------------------------------------------------------
# Corpus.


def lint(
    files: list[Path],
    root: Path,
    *,
    base: str | None = None,
    timeout: float = STEP_TIMEOUT_S,
    scratch: Path | None = None,
    fixture: bool = False,
) -> tuple[list[dict], list[tuple[str, str]]]:
    """Check every task of ``files`` on its base, then score them like :func:`speclint.lint_files`.

    Returns (records without `ts`, parse errors). Raises :class:`CheckError` when the check
    cannot start.
    """
    root = root.resolve()
    checks: dict[tuple[str, str], Check] = {}
    with BaseTrees(root, scratch, fixture) as trees:
        head = trees.resolve(base or "HEAD")
        for path in files:
            try:
                data = tomllib.loads(path.read_text(encoding="utf-8"))
            except (OSError, tomllib.TOMLDecodeError, UnicodeDecodeError):
                continue  # lint_files reports it
            rel = speclint._relative(path, root)
            tasks = speclint._tables(data.get("task"))
            plan_base: tuple[str | None, str] | None = None
            for task in tasks:
                key = (rel, speclint._str(task.get("id")))
                role = speclint._str(task.get("role")).strip() or "implementer"
                if role != "implementer":
                    checks[key] = Check("unknown", "not_run", f"not run: a {role} task")
                    continue
                if not speclint._tables(task.get("verify")) and not speclint.accept_tests(task):
                    checks[key] = Check("unknown", "not_run", "not run: no verify step")
                    continue
                if plan_base is None:
                    plan_base = (head, f"--base {base}") if base else trees.plan_base(path, rel, tasks, head)
                sha, why = plan_base
                if sha is None:
                    checks[key] = Check("unknown", "no_base", why, base_reason=why)
                    continue
                checks[key] = check_task(task, trees, sha, path.parent, timeout)
                checks[key].base_reason = why
    records, errors = speclint.lint_files(files, root, {key: check.red_on_base for key, check in checks.items()})
    for record in records:
        check = checks.get((record["plan_path"], record["task_id"]))
        record["mode"] = "dynamic"
        record["red_on_base_detail"] = check.detail() if check else None
    return records, errors


def summary_lines(records: list[dict]) -> list[str]:
    """The red-on-base block of the summary: tasks by outcome, and the bases used."""
    groups = {
        "all": records,
        "active": [r for r in records if not r["archived"]],
        "archived": [r for r in records if r["archived"]],
    }

    def row(label: str, cells: list[str]) -> str:
        return f"  {label:<38}" + "".join(f"{cell:>10}" for cell in cells)

    def outcome(record: dict) -> str:
        return (record.get("red_on_base_detail") or {}).get("outcome", "")

    lines = ["", row("Red on base (tasks)", list(groups))]
    for key, label in OUTCOMES.items():
        lines.append(row(label, [str(sum(1 for r in g if outcome(r) == key)) for g in groups.values()]))
    bases: dict[str, str | None] = {}
    for record in records:
        if outcome(record) not in ("", "not_run"):
            bases[record["plan_path"]] = record["red_on_base_detail"]["base"]
    checked = sorted({sha for sha in bases.values() if sha})
    lines += [
        "",
        f"  plans checked: {sum(1 for sha in bases.values() if sha)}"
        f" (base {', '.join(sha[:12] for sha in checked) or '-'});"
        f" without a known base: {sum(1 for sha in bases.values() if not sha)}"
        " (archived, or already run: see red_on_base_detail.base_reason)",
    ]
    return lines
