#!/usr/bin/env python3
"""speclint: the static spec-quality score (SQS v1, linter id ``sq-2``) for roko task specs.

Scores every ``[[task]]`` of ``plans/**/tasks.toml`` against rules SQ01-SQ12 and the static hard
fails of S07 section 4.2 (``tmp/cybernetic-harness/specs/S07-spec-quality.md``), writes one
``spec.quality`` JSONL record per task (S07 section 5) and prints a summary::

    python3 benchmarks/viabilitybench/speclint/speclint.py plans/ \\
        --out "$VB_RESULTS/speclint/<run_id>/speclint.jsonl"

Without ``--out`` the records go to ``$VB_RESULTS/speclint/<run_id>/speclint.jsonl``, outside the
repo (decision D4; ``$VB_RESULTS`` defaults to ``~/.roko-bench/viability``). Standard library only,
no model calls. Records are deterministic apart from ``ts``.

Static mode cannot run anything, so SQ06 (red on base) scores 0 and HF3 is not evaluated; both are
listed under ``unknown`` in every record. ``--dynamic`` first runs each implementer task's verify
steps on a clean checkout of the base commit and passes the task's ``red_on_base`` to
:func:`score_task` (``dynamic.py``, S07.2)::

    python3 benchmarks/viabilitybench/speclint/speclint.py plans/ --dynamic [--base REV]

The rule definitions below are frozen as ``sq-2``: the Rust port (``roko plan validate
--spec-quality``, gap-46ab3f) must match them within 0.5 points on the golden fixtures in
``fixtures/``. Change a definition only together with the linter id.

Linter ids:

- ``sq-1``: SQ01-SQ12 and the static hard fails as S07 section 4.2 defines them.
- ``sq-2``: well-formed ``[task.accept]`` entries count as scoped test verify steps and as observable
  acceptance (bug-019f02).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import posixpath
import re
import sys
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath

LINTER = "sq-2"

WEIGHTS = {
    "SQ01": 10,
    "SQ02": 15,
    "SQ03": 5,
    "SQ04": 15,
    "SQ05": 5,
    "SQ06": 15,
    "SQ07": 8,
    "SQ08": 4,
    "SQ09": 5,
    "SQ10": 8,
    "SQ11": 5,
    "SQ12": 5,
}
RULE_NAMES = {
    "SQ01": "goal",
    "SQ02": "acceptance",
    "SQ03": "traceability",
    "SQ04": "verify strength",
    "SQ05": "specificity",
    "SQ06": "red on base",
    "SQ07": "context",
    "SQ08": "scope",
    "SQ09": "non-goals",
    "SQ10": "vagueness",
    "SQ11": "anchors",
    "SQ12": "hidden hook",
}
HARD_FAILS = {
    "HF1": "implementer task without verify",
    "HF2": "vacuous verify step",
    "HF3": "verify passes on the unchanged base",
    "HF4": "missing context file",
    "HF5": "greenfield claim",
}
STATIC_UNKNOWN = ["HF3", "SQ06"]

# SQ04: the value of a task's strongest verify step.
CLASS_VALUE = {"test": 1.0, "run": 0.8, "compile": 0.5, "structural": 0.25, "vacuous": 0.0}
CLASS_ORDER = ("test", "run", "compile", "structural", "vacuous")

# SQ08 and the --strict limit on `files` (plan_policy.rs NORMAL_MAX_FILES_PER_TASK).
MAX_FILES = 32

# SQ10: vague-term lexicon v1 (S07 section 4.2), matched case-insensitively outside code spans.
VAGUE_TERMS = (
    "improve", "better", "robust", "clean up", "as needed", "appropriate", "appropriately",
    "proper", "properly", "various", "some", "etc.", "TBD", "TODO", "maybe", "possibly", "nice",
    "good", "reasonable", "efficient", "efficiently", "user-friendly", "seamless", "seamlessly",
    "flexible", "handle", "handles", "support for", "if possible", "and/or", "polish",
)

# HF5: the phrases of PLAN_033 (plan_validate.rs GREENFIELD_PHRASES), searched in prompt,
# description and title when the workspace has crates.
GREENFIELD_PHRASES = (
    "no rust crates exist",
    "no existing crates",
    "starting from scratch",
    "greenfield project",
    "greenfield implementation",
    "new project from scratch",
    "no existing code",
    "empty workspace",
)

_VAGUE_RE = re.compile(
    r"(?<![\w-])(?:"
    + "|".join(re.escape(term).replace(r"\ ", r"\s+") for term in sorted(VAGUE_TERMS, key=len, reverse=True))
    + r")(?![\w-])",
    re.IGNORECASE,
)
_CODE_FENCE_RE = re.compile(r"```.*?```", re.DOTALL)
_CODE_SPAN_RE = re.compile(r"`[^`\n]*`")

# S07 section 3.3 counts a task as having acceptance criteria when it has `acceptance`,
# `acceptance_contract` or acceptance phrasing. The phrasing test is lexical, as in the prototype:
# it also matches incidental mentions such as "run the acceptance check".
_ACCEPTANCE_PHRASE_RE = re.compile(
    r"\b(?:acceptance|criteri(?:a|on)|done when|definition of done|expected (?:output|result|behaviou?r))\b",
    re.IGNORECASE,
)
# SQ02 reads criteria only from an explicit section: a line that starts with the marker and ends
# in a colon or the end of the line, plus the bullet lines under it.
_ACCEPTANCE_HEADER_RE = re.compile(
    r"^\s*(?:#+\s*)?(?:[-*+]\s+)?(?:\*\*)?"
    r"(?:acceptance(?:\s+criteria)?|done\s+when|success\s+criteria|definition\s+of\s+done"
    r"|expected\s+(?:output|result|behaviou?r))"
    r"(?:\*\*)?\s*(?::|$)(?:\*\*)?\s*(.*)$",
    re.IGNORECASE,
)
_BULLET_RE = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+(.*\S)\s*$")
_AC_ID_RE = re.compile(r"^\s*(AC\d+)\b")
# SQ02: a criterion is observable when it names a comparator, a concrete value or an observable
# effect ("returns/raises/exits/prints" and their kin).
_OBSERVABLE_RE = re.compile(
    r"==|!=|>=|<=|≥|≤|`[^`]+`|\"[^\"]+\"|\d"
    r"|\b(?:equals?|equal to|exactly|at least|at most|no more than|fewer than|more than|less than"
    r"|greater than|same as|identical|unchanged|matches|contains?|returns?|returned|raises?|raised"
    r"|exits?|exited|exit code|prints?|printed|outputs?|emits?|emitted|writes?|written|fails?"
    r"|failed|passes|passed|rejects?|rejected|errors?|panics?|responds?|logs?|lists?|shows?"
    r"|displays?|reports?|produces?|creates?|deletes?|removes?|exists?)\b",
    re.IGNORECASE,
)
# SQ09: an explicit scope exclusion in the prose.
_NON_GOAL_RE = re.compile(
    r"\b(?:do not|don't|must not|out of scope|non-goals?|not in scope)\b", re.IGNORECASE
)
# SQ11: a concrete file, type or function.
_ANCHOR_RE = re.compile(
    r"`[^`\n]+`"
    r"|\b[\w.-]+/[\w./-]*\.[A-Za-z0-9]{1,8}\b"
    r"|\b[\w-]+\.(?:rs|toml|md|json|jsonl|ts|tsx|js|mjs|cjs|py|sh|ya?ml|go|txt|csv|html|css|lock)\b"
    r"|\b\w+::\w+"
    r"|\b[A-Za-z_]\w*\(\)"
    r"|\b[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]*)+\b"
    r"|\b[a-z][a-z0-9]*_[a-z0-9_]+\b"
)


# --------------------------------------------------------------------------------------------
# Shell analysis. Verify steps run as `bash -o pipefail -c <command>` (graph_task_dispatch.rs), so
# a pipeline fails when any stage fails; only a trailing `|| true`, `; exit 0` or a bare
# true/echo makes a step unable to fail.


@dataclass
class Cmd:
    words: list[str]
    op: str  # the operator before this command at its level: "", ";", "&&", "||", "|", "&"
    depth: int  # 0 = the step's top level; > 0 inside $(...), `...`, (...) or <(...)


def parse_shell(src: str) -> list[Cmd]:
    """Split a bash command into simple commands, in the order they complete.

    Quotes are removed, redirections are dropped, and commands inside ``$(...)``, backticks,
    subshells and process substitutions are listed before the command that contains them.
    """
    out: list[Cmd] = []
    _parse_into(src, 0, out)
    return out


def _parse_into(src: str, depth: int, out: list[Cmd]) -> None:
    n = len(src)
    i = 0
    words: list[str] = []
    word: list[str] = []
    in_word = False
    skip_next_word = False  # the next word is a redirection target
    op = ""

    def end_word() -> None:
        nonlocal word, in_word, skip_next_word
        if in_word:
            if skip_next_word:
                skip_next_word = False
            else:
                words.append("".join(word))
        word, in_word = [], False

    def end_command(next_op: str) -> None:
        nonlocal words, op
        end_word()
        if words:
            out.append(Cmd(words, op, depth))
            op = next_op
        elif next_op in ("&&", "||", "|"):
            op = next_op
        # A blank line or `;` after `&&`, `||` or `|` keeps that operator: the list continues.
        words = []

    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c in " \t\r":
            end_word()
            i += 1
        elif c == "\n":
            end_command(";")
            i += 1
        elif c == "#" and not in_word:
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif c == "\\":
            if nxt == "\n":
                i += 2
                continue
            if nxt:
                word.append(nxt)
            in_word = True
            i += 2
        elif c == "'":
            j = src.find("'", i + 1)
            j = n if j < 0 else j
            word.append(src[i + 1 : j])
            in_word = True
            i = j + 1
        elif c == '"':
            i = _read_double_quoted(src, i + 1, depth, out, word)
            in_word = True
        elif c == "`":
            j = _backtick_end(src, i + 1)
            _parse_into(src[i + 1 : j], depth + 1, out)
            word.append("$(…)")
            in_word = True
            i = j + 1
        elif c == "$" and nxt == "(":
            j = _paren_end(src, i + 2)
            if src[i + 2 : i + 3] != "(":  # $(( arithmetic )) holds no commands
                _parse_into(src[i + 2 : j], depth + 1, out)
            word.append("$(…)")
            in_word = True
            i = j + 1
        elif c == "$" and nxt == "{":
            j = _brace_end(src, i + 2)
            word.append(src[i : j + 1])
            in_word = True
            i = j + 1
        elif c in "<>":
            if in_word and "".join(word).isdigit():  # the fd of `2>`
                word, in_word = [], False
            else:
                end_word()
            j = i
            while j < n and src[j] in "<>":
                j += 1
            if j < n and src[j] == "&":  # 2>&1, >&2
                j += 1
                while j < n and (src[j].isdigit() or src[j] == "-"):
                    j += 1
                i = j
            elif j < n and src[j] == "(" and j == i + 1:  # <( ... ) process substitution
                k = _paren_end(src, j + 1)
                _parse_into(src[j + 1 : k], depth + 1, out)
                i = k + 1
            else:
                skip_next_word = True
                i = j
        elif c == "|":
            if nxt == "|":
                end_command("||")
                i += 2
            else:
                end_command("|")
                i += 2 if nxt == "&" else 1
        elif c == "&":
            if nxt == "&":
                end_command("&&")
                i += 2
            elif nxt == ">":  # &> file
                end_word()
                i += 3 if src[i + 2 : i + 3] == ">" else 2
                skip_next_word = True
            else:
                end_command("&")
                i += 1
        elif c == ";":
            end_command(";")
            i += 2 if nxt == ";" else 1
        elif c == "(" and not in_word and not words:
            j = _paren_end(src, i + 1)
            _parse_into(src[i + 1 : j], depth + 1, out)
            words.append("(subshell)")
            i = j + 1
        else:
            word.append(c)
            in_word = True
            i += 1
    end_command("")


def _read_double_quoted(src: str, i: int, depth: int, out: list[Cmd], word: list[str]) -> int:
    n = len(src)
    while i < n:
        c = src[i]
        if c == '"':
            return i + 1
        if c == "\\" and i + 1 < n:
            word.append(src[i + 1])
            i += 2
        elif c == "$" and src[i + 1 : i + 2] == "(":
            j = _paren_end(src, i + 2)
            if src[i + 2 : i + 3] != "(":
                _parse_into(src[i + 2 : j], depth + 1, out)
            word.append("$(…)")
            i = j + 1
        elif c == "`":
            j = _backtick_end(src, i + 1)
            _parse_into(src[i + 1 : j], depth + 1, out)
            word.append("$(…)")
            i = j + 1
        else:
            word.append(c)
            i += 1
    return n


def _skip_double_quoted(src: str, i: int) -> int:
    n = len(src)
    while i < n:
        c = src[i]
        if c == "\\":
            i += 2
        elif c == '"':
            return i + 1
        elif c == "$" and src[i + 1 : i + 2] == "(":
            i = _paren_end(src, i + 2) + 1
        elif c == "`":
            i = _backtick_end(src, i + 1) + 1
        else:
            i += 1
    return n


def _paren_end(src: str, i: int) -> int:
    """Index of the `)` closing a `(` that ends just before ``i`` (len(src) if unbalanced)."""
    n = len(src)
    level = 1
    while i < n:
        c = src[i]
        if c == "\\":
            i += 2
            continue
        if c == "'":
            j = src.find("'", i + 1)
            i = n if j < 0 else j + 1
            continue
        if c == '"':
            i = _skip_double_quoted(src, i + 1)
            continue
        if c == "`":
            i = _backtick_end(src, i + 1) + 1
            continue
        if c == "(":
            level += 1
        elif c == ")":
            level -= 1
            if level == 0:
                return i
        i += 1
    return n


def _brace_end(src: str, i: int) -> int:
    n = len(src)
    level = 1
    while i < n:
        c = src[i]
        if c == "\\":
            i += 2
            continue
        if c == "{":
            level += 1
        elif c == "}":
            level -= 1
            if level == 0:
                return i
        i += 1
    return n


def _backtick_end(src: str, i: int) -> int:
    n = len(src)
    while i < n:
        if src[i] == "\\":
            i += 2
            continue
        if src[i] == "`":
            return i
        i += 1
    return n


_OPENERS = {"if", "for", "while", "until", "case", "select", "{"}
_CLOSERS = {"fi", "done", "esac", "}"}
_LEAD_WORDS = {"then", "do", "else", "elif", "!", "time"}
_ASSIGNMENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\+?=")


def _top_level_units(cmds: list[Cmd]) -> list[tuple[str, list[str] | None]]:
    """The step's top-level commands as (operator, words); a compound block is (operator, None)."""
    units: list[tuple[str, list[str] | None]] = []
    level = 0
    for cmd in cmds:
        if cmd.depth:
            continue
        words = cmd.words
        start_level = level
        k = 0
        while k < len(words) and (words[k] in _OPENERS or words[k] in _LEAD_WORDS):
            if words[k] in _OPENERS:
                level += 1
            if words[k] == "for" or words[k] == "case" or words[k] == "select":
                break  # the rest is a loop header, not a command
            k += 1
        if words and words[0] in _CLOSERS:
            level = max(0, level - 1)
            continue
        if start_level == 0:
            units.append((cmd.op, None if level > 0 else words))
    return units


def _always_succeeds(words: list[str] | None) -> bool:
    if not words:
        return False
    rest = [w for w in words if not _ASSIGNMENT_RE.match(w)]
    if not rest:
        return False
    head = rest[0]
    return head in ("true", ":", "echo", "printf") or rest == ["exit", "0"]


def vacuous_reason(command: str) -> str | None:
    """Why a verify step can never fail (HF2), or None."""
    if not command.strip():
        return "empty command"
    units = _top_level_units(parse_shell(command))
    if not units:
        return "no command"
    lists: list[list[tuple[str, list[str] | None]]] = [[]]
    for op, words in units:
        if op in (";", "&") and lists[-1]:
            lists.append([])
        lists[-1].append((op, words))
    for lst in lists:
        if len(lst) == 1 and lst[0][1] == ["exit", "0"]:
            return "unconditional `exit 0`"
    last = lists[-1]
    # Group the final list into pipelines; with pipefail a pipeline fails if any stage fails.
    pipelines: list[tuple[str, list[list[str] | None]]] = []
    for op, words in last:
        if op == "|" and pipelines:
            pipelines[-1][1].append(words)
        else:
            pipelines.append((op, [words]))
    final_op, final = pipelines[-1]
    if all(_always_succeeds(words) for words in final):
        if len(pipelines) == 1:
            return "the step only runs `" + " ".join(final[0] or []) + "`"
        if final_op == "||":
            return "ends in `|| " + " ".join(final[0] or []) + "`"
    return None


# Programs, by what running them proves.
_SETUP = {
    "cd", "pushd", "popd", "cp", "mv", "rm", "rmdir", "mkdir", "touch", "ln", "chmod", "chown",
    "export", "set", "unset", "source", ".", "echo", "printf", "true", ":", "false", "exit",
    "return", "sleep", "mktemp", "tee", "trap", "local", "read", "wait", "kill", "shift",
    "declare", "typeset", "let", "ulimit", "umask", "eval", "(subshell)", "install", "pwd",
}
_STRUCTURAL = {
    "grep", "egrep", "fgrep", "rg", "ag", "test", "[", "[[", "ls", "wc", "head", "tail", "cat",
    "find", "fd", "stat", "file", "diff", "cmp", "jq", "yq", "awk", "gawk", "sed", "sort", "uniq",
    "cut", "tr", "xmllint", "git", "sha256sum", "shasum", "md5sum", "readlink", "realpath",
    "basename", "dirname", "du", "tomlq", "taplo", "column", "comm", "od", "xxd", "strings",
    "tree", "nl", "paste", "join", "expr", "seq", "date", "which", "type", "hash", "command",
}
_ASSERTING = {"grep", "egrep", "fgrep", "rg", "ag", "test", "[", "[[", "diff", "cmp", "jq", "yq"}
_TEST_RUNNERS = {
    "pytest", "py.test", "vitest", "jest", "mocha", "ava", "tap", "playwright", "cypress", "karma",
    "ctest", "phpunit", "rspec", "nextest",
}
_COMPILERS = {
    "tsc", "vue-tsc", "rustc", "gcc", "g++", "cc", "clang", "clang++", "javac", "kotlinc",
    "swiftc", "esbuild", "webpack", "rollup", "mypy", "pyright", "eslint", "ruff", "shellcheck",
}
_TEST_TOKENS = {"test", "tests", "spec", "vitest", "jest", "pytest"}
_BUILD_TOKENS = {"build", "typecheck", "tsc", "compile", "check", "lint", "types"}
_HARNESS_TOKENS = {"check", "verify", "validate", "accept", "acceptance", "smoke", "e2e", "harness"}
_CARGO_VALUE_FLAGS = {
    "-p", "--package", "--manifest-path", "--test", "--bin", "--example", "--bench", "--features",
    "-F", "--target", "--target-dir", "--profile", "-j", "--jobs", "--color", "--message-format",
    "--exclude", "-Z", "--config", "--lockfile-path",
}
_TEST_BINARY_VALUE_FLAGS = {"--test-threads", "--skip", "--format", "--color", "--logfile", "-Z"}


@dataclass
class Use:
    """One program invocation inside a verify step."""

    kind: str  # test | compile | structural | assert | exec | setup
    scope: str = ""  # test and compile: "scoped" or "workspace"
    no_run: bool = False  # `cargo test --no-run`
    target: str = ""  # exec: the script or binary path, relative to the repo root
    harness: bool = False  # exec of a check script the task does not write


def _tokens(name: str) -> set[str]:
    return {t for t in re.split(r"[-_.:/]+", name.lower()) if t}


def _strip_wrappers(words: list[str]) -> list[str]:
    w = list(words)
    while w and w[0] in _LEAD_WORDS | {"if", "while", "until", "{"}:
        w = w[1:]
    if not w or w[0] in _CLOSERS | {"for", "case", "select", "function", "in"}:
        return []
    while w and _ASSIGNMENT_RE.match(w[0]):
        w = w[1:]
    while w:
        head = PurePosixPath(w[0]).name
        if head == "env":
            w = w[1:]
            while w and (w[0].startswith("-") or _ASSIGNMENT_RE.match(w[0])):
                w = w[1:]
        elif head in ("timeout", "gtimeout"):
            w = w[1:]
            while w and w[0].startswith("-"):
                w = w[1:]
            w = w[1:]
        elif head in ("nice", "nohup", "exec", "builtin", "stdbuf", "time"):
            w = w[1:]
            while w and w[0].startswith("-"):
                w = w[1:]
        elif head == "xargs":
            w = w[1:]
            while w and w[0].startswith("-"):
                flag = w.pop(0)
                if flag in ("-I", "-n", "-L", "-P", "-d", "-E", "-s") and w:
                    w.pop(0)
        else:
            break
    return w


def _first_positional(args: list[str]) -> str:
    return next((a for a in args if not a.startswith(("-", "+"))), "")


def _has_positional(args: list[str], skip: frozenset[str] | set[str] = frozenset()) -> bool:
    return any(not a.startswith("-") and a not in skip for a in args)


def _cargo_scope(sub: str, args: list[str]) -> str:
    rest = args[args.index(sub) + 1 :] if sub in args else args
    positional = []
    i = 0
    after_dashes = False
    while i < len(rest):
        arg = rest[i]
        if arg == "--" and not after_dashes:
            after_dashes = True
        elif after_dashes and arg in _TEST_BINARY_VALUE_FLAGS:
            i += 1
        elif not after_dashes and arg in ("-p", "--package", "--manifest-path", "--test", "--bin", "--example"):
            return "scoped"
        elif not after_dashes and arg.startswith(("--package=", "--manifest-path=", "--test=", "--bin=")):
            return "scoped"
        elif not after_dashes and arg.startswith("-p") and len(arg) > 2 and not arg.startswith("--"):
            return "scoped"
        elif not after_dashes and arg in _CARGO_VALUE_FLAGS:
            i += 1
        elif not arg.startswith("-"):
            positional.append(arg)
        i += 1
    if sub in ("test", "t", "nextest") and positional and "--workspace" not in args:
        return "scoped"
    return "workspace"


def _script_use(target: str, script_args: list[str], cwd: str, task_files: set[str]) -> Use:
    path = _resolve(cwd, target)
    tokens = _tokens(PurePosixPath(target).name)
    if tokens & _TEST_TOKENS:
        return Use("test", scope="scoped" if (cwd or script_args) else "workspace", target=path)
    harness = bool(tokens & _HARNESS_TOKENS) and path not in task_files
    return Use("exec", target=path, harness=harness)


def _resolve(cwd: str, path: str) -> str:
    if path.startswith(("$", "~", "/")):
        return path
    joined = posixpath.normpath(posixpath.join(cwd, path)) if cwd else posixpath.normpath(path)
    return joined


def _classify(words: list[str], cwd: str, task_files: set[str]) -> list[Use]:
    argv = _strip_wrappers(words)
    if not argv:
        return []
    raw = argv[0]
    prog = PurePosixPath(raw).name
    args = argv[1:]
    scoped = "scoped" if cwd else ""

    if prog in ("sh", "bash", "zsh", "dash"):
        if "-c" in args:
            idx = args.index("-c")
            if idx + 1 < len(args):
                nested: list[Use] = []
                for cmd in parse_shell(args[idx + 1]):
                    nested.extend(_classify(cmd.words, cwd, task_files))
                return nested
            return []
        if "-n" in args:
            return [Use("compile", scope="scoped")]
        script = _first_positional(args)
        if not script:
            return [Use("setup")]
        return [_script_use(script, args[args.index(script) + 1 :], cwd, task_files)]

    if prog == "cargo":
        sub = _first_positional(args)
        if sub in ("test", "t"):
            if "--no-run" in args:
                return [Use("compile", scope=scoped or _cargo_scope(sub, args), no_run=True)]
            return [Use("test", scope=scoped or _cargo_scope(sub, args))]
        if sub == "nextest":
            return [Use("test", scope=scoped or _cargo_scope(sub, args))]
        if sub in ("check", "c", "build", "b", "clippy", "doc", "rustc", "bench"):
            return [Use("compile", scope=scoped or _cargo_scope(sub, args))]
        if sub in ("run", "r"):
            return [Use("exec")]
        if sub in ("fetch", "install", "update", "generate-lockfile", "clean"):
            return [Use("setup")]
        return [Use("structural")]

    if prog in ("npm", "pnpm", "yarn", "bun"):
        sub = _first_positional(args)
        scoped_flag = any(a in ("--prefix", "-C", "--dir", "--filter", "-w", "--workspace") for a in args)
        scope = "scoped" if (cwd or scoped_flag or "--" in args) else "workspace"
        if sub in ("ci", "install", "i", "add", "remove", "uninstall", "link", "audit", "outdated"):
            return [Use("setup")]
        if sub in ("exec", "dlx", "x") or (prog == "bun" and sub == "x"):
            rest = args[args.index(sub) + 1 :]
            return _classify(rest, cwd, task_files)
        if sub in ("test", "t", "tst"):
            return [Use("test", scope=scope)]
        script = _first_positional(args[args.index(sub) + 1 :]) if sub in ("run", "run-script") else sub
        tokens = _tokens(script)
        if tokens & _TEST_TOKENS:
            return [Use("test", scope=scope)]
        if tokens & _BUILD_TOKENS:
            return [Use("compile", scope=scope)]
        return [Use("exec")]

    if prog in ("npx", "pnpx", "bunx"):
        rest = list(args)
        while rest and rest[0].startswith("-"):
            flag = rest.pop(0)
            if flag in ("-p", "--package") and rest:
                rest.pop(0)
        return _classify(rest, cwd, task_files)

    if prog in _TEST_RUNNERS:
        skip = {"run", "watch", "dev", "related", "test"}
        return [Use("test", scope=scoped or ("scoped" if _has_positional(args, skip) or "-k" in args or "-t" in args else "workspace"))]

    if prog in ("next", "vite"):
        sub = _first_positional(args)
        if sub in ("build", "lint"):
            return [Use("compile", scope=scoped or "workspace")]
        return [Use("exec")]

    if prog in _COMPILERS:
        if prog in ("tsc", "vue-tsc"):
            return [Use("compile", scope=scoped or ("scoped" if "-p" in args or "--project" in args else "workspace"))]
        if prog in ("eslint", "ruff", "mypy", "pyright", "shellcheck"):
            paths = [a for a in args if not a.startswith("-") and a not in ("check", ".")]
            return [Use("compile", scope=scoped or ("scoped" if paths else "workspace"))]
        return [Use("compile", scope="scoped")]

    if prog in ("python", "python3", "python2", "py", "pypy3") or re.fullmatch(r"python3\.\d+", prog):
        if "-m" in args:
            idx = args.index("-m")
            module = args[idx + 1] if idx + 1 < len(args) else ""
            rest = args[idx + 2 :]
            if module in ("pytest", "unittest", "doctest", "nose2"):
                return [Use("test", scope=scoped or ("scoped" if _has_positional(rest, {"discover"}) else "workspace"))]
            if module in ("py_compile", "compileall", "mypy", "pyright"):
                return [Use("compile", scope="scoped")]
            if module in ("json.tool", "tomllib"):
                return [Use("assert")]
            if module in ("pip", "venv", "ensurepip"):
                return [Use("setup")]
            return [Use("exec")]
        if "-c" in args:
            idx = args.index("-c")
            code = args[idx + 1] if idx + 1 < len(args) else ""
            asserts = bool(re.search(r"\bassert\b|\braise\b|exit\(|\bSystemExit\b", code))
            return [Use("assert" if asserts else "structural")]
        script = _first_positional(args)
        if not script:
            return [Use("setup")]
        return [_script_use(script, args[args.index(script) + 1 :], cwd, task_files)]

    if prog in ("node", "deno"):
        if prog == "deno":
            sub = _first_positional(args)
            if sub == "test":
                return [Use("test", scope=scoped or ("scoped" if _has_positional(args[1:]) else "workspace"))]
            if sub == "check":
                return [Use("compile", scope="scoped")]
            args = args[args.index(sub) + 1 :] if sub == "run" else args
        if "--test" in args:
            return [Use("test", scope=scoped or ("scoped" if _has_positional(args) else "workspace"))]
        if "--check" in args or "-c" in args:
            return [Use("compile", scope="scoped")]
        for flag in ("-e", "--eval", "-p", "--print"):
            if flag in args:
                idx = args.index(flag)
                code = args[idx + 1] if idx + 1 < len(args) else ""
                asserts = bool(re.search(r"process\.exit|\bassert\b|\bthrow\b", code))
                return [Use("assert" if asserts else "structural")]
        script = _first_positional(args)
        if not script:
            return [Use("setup")]
        return [_script_use(script, args[args.index(script) + 1 :], cwd, task_files)]

    if prog == "go":
        sub = _first_positional(args)
        rest = args[args.index(sub) + 1 :] if sub else []
        pkgs = [a for a in rest if not a.startswith("-")]
        whole = not pkgs or "./..." in pkgs or "..." in pkgs
        scope = scoped or ("scoped" if "-run" in rest or not whole else "workspace")
        if sub == "test":
            return [Use("test", scope=scope)]
        if sub in ("build", "vet"):
            return [Use("compile", scope=scope)]
        return [Use("exec")]

    if prog in ("make", "just", "task"):
        target = _first_positional(args)
        tokens = _tokens(target)
        scope = scoped or "workspace"
        if tokens & _TEST_TOKENS:
            return [Use("test", scope=scope)]
        if tokens & _BUILD_TOKENS or not target:
            return [Use("compile", scope=scope)]
        return [Use("exec")]

    if prog in _ASSERTING:
        return [Use("assert")]
    if prog in _STRUCTURAL:
        return [Use("structural")]
    if prog in _SETUP:
        return [Use("setup")]
    if "/" in raw or raw.startswith("$"):
        return [_script_use(raw, args, cwd, task_files)]
    return [Use("exec")]


@dataclass
class StepAnalysis:
    cls: str  # test | run | compile | structural | vacuous
    vacuous: str | None = None
    scopes: list[str] = field(default_factory=list)  # scopes of test and compile commands
    no_run: bool = False  # classed compile because of `cargo test --no-run`
    self_exit: bool = False  # runs a program and checks only its exit code


def analyze_step(command: str, task_files: set[str] | frozenset[str] = frozenset()) -> StepAnalysis:
    """Class a verify step by the strongest thing it proves (SQ04), and flag HF2.

    ``task_files`` are the task's own outputs: running one of them and checking only its exit
    code is the "self-exit" smell, which S07 counts as structural.
    """
    cmds = parse_shell(command)
    uses: list[Use] = []
    cwd = ""
    own = set(task_files)
    for cmd in cmds:
        argv = _strip_wrappers(cmd.words)
        if argv and argv[0] == "cd":
            target = _first_positional(argv[1:])
            cwd = "" if not target or target.startswith(("$", "~", "/", "-")) else _resolve(cwd, target)
            if cwd in (".", "") or cwd.startswith(".."):
                cwd = ""
            continue
        uses.extend(_classify(cmd.words, cwd, own))

    # A run counts only when an assertion follows it (a pipe into grep, a test on its output or
    # on the state it left), or when it is a check script the task does not write.
    run = False
    seen_exec = False
    for use in uses:
        if use.kind == "exec":
            seen_exec = True
            run = run or use.harness
        elif use.kind == "assert" and seen_exec:
            run = True
    kinds = {use.kind for use in uses}
    if "test" in kinds:
        cls = "test"
    elif run:
        cls = "run"
    elif "compile" in kinds:
        cls = "compile"
    else:
        cls = "structural"
    analysis = StepAnalysis(
        cls=cls,
        scopes=[use.scope for use in uses if use.kind in ("test", "compile")],
        no_run=cls == "compile" and any(use.no_run for use in uses),
        self_exit=cls == "structural" and seen_exec,
    )
    reason = vacuous_reason(command)
    if reason:
        analysis.cls = "vacuous"
        analysis.vacuous = reason
        analysis.scopes = []
    return analysis


# --------------------------------------------------------------------------------------------
# Field access. Plans are hand-written, so every field is type-checked before use.


def _str(value: object) -> str:
    return value if isinstance(value, str) else ""


def _table(value: object) -> dict:
    return value if isinstance(value, dict) else {}


def _tables(value: object) -> list[dict]:
    return [item for item in value if isinstance(item, dict)] if isinstance(value, list) else []


def _strings(value: object) -> list[str]:
    """Non-empty trimmed strings of a list, like plan_validate.rs `string_array`."""
    if not isinstance(value, list):
        return []
    return [item.strip() for item in value if isinstance(item, str) and item.strip()]


def words(text: str) -> list[str]:
    return [token for token in text.split() if any(ch.isalnum() for ch in token)]


def prose(text: str) -> str:
    """Text without fenced code blocks and inline code spans."""
    return _CODE_SPAN_RE.sub(" ", _CODE_FENCE_RE.sub(" ", text))


def read_file_entries(task: dict) -> list[dict]:
    """`[task.context].read_files` as {path, why, lines} tables; a bare string is a path."""
    entries = []
    raw = _table(task.get("context")).get("read_files")
    for item in raw if isinstance(raw, list) else []:
        if isinstance(item, str):
            item = {"path": item}
        if isinstance(item, dict):
            entries.append(item)
    return entries


def accept_tests(task: dict) -> list[dict]:
    """The `[task.accept]` tests that compile into a verify step when a plan loads (gap-d14a43).

    An entry counts when it names a `src`, a `dest` and a `runner` and has a positive `count`: the
    loader turns it into a step that runs the pinned test and requires exactly `count` passing tests.
    """
    tests = []
    for entry in _tables(_table(task.get("accept")).get("files")):
        count = entry.get("count")
        named = all(_str(entry.get(key)).strip() for key in ("src", "dest", "runner"))
        if named and isinstance(count, int) and not isinstance(count, bool) and count > 0:
            tests.append(entry)
    return tests


def acceptance_criteria(task: dict) -> tuple[list[tuple[str, bool]], bool]:
    """(criterion, observable) pairs for SQ02, and the S07 section 3.3 has-acceptance flag."""
    criteria: list[tuple[str, bool]] = []
    for item in _strings(task.get("acceptance")):
        criteria.append((item, bool(_OBSERVABLE_RE.search(item))))
    contract = _table(task.get("acceptance_contract"))
    # Contract entries and pinned acceptance tests are machine-checked, so they are observable by
    # construction.
    for gate in _tables(contract.get("gates")):
        criteria.append(("gate " + _str(gate.get("id")), True))
    for key in ("no_stub", "agent_output", "review_verdict"):
        if contract.get(key):
            criteria.append((key, True))
    for row in _tables(_table(contract.get("parity_ledger")).get("rows")):
        criteria.append(("parity " + _str(row.get("requirement_id")), True))
    accept = accept_tests(task)
    for test in accept:
        criteria.append(("accept " + _str(test.get("src")).strip(), True))
    text = "\n".join(t for t in (_str(task.get("goal")), _str(task.get("description"))) if t)
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        match = _ACCEPTANCE_HEADER_RE.match(lines[i])
        i += 1
        if not match:
            continue
        if match.group(1).strip():
            criteria.append((match.group(1).strip(), bool(_OBSERVABLE_RE.search(match.group(1)))))
        while i < len(lines) and (bullet := _BULLET_RE.match(lines[i])):
            criteria.append((bullet.group(1), bool(_OBSERVABLE_RE.search(bullet.group(1)))))
            i += 1
    has_fields = bool(_strings(task.get("acceptance"))) or bool(contract) or bool(accept)
    return criteria, has_fields or bool(_ACCEPTANCE_PHRASE_RE.search(text))


# --------------------------------------------------------------------------------------------
# PLAN_CONTEXT_SYMBOL anchors (plan_policy.rs `explicit_symbol_anchor`, `find_anchor_line`).


def explicit_symbol_anchor(symbol: str) -> str | None:
    trimmed = symbol.strip()
    if trimmed.startswith("exact:"):
        candidate = trimmed[len("exact:") :].strip()
    elif trimmed.startswith("`"):
        rest = trimmed[1:]
        if "`" not in rest:
            return None
        candidate = rest.split("`", 1)[0].strip()
    elif "—" in trimmed:
        candidate = trimmed.split("—", 1)[0].strip()
    elif not any(ch.isspace() for ch in trimmed):
        candidate = trimmed
    else:
        return None
    if candidate.endswith("()"):
        candidate = candidate[:-2]
    candidate = candidate.strip("`,;")
    if not candidate:
        return None
    for segment in candidate.split("::"):
        if not segment or segment[0].isdigit():
            return None
        if not all(ch == "_" or (ch.isascii() and ch.isalnum()) for ch in segment):
            return None
    return candidate


def _contains_identifier(line: str, ident: str) -> bool:
    start = line.find(ident)
    while start >= 0:
        before = line[start - 1] if start > 0 else ""
        end = start + len(ident)
        after = line[end] if end < len(line) else ""
        if not _is_ident_char(before) and not _is_ident_char(after):
            return True
        start = line.find(ident, start + 1)
    return False


def _is_ident_char(ch: str) -> bool:
    return bool(ch) and (ch == "_" or (ch.isascii() and ch.isalnum()))


def find_anchor(content: str, anchor: str) -> bool:
    final = anchor.rsplit("::", 1)[-1]
    return any(_contains_identifier(line, anchor) or _contains_identifier(line, final) for line in content.splitlines())


# --------------------------------------------------------------------------------------------
# Scoring.


class Workspace:
    """The repo root: file existence (HF4), crate count (HF5) and context file text (SQ07)."""

    def __init__(self, root: Path) -> None:
        self.root = root.resolve()
        crates = self.root / "crates"
        self.crate_count = sum(1 for p in crates.iterdir() if p.is_dir()) if crates.is_dir() else 0
        self._text: dict[str, str | None] = {}

    def exists(self, rel: str) -> bool:
        return (self.root / rel).exists()

    def text(self, rel: str) -> str | None:
        if rel not in self._text:
            self._text[rel] = self._read(rel)
        return self._text[rel]

    def _read(self, rel: str) -> str | None:
        if rel.startswith("/") or "\\" in rel or any(ch in rel for ch in "*?[]"):
            return None
        path = self.root / rel
        try:
            resolved = path.resolve()
            if not resolved.is_file() or not resolved.is_relative_to(self.root):
                return None
            return resolved.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError, ValueError):
            return None


@dataclass
class PlanContext:
    workspace: Workspace
    plan_id: str
    plan_path: str  # relative to the workspace root when possible
    archived: bool
    tasks_by_id: dict[str, dict]
    plan_outputs: dict[str, set[str]]  # plan id -> files its tasks write, for depends_on_plan


def task_outputs(task: dict) -> set[str]:
    return set(_strings(task.get("files"))) | set(_strings(task.get("write_files")))


def dependency_outputs(task: dict, ctx: PlanContext) -> set[str]:
    """Files written by the task's transitive dependencies and its prerequisite plans (PLAN_031)."""
    created: set[str] = set()
    seen: set[str] = set()
    pending = list(_strings(task.get("depends_on")))
    while pending:
        dep_id = pending.pop()
        if dep_id in seen:
            continue
        seen.add(dep_id)
        dep = ctx.tasks_by_id.get(dep_id)
        if dep is not None:
            created |= task_outputs(dep)
            pending.extend(_strings(dep.get("depends_on")))
    for plan_id in _strings(task.get("depends_on_plan")):
        created |= ctx.plan_outputs.get(plan_id, set())
    return created


def band(score: float) -> str:
    if score >= 80:
        return "A"
    if score >= 60:
        return "B"
    if score >= 40:
        return "C"
    return "D"


def score_task(task: dict, ctx: PlanContext, red_on_base: str = "unknown") -> dict:
    """Score one task. ``red_on_base`` is "fail", "pass" or "unknown" (static mode)."""
    role = _str(task.get("role")).strip() or "implementer"
    title = _str(task.get("title"))
    goal = _str(task.get("goal"))
    description = _str(task.get("description"))
    acceptance = _strings(task.get("acceptance"))
    steps = _tables(task.get("verify"))
    files = task_outputs(task)
    context = _table(task.get("context"))
    rules: dict[str, float] = {}
    detail: dict[str, list[str]] = {}

    # SQ01 goal: words of goal + description; 1 if >= 40, 0.5 if 15-39, else 0.
    desc_words = len(words(goal)) + len(words(description))
    rules["SQ01"] = 1.0 if desc_words >= 40 else 0.5 if desc_words >= 15 else 0.0

    # SQ02 acceptance: the share of criteria that are observable; 0 without criteria.
    criteria, has_acceptance = acceptance_criteria(task)
    n_observable = sum(1 for _, observable in criteria if observable)
    rules["SQ02"] = n_observable / len(criteria) if criteria else 0.0

    # SQ03 traceability: the share of `acceptance` items (AC ids explicit or by position) named
    # in some verify step's `covers`.
    ac_ids = []
    for index, item in enumerate(acceptance):
        match = _AC_ID_RE.match(item)
        ac_ids.append(match.group(1) if match else f"AC{index + 1}")
    covered = {c for step in steps for c in _strings(step.get("covers"))}
    rules["SQ03"] = sum(1 for ac in ac_ids if ac in covered) / len(ac_ids) if ac_ids else 0.0

    # SQ04 verify strength: the value of the strongest step. A `[task.accept]` test becomes a step
    # that runs its pinned test and requires exactly `count` passing tests: a scoped test run,
    # listed after the task's own steps.
    analyses = [analyze_step(_str(step.get("command")), files) for step in steps]
    accept = accept_tests(task)
    classes = [a.cls for a in analyses] + ["test"] * len(accept)
    max_class = min(classes, key=CLASS_ORDER.index) if classes else "none"
    rules["SQ04"] = CLASS_VALUE.get(max_class, 0.0)

    # SQ05 specificity: test and compile runs scoped to a package, file or test name.
    scopes = [scope for a in analyses for scope in a.scopes] + ["scoped"] * len(accept)
    n_scoped = scopes.count("scoped")
    rules["SQ05"] = 0.0 if not n_scoped else 1.0 if n_scoped == len(scopes) else 0.5

    # SQ06 red on base: dynamic; unknown scores 0 and is flagged.
    rules["SQ06"] = 1.0 if red_on_base == "fail" else 0.0

    # SQ07 context: read_files, each with its own `why`, and every symbol anchored in them.
    entries = read_file_entries(task)
    deps = dependency_outputs(task, ctx)
    paths = [_str(e.get("path")).strip() for e in entries]
    n_why = sum(1 for e in entries if _str(e.get("why")).strip().lower() not in ("", "context"))
    symbols = _strings(context.get("symbols"))
    contents = [text for p in paths if p and (text := ctx.workspace.text(p)) is not None]
    deferred = any(p in deps for p in paths)
    n_anchored = 0
    for symbol in symbols:
        anchor = explicit_symbol_anchor(symbol)
        if anchor is not None and (deferred or any(find_anchor(text, anchor) for text in contents)):
            n_anchored += 1
    if not entries:
        rules["SQ07"] = 0.0
    else:
        rules["SQ07"] = 1.0 if n_why == len(entries) and n_anchored == len(symbols) else 0.5

    # SQ08 scope: `files` present and bounded; `max_loc` set.
    max_loc = task.get("max_loc")
    max_loc_set = isinstance(max_loc, int) and not isinstance(max_loc, bool) and max_loc > 0
    bounded = 1 <= len(files) <= MAX_FILES and not any(ch in f for f in files for ch in "*?[]")
    rules["SQ08"] = (float(bounded) + float(max_loc_set)) / 2

    # SQ09 non-goals: `non_goals`, `anti_patterns` or an explicit exclusion in the prose.
    has_non_goals = bool(
        _strings(task.get("non_goals"))
        or _strings(context.get("anti_patterns"))
        or _NON_GOAL_RE.search(goal + "\n" + description)
    )
    rules["SQ09"] = 1.0 if has_non_goals else 0.0

    # SQ10 vagueness: lexicon hits per 100 prose words, d; max(0, 1 - d/5).
    spec_text = "\n".join([title, goal, description, *acceptance])
    plain = prose(spec_text)
    vague = [m.group(0).lower() for m in _VAGUE_RE.finditer(plain)]
    n_words = len(words(plain))
    density = 100.0 * len(vague) / n_words if n_words else 0.0
    rules["SQ10"] = max(0.0, 1.0 - density / 5.0)

    # SQ11 anchors: a concrete file, type or function named in the spec text.
    rules["SQ11"] = 1.0 if _ANCHOR_RE.search(spec_text) else 0.0

    # SQ12 hidden hook: `[task.hidden]` with an interface.
    hidden = task.get("hidden")
    rules["SQ12"] = 1.0 if _strings(_table(hidden).get("interface")) else 0.0

    hard: list[str] = []
    if role == "implementer" and not steps and not accept:
        hard.append("HF1")
    vacuous = [f"step {i + 1}: {a.vacuous}" for i, a in enumerate(analyses) if a.vacuous]
    if vacuous:
        hard.append("HF2")
        detail["HF2"] = vacuous
    expects_red = role == "implementer" and not (
        steps and all(_str(s.get("expect")) == "pass_on_base" for s in steps)
    )
    if red_on_base == "pass" and expects_red:
        hard.append("HF3")
    missing = list(dict.fromkeys(p for p in paths if p and not ctx.workspace.exists(p) and p not in deps))
    if missing:
        hard.append("HF4")
        detail["HF4"] = missing
    if ctx.workspace.crate_count:
        claims = [
            phrase
            for text in (_str(task.get("prompt")), description, title)
            for phrase in GREENFIELD_PHRASES
            if phrase in text.lower()
        ]
        if claims:
            hard.append("HF5")
            detail["HF5"] = claims

    score = round(sum(WEIGHTS[rule] * value for rule, value in rules.items()), 2)
    canonical = json.dumps(task, sort_keys=True, ensure_ascii=False, separators=(",", ":"), default=str)
    return {
        "ev": "spec.quality",
        "linter": LINTER,
        "mode": "static" if red_on_base == "unknown" else "dynamic",
        "plan_id": ctx.plan_id,
        "plan_path": ctx.plan_path,
        "archived": ctx.archived,
        "task_id": _str(task.get("id")),
        "role": role,
        "spec_hash": "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest(),
        "spec_origin": "authored",
        "score": score,
        "band": band(score),
        "hard_fail": hard,
        "hard_fail_detail": detail,
        "unknown": list(STATIC_UNKNOWN) if red_on_base == "unknown" else [],
        "rules": {rule: round(value, 4) for rule, value in rules.items()},
        "verify_classes": classes,
        "red_on_base": red_on_base,
        "features": {
            "verify_max_class": max_class,
            "n_verify": len(steps) + len(accept),
            "n_accept": len(accept),
            "n_no_run": sum(1 for a in analyses if a.no_run),
            "n_self_exit": sum(1 for a in analyses if a.self_exit),
            "has_test_verify": "test" in classes,
            "has_acceptance": has_acceptance,
            "has_acceptance_fields": bool(acceptance) or bool(_table(task.get("acceptance_contract"))) or bool(accept),
            "n_acceptance": len(criteria),
            "n_observable": n_observable,
            "ac_coverage": round(rules["SQ03"], 4),
            "desc_words": desc_words,
            "vague_density": round(density, 4),
            "vague_terms": vague,
            "n_read_files": len(entries),
            "n_read_files_why": n_why,
            "n_symbols": len(symbols),
            "n_symbols_anchored": n_anchored,
            "n_files": len(files),
            "max_loc": max_loc if max_loc_set else None,
            "has_non_goals": has_non_goals,
            "has_hidden_hook": isinstance(hidden, dict),
            "refine_rounds": 0,
        },
        "critic": None,
        "ambiguity": None,
    }


# --------------------------------------------------------------------------------------------
# Corpus.


def find_root(start: Path) -> Path:
    """The nearest ancestor holding `.git` or `roko.toml`; the current directory otherwise."""
    for candidate in [start.resolve(), *start.resolve().parents]:
        if (candidate / ".git").exists() or (candidate / "roko.toml").exists():
            return candidate
    return Path.cwd().resolve()


def discover(paths: list[Path]) -> list[Path]:
    found: set[Path] = set()
    for path in paths:
        if path.is_file():
            found.add(path.resolve())
        elif path.is_dir():
            for candidate in path.rglob("tasks.toml"):
                rel = candidate.relative_to(path)
                if not any(part.startswith(".") for part in rel.parts[:-1]):
                    found.add(candidate.resolve())
    return sorted(found, key=lambda p: p.as_posix())


def _relative(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def lint_files(files: list[Path], root: Path, red_on_base: dict[tuple[str, str], str] | None = None) -> tuple[list[dict], list[tuple[str, str]]]:
    """Score every task of ``files``; returns (records without `ts`, parse errors)."""
    workspace = Workspace(root)
    parsed: list[tuple[Path, dict]] = []
    errors: list[tuple[str, str]] = []
    for path in files:
        try:
            with path.open("rb") as handle:
                parsed.append((path, tomllib.load(handle)))
        except (OSError, tomllib.TOMLDecodeError, UnicodeDecodeError) as err:
            errors.append((_relative(path, workspace.root), str(err)))
    plan_outputs: dict[str, set[str]] = {}
    for path, data in parsed:
        outputs = plan_outputs.setdefault(_plan_id(path, data), set())
        for task in _tables(data.get("task")):
            outputs |= task_outputs(task)
    records = []
    for path, data in parsed:
        tasks = _tables(data.get("task"))
        rel = _relative(path, workspace.root)
        ctx = PlanContext(
            workspace=workspace,
            plan_id=_plan_id(path, data),
            plan_path=rel,
            archived="archive" in PurePosixPath(rel).parts,
            tasks_by_id={_str(t.get("id")).strip(): t for t in tasks if _str(t.get("id")).strip()},
            plan_outputs=plan_outputs,
        )
        for task in tasks:
            key = (rel, _str(task.get("id")))
            records.append(score_task(task, ctx, (red_on_base or {}).get(key, "unknown")))
    return records, errors


def _plan_id(path: Path, data: dict) -> str:
    return _str(_table(data.get("meta")).get("plan")).strip() or path.parent.name


# --------------------------------------------------------------------------------------------
# Summary.


def _pct(part: int, whole: int) -> str:
    return f"{100.0 * part / whole:5.1f}%" if whole else "    - "


def _quantile(values: list[float], q: float) -> float:
    ordered = sorted(values)
    pos = q * (len(ordered) - 1)
    low, high = math.floor(pos), math.ceil(pos)
    return ordered[low] + (ordered[high] - ordered[low]) * (pos - low)


def summarize(records: list[dict], files: int, errors: list[tuple[str, str]], worst: int = 10, dynamic: bool = False) -> str:
    unknown = () if dynamic else STATIC_UNKNOWN
    groups = {
        "all": records,
        "active": [r for r in records if not r["archived"]],
        "archived": [r for r in records if r["archived"]],
    }
    names = list(groups)
    lines = [
        f"speclint {LINTER} ({'dynamic' if dynamic else 'static'}): {files} files, {len(records)} tasks "
        f"({len(groups['active'])} active, {len(groups['archived'])} archived), {len(errors)} parse errors",
    ]
    for path, err in errors:
        lines.append(f"  parse error: {path}: {err}")

    def row(label: str, cells: list[str]) -> str:
        return f"  {label:<38}" + "".join(f"{cell:>10}" for cell in cells)

    def rate(pred) -> list[str]:
        return [_pct(sum(1 for r in g if pred(r)), len(g)) for g in groups.values()]

    lines += ["", row("", names)]
    lines.append(row("tasks", [str(len(g)) for g in groups.values()]))
    for label, fn in (
        ("score mean", lambda v: sum(v) / len(v)),
        ("score median", lambda v: _quantile(v, 0.5)),
        ("score p10", lambda v: _quantile(v, 0.1)),
        ("score p90", lambda v: _quantile(v, 0.9)),
    ):
        lines.append(row(label, [f"{fn([r['score'] for r in g]):.1f}" if g else "-" for g in groups.values()]))
    for letter in "ABCD":
        lines.append(row(f"band {letter}", [str(sum(1 for r in g if r["band"] == letter)) for g in groups.values()]))

    lines += ["", row("Hard fails (tasks)", names)]
    for hf, label in HARD_FAILS.items():
        if hf in unknown:
            lines.append(row(f"{hf} {label}", ["unknown"] * 3))
        else:
            lines.append(row(f"{hf} {label}", [str(sum(1 for r in g if hf in r["hard_fail"])) for g in groups.values()]))

    lines += ["", row("Rules: mean score / full credit", names)]
    for rule, weight in WEIGHTS.items():
        cells = []
        for g in groups.values():
            if not g:
                cells.append("-")
                continue
            mean = sum(r["rules"][rule] for r in g) / len(g)
            full = 100.0 * sum(1 for r in g if r["rules"][rule] >= 1.0) / len(g)
            cells.append(f"{mean:.2f}/{full:3.0f}%")
        suffix = " (unknown)" if rule in unknown else ""
        lines.append(row(f"{rule} {RULE_NAMES[rule]} ({weight}){suffix}", cells))

    lines += ["", row("S07 section 3.3 rates", names)]
    lines.append(row("no acceptance criteria (fields/phrasing)", rate(lambda r: not r["features"]["has_acceptance"])))
    lines.append(row("no acceptance fields", rate(lambda r: not r["features"]["has_acceptance_fields"])))
    lines.append(row("strongest verify is structural", rate(lambda r: r["features"]["verify_max_class"] == "structural")))
    lines.append(row("has a test-runner verify step", rate(lambda r: r["features"]["has_test_verify"])))
    lines.append(row("no read_files", rate(lambda r: r["features"]["n_read_files"] == 0)))
    lines.append(row("hidden-test hook (tasks)", [str(sum(1 for r in g if r["features"]["has_hidden_hook"])) for g in groups.values()]))
    lines.append(row("pinned acceptance tests (tasks)", [str(sum(1 for r in g if r["features"]["n_accept"])) for g in groups.values()]))

    lines += ["", row("Verify steps by class", names)]
    for cls in CLASS_ORDER:
        lines.append(row(cls, [str(sum(r["verify_classes"].count(cls) for r in g)) for g in groups.values()]))
    lines.append(row("  compile: cargo test --no-run", [str(sum(r["features"]["n_no_run"] for r in g)) for g in groups.values()]))
    lines.append(row("  structural: self-exit runs", [str(sum(r["features"]["n_self_exit"] for r in g)) for g in groups.values()]))
    lines += ["", row("Strongest step per task", names)]
    for cls in (*CLASS_ORDER, "none"):
        lines.append(row(cls, [str(sum(1 for r in g if r["features"]["verify_max_class"] == cls)) for g in groups.values()]))

    ranked = sorted(records, key=lambda r: (r["score"], r["plan_path"], r["task_id"]))[:worst]
    lines += ["", f"Worst {len(ranked)} tasks"]
    for r in ranked:
        zero = [rule for rule, value in r["rules"].items() if value == 0 and rule not in unknown]
        hard = f" hard={','.join(r['hard_fail'])}" if r["hard_fail"] else ""
        lines.append(f"  {r['score']:5.1f} {r['band']}  {r['plan_path']} {r['task_id']}{hard}  zero: {' '.join(zero)}")
    return "\n".join(lines)


# --------------------------------------------------------------------------------------------
# CLI.


def default_out() -> Path:
    base = Path(os.environ.get("VB_RESULTS") or Path.home() / ".roko-bench" / "viability")
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return base / "speclint" / run_id / "speclint.jsonl"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="speclint.py",
        description="Score task specs (S07 SQS v1, static or dynamic mode) and write spec.quality records.",
    )
    parser.add_argument("paths", nargs="+", type=Path, help="plan directories or tasks.toml files")
    parser.add_argument("--out", help="JSONL output path; '-' for stdout (default: $VB_RESULTS/speclint/<run_id>/speclint.jsonl)")
    parser.add_argument("--root", type=Path, help="workspace root for context files (default: nearest .git or roko.toml)")
    parser.add_argument("--worst", type=int, default=10, help="how many of the lowest-scoring tasks to list")
    parser.add_argument("--strict", action="store_true", help="exit 1 when any task has a hard fail")
    parser.add_argument("--dynamic", action="store_true", help="run each implementer task's verify steps twice on a clean base checkout first, to score SQ06 and HF3 (dynamic.py)")
    parser.add_argument("--base", help="with --dynamic: the commit to check every plan against (default: HEAD for plans that have not run; none for the rest)")
    parser.add_argument("--timeout", type=float, help="with --dynamic: the most seconds a verify step may take (default: 120)")
    parser.add_argument("--scratch", type=Path, help="with --dynamic: an existing directory outside the checkout for the base checkouts (default: the system temp directory)")
    parser.add_argument("--fixture", action="store_true", help="with --dynamic: the workspace root (default: the first path) is a plain directory; check against a one-commit snapshot of it")
    args = parser.parse_args(argv)
    if not args.dynamic and (args.base or args.timeout is not None or args.scratch or args.fixture):
        parser.error("--base, --timeout, --scratch and --fixture need --dynamic")
    if args.timeout is not None and args.timeout <= 0:
        parser.error("--timeout must be positive")

    files = discover(args.paths)
    if not files:
        print("speclint: no tasks.toml under " + ", ".join(str(p) for p in args.paths), file=sys.stderr)
        return 2
    if args.root:
        root = args.root.resolve()
    elif args.fixture:
        root = (args.paths[0] if args.paths[0].is_dir() else args.paths[0].parent).resolve()
    else:
        root = find_root(args.paths[0])
    if args.dynamic:
        import dynamic  # it imports this module, so load it only when asked

        try:
            records, errors = dynamic.lint(
                files,
                root,
                base=args.base,
                timeout=args.timeout or dynamic.STEP_TIMEOUT_S,
                scratch=args.scratch,
                fixture=args.fixture,
            )
        except dynamic.CheckError as err:
            print(f"speclint: {err}", file=sys.stderr)
            return 2
    else:
        records, errors = lint_files(files, root)
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lines = [json.dumps({**record, "ts": ts}, sort_keys=True, ensure_ascii=False) for record in records]

    summary_stream = sys.stdout
    if args.out == "-":
        sys.stdout.write("\n".join(lines) + ("\n" if lines else ""))
        summary_stream = sys.stderr
    else:
        out = Path(args.out) if args.out else default_out()
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
        print(f"records: {out}", file=summary_stream)
    print(summarize(records, len(files), errors, args.worst, dynamic=args.dynamic), file=summary_stream)
    if args.dynamic:
        print("\n".join(dynamic.summary_lines(records)), file=summary_stream)
    if args.strict and any(record["hard_fail"] for record in records):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
