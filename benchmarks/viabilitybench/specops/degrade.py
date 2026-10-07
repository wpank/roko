"""The spec-degradation operator D-v1 (3233, S07 §4.5): ``D(spec, levels, seed) -> (spec', manifest)``.

H3 compares cheap and frontier models on precise specs (P) and their vague twins (V). D-v1 makes V from P by
removing what a precise TSS v1 spec states, one level at a time, and lists every change in its manifest:

- L1 drop_verify: hide every verify step from the agent. The manifest keeps them (``hidden_verify``), so they
  still score VS.
- L2 drop_acceptance: remove ``acceptance``, the verify steps' ``covers``, and every sentence that names an
  acceptance criterion (``AC1``, "the acceptance list").
- L3 drop_context: remove ``context.read_files`` and ``context.symbols``, and convention pointers in the prose
  ("see CONTRIBUTING.md", "read its --help"), whether a clause or a parenthetical.
- L4 de_anchor: replace ``path::fn`` anchors and backticked identifiers in the prose with generic nouns ("the
  file", "the function", ...).
- L5 lexical: insert k = 3 terms of speclint's vague-term lexicon v1 at seeded word positions of the
  description, so that it holds at least k.
- L6 summarize (an LLM call) is off in D-v1.

``vague`` is L1–L5 (:data:`VAGUE`); a dose-response series applies a prefix of them. The levels always apply in
the order L1 to L5, whatever order they are given in. D-v1 is deterministic given the seed, and idempotent:
applying it again with the same levels and seed changes nothing, because every level removes or replaces what it
finds and L5 tops the lexicon count up to k rather than adding k more. The input spec is never modified.
"""

from __future__ import annotations

import copy
import random
import re
import sys
from pathlib import Path

from .manifest import OPERATOR, Manifest

_SPECLINT_DIR = Path(__file__).resolve().parents[1] / "speclint"
if str(_SPECLINT_DIR) not in sys.path:
    sys.path.insert(0, str(_SPECLINT_DIR))
import speclint  # noqa: E402

LEVELS = ("L1", "L2", "L3", "L4", "L5", "L6")
VAGUE = ("L1", "L2", "L3", "L4", "L5")

# How many lexicon terms L5 makes the description hold.
LEXICAL_K = 3

# The prose fields a level rewrites, and the list fields whose items are prose.
TEXT_FIELDS = ("title", "goal", "description", "prompt")
LIST_FIELDS = ("acceptance", "non_goals")

_SENTENCE_SPLIT = re.compile(r"(?<=[.!?])\s+")
_AC_SENTENCE = re.compile(r"\bAC\d*\b|\bacceptance\b", re.IGNORECASE)
_POINTER = re.compile(
    r"\b(?:see|read|follow|consult)\b[^;.()]*?"
    r"(?:\bconventions?\b|CONTRIBUTING|\.md\b|--help\b|\bdocstrings?\b|\bguide\b|\bdocs?\b)"
    r"|--help\b",
    re.IGNORECASE,
)
_PARENTHETICAL = re.compile(r"\s*\(([^()]*)\)")
_BACKTICKED = re.compile(r"`([^`]+)`")
_BARE_PATH_FN = re.compile(r"\b[\w./-]+::\w+(?:\([^)]*\))?")
_CAMEL = re.compile(r"^[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]*)+$")


def degrade(spec: dict, levels, seed: int) -> tuple[dict, Manifest]:
    """Apply D-v1's ``levels`` to the task ``spec`` (a ``[[task]]`` table) with ``seed``.

    Returns the degraded copy and its :class:`Manifest`. Raises ``ValueError`` for an unknown level or for L6,
    which D-v1 leaves off.
    """
    chosen = sorted(set(levels))
    for level in chosen:
        if level not in LEVELS:
            raise ValueError(f"unknown level {level!r}; D-v1 has {', '.join(LEVELS)}")
    if "L6" in chosen:
        raise ValueError("L6 (an LLM summary) is off in D-v1")
    task_id = str(spec.get("id", ""))
    out = copy.deepcopy(spec)
    manifest = Manifest(task_id=task_id, levels=chosen, seed=seed)
    steps = {"L1": _drop_verify, "L2": _drop_acceptance, "L3": _drop_context, "L4": _de_anchor}
    for level in chosen:
        if level == "L5":
            _lexical(out, manifest, random.Random(f"{OPERATOR}:{seed}:{task_id}"))
        else:
            steps[level](out, manifest)
    return out, manifest


def _drop_verify(spec: dict, manifest: Manifest) -> None:
    for step in spec.pop("verify", None) or []:
        manifest.removed.append({"level": "L1", "field": "verify", "item": copy.deepcopy(step)})
        manifest.hidden_verify.append(copy.deepcopy(step))


def _drop_acceptance(spec: dict, manifest: Manifest) -> None:
    for item in spec.pop("acceptance", None) or []:
        manifest.removed.append({"level": "L2", "field": "acceptance", "item": item})
    for index, step in enumerate(spec.get("verify") or []):
        if isinstance(step, dict) and "covers" in step:
            covers = step.pop("covers")
            manifest.removed.append({"level": "L2", "field": f"verify[{index}].covers", "item": covers})
    for name in TEXT_FIELDS:
        _drop_sentences(spec, name, "L2", _AC_SENTENCE, manifest)


def _drop_context(spec: dict, manifest: Manifest) -> None:
    context = spec.get("context")
    if isinstance(context, dict):
        for key in ("read_files", "symbols"):
            for item in context.pop(key, None) or []:
                manifest.removed.append({"level": "L3", "field": f"context.{key}", "item": item})
        if not context:
            spec.pop("context")
    for name in TEXT_FIELDS:
        text = spec.get(name)
        if not isinstance(text, str):
            continue
        kept = _PARENTHETICAL.sub(lambda match: _drop_pointer(match, name, manifest), text)
        if kept != text:
            spec[name] = kept
        _drop_sentences(spec, name, "L3", _POINTER, manifest, clauses=True)


def _drop_pointer(match: re.Match, name: str, manifest: Manifest) -> str:
    if not _POINTER.search(match.group(1)):
        return match.group(0)
    manifest.removed.append({"level": "L3", "field": name, "item": match.group(0).strip()})
    return ""


def _drop_sentences(spec: dict, name: str, level: str, pattern: re.Pattern, manifest: Manifest, clauses: bool = False) -> None:
    """Remove the sentences of prose field ``name`` that match ``pattern``; with ``clauses``, the matching
    ``;``-separated clauses of a sentence instead."""
    text = spec.get(name)
    if not isinstance(text, str):
        return
    kept_sentences = []
    changed = False
    for sentence in _SENTENCE_SPLIT.split(text.strip()):
        parts = sentence.split("; ") if clauses else [sentence]
        kept = [part for part in parts if not pattern.search(part)]
        for part in parts:
            if part not in kept:
                manifest.removed.append({"level": level, "field": name, "item": part})
        if len(kept) < len(parts):
            changed = True
        if kept:
            joined = "; ".join(kept)
            if clauses and not joined.endswith((".", "!", "?")) and sentence.endswith((".", "!", "?")):
                joined += sentence[-1]
            kept_sentences.append(joined)
    if changed:
        spec[name] = " ".join(kept_sentences)


def _generic_noun(anchor: str) -> str:
    """The generic noun L4 puts in place of an anchor."""
    if "::" in anchor:
        last = anchor.split("::")[-1].split("(")[0]
        return "the type" if _CAMEL.match(last) else "the function"
    if " " in anchor.strip():
        return "the command"
    if anchor.startswith("-"):
        return "the option"
    if "/" in anchor or re.search(r"\.\w{1,4}$", anchor):
        return "the file"
    if _CAMEL.match(anchor):
        return "the type"
    return "the identifier"


def _de_anchor_text(text: str, name: str, manifest: Manifest) -> str:
    def backticked(match: re.Match) -> str:
        noun = _generic_noun(match.group(1))
        manifest.replaced.append({"level": "L4", "field": name, "before": match.group(0), "after": noun})
        return noun

    def bare(match: re.Match) -> str:
        noun = _generic_noun(match.group(0))
        manifest.replaced.append({"level": "L4", "field": name, "before": match.group(0), "after": noun})
        return noun

    return _BARE_PATH_FN.sub(bare, _BACKTICKED.sub(backticked, text))


def _de_anchor(spec: dict, manifest: Manifest) -> None:
    for name in TEXT_FIELDS:
        text = spec.get(name)
        if isinstance(text, str):
            spec[name] = _de_anchor_text(text, name, manifest)
    for name in LIST_FIELDS:
        items = spec.get(name)
        if isinstance(items, list):
            spec[name] = [
                _de_anchor_text(item, f"{name}[{index}]", manifest) if isinstance(item, str) else item
                for index, item in enumerate(items)
            ]


def _lexical(spec: dict, manifest: Manifest, rng: random.Random) -> None:
    name = next((name for name in ("description", "goal", "title") if isinstance(spec.get(name), str) and spec[name].strip()), None)
    if name is None:
        return
    words = spec[name].split()
    missing = LEXICAL_K - len(speclint._VAGUE_RE.findall(spec[name]))
    if missing <= 0:
        return
    terms = rng.sample(sorted(speclint.VAGUE_TERMS), missing)
    positions = sorted(rng.sample(range(1, len(words) + 1), min(missing, len(words))))
    # A term of several words ("as needed") goes in word by word, so each recorded position is a word index of the
    # result.
    offset = 0
    for position, term in zip(positions, terms):
        at = position + offset
        words[at:at] = term.split()
        offset += len(term.split())
        manifest.inserted.append({"level": "L5", "field": name, "term": term, "position": at})
    spec[name] = " ".join(words)
