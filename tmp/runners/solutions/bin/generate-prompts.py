#!/usr/bin/env python3
"""
generate-prompts.py — convert tmp/solutions/roko/tasks/*.md into mechanical
runner prompts under tmp/runners/solutions/prompts/.

Also writes:
  - batches.toml (one [[batch]] per task with deps + scope)
  - ISSUE-TRACKER.md (every task as a [ ] checkbox grouped by phase/file)
  - STATUS.md (counts per phase, computed from tracker state)

Idempotent: re-running overwrites all generated artifacts. Hand-edits to
files under prompts/ or to batches.toml are LOST. Edit the source under
tmp/solutions/roko/tasks/ instead, or extend this script.

Usage:
  python3 bin/generate-prompts.py              # full regenerate
  python3 bin/generate-prompts.py --check      # dry-run; report drift only
  python3 bin/generate-prompts.py --only STAB  # regenerate one prefix
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

# ---------------------------------------------------------------------------
# Layout
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parents[3].parent  # .../roko/
SOURCE_DIR = REPO_ROOT / "tmp" / "solutions" / "roko" / "tasks"
RUNNER_DIR = Path(__file__).resolve().parents[1]
PROMPTS_DIR = RUNNER_DIR / "prompts"
BATCHES_TOML = RUNNER_DIR / "batches.toml"
TRACKER_MD = RUNNER_DIR / "ISSUE-TRACKER.md"
STATUS_MD = RUNNER_DIR / "STATUS.md"

# ---------------------------------------------------------------------------
# Prefix table — maps source file 01-..19-.. to a 4-letter runner prefix
# ---------------------------------------------------------------------------

PREFIX_TABLE: list[tuple[str, str, str, int]] = [
    # (source-filename-pattern, prefix, phase-label, phase-num)
    ("01-STABILITY-AND-FIXES.md",     "STAB", "Phase 0: Stability",                0),
    ("02-ORCHESTRATION.md",           "ORCH", "Phase 1: Orchestration",            1),
    ("03-INFERENCE-DISPATCH.md",      "DISP", "Phase 1: Dispatch",                 1),
    ("04-GATE-PIPELINE.md",           "GATE", "Phase 1: Gate pipeline",            1),
    ("05-GATE-EVOLUTION.md",          "EVAL", "Phase 2/3: Gate evolution",         2),
    ("06-PROMPT-ASSEMBLY.md",         "PROM", "Phase 2: Prompt assembly",          2),
    ("07-LEARNING-FEEDBACK.md",       "LERN", "Phase 0/3: Learning feedback",      0),
    ("08-UX-CLI.md",                  "UX__", "Phase 0/2: UX & CLI",               0),
    ("09-ACP-MCP.md",                 "ACPM", "Phase 4: ACP & MCP",                4),
    ("10-PERFORMANCE.md",             "PERF", "Phase 3: Performance",              3),
    ("11-INNOVATIONS.md",             "INNO", "Phase 3: Innovations",              3),
    ("12-CODE-DEBT.md",               "DEBT", "Phase 1: Code debt",                1),
    ("13-GTM-AND-INTEGRATIONS.md",    "GTM_", "Phase 4: GTM",                      4),
    ("14-RUNNER-PATTERNS.md",         "RNNR", "Phase 2/3: Runner patterns",        2),
    ("15-TESTING-VERIFICATION.md",    "TEST", "Phase 4: Testing",                  4),
    ("16-CONFIG-AND-WIRING.md",       "CONF", "Phase 0/1: Config & wiring",        0),
    ("17-SAFETY-SECURITY.md",         "SAFE", "Phase 4: Safety",                   4),
    ("18-OBSERVABILITY.md",           "OBS_", "Phase 3: Observability",            3),
    ("19-CROSS-CUTTING.md",           "XCUT", "Phase 1/2: Cross-cutting",          1),
]


def prefix_for(source_filename: str) -> tuple[str, str, int]:
    for fname, prefix, phase_label, phase_num in PREFIX_TABLE:
        if source_filename == fname:
            return prefix, phase_label, phase_num
    raise ValueError(f"unknown source file: {source_filename}")


# ---------------------------------------------------------------------------
# Source task model
# ---------------------------------------------------------------------------


@dataclass
class Task:
    source_file: str
    source_num: str           # "1.07"
    title: str
    priority: str             # "P0".."P3" or "??"
    effort: str               # "1 hour", "30 min", "med", etc.
    write_scope: list[str]    # workspace-relative paths
    deps_source_nums: list[str]  # ["1.05", "1.06"]
    context: str
    implementation: str
    design: str               # may be empty
    verification_criteria: list[str]  # raw checklist items (without [ ])
    body_lines: list[str] = field(default_factory=list)  # raw lines for fallback

    @property
    def batch_id(self) -> str:
        prefix, _, _ = prefix_for(self.source_file)
        # source_num is like "1.07" -> strip the major (file index) and use suffix
        _, minor = self.source_num.split(".", 1)
        return f"{prefix}_{int(minor):02d}"

    @property
    def tracker_anchor(self) -> str:
        return self.batch_id.lower().replace("_", "-")


# ---------------------------------------------------------------------------
# Parsers
# ---------------------------------------------------------------------------

# Three task header formats observed across the source files:
#
#   Format A (files 01..09, 11..15, 17..19): "### Task 1.07: Title"
#   Format B (file 10):                       "### Task 10.6 -- Title"
#   Format C (file 16):                       "## Task 16.1: Title"
#
# Task numbers may be N.N, N.NN, or even N.N.N. Sub-section markers also
# vary: most files use h4 (#### Context, #### Implementation Steps,
# #### Verification Criteria), file 10 uses bold markers (**What:**, **Steps:**,
# **Acceptance:**), and file 16 uses h3 (### Problem, ### Fix, ### Acceptance).

TASK_HEADER_RE = re.compile(
    r"^(#{2,3})\s+Task\s+([0-9]+(?:\.[0-9]+)+)\s*[:\-]+\s+(.+?)\s*$",
    re.MULTILINE,
)


def split_tasks(file_text: str) -> list[tuple[str, str, str]]:
    """
    Returns list of (source_num, title, body_text). body_text is everything
    between this header and the next task header or EOF.
    """
    headers: list[tuple[int, str, str, str]] = []  # start, level, num, title
    for m in TASK_HEADER_RE.finditer(file_text):
        headers.append((m.start(), m.group(1), m.group(2), m.group(3)))

    tasks: list[tuple[str, str, str]] = []
    for i, (start, _level, num, title) in enumerate(headers):
        end = headers[i + 1][0] if i + 1 < len(headers) else len(file_text)
        body = file_text[start:end]
        body = body.split("\n", 1)[1] if "\n" in body else ""
        tasks.append((num, title, body))
    return tasks


# Three meta-line variants are seen:
#   **Effort**: value      (colon outside bold) — files 01..09 etc.
#   **Effort:** value      (colon inside bold)  — files 10, 16
#   **Effort:**  followed by bullets on next lines — file 10
META_FIELD_RE = re.compile(
    r"^\s*\*\*\s*([A-Za-z][A-Za-z _]*?)\s*\*\*\s*:?\s*(.*)$"
    r"|^\s*\*\*\s*([A-Za-z][A-Za-z _]*?)\s*:\s*\*\*\s*(.*)$",
    re.MULTILINE,
)
LIST_ITEM_RE = re.compile(r"^\s*-\s+(.+?)\s*$", re.MULTILINE)
# Match h3 OR h4 sub-section headers (file 16 uses h3, others h4).
SECTION_RE = re.compile(r"^#{3,4}\s+(.+?)\s*$", re.MULTILINE)


def extract_meta(body: str) -> dict[str, str]:
    """Extract Priority / Estimated Effort / Depends On markers.

    Considers only the prelude before the first sub-section header so we
    don't pick up `**Files:**` blocks that belong to a sub-section in
    file 10's format.
    """
    meta: dict[str, str] = {}
    head = SECTION_RE.split(body, maxsplit=1)[0]
    for m in META_FIELD_RE.finditer(head):
        key = (m.group(1) or m.group(3) or "").strip().lower()
        val = (m.group(2) or m.group(4) or "").strip()
        if key:
            meta[key] = val
    return meta


def extract_list_after_marker(body: str, marker: str) -> list[str]:
    """
    Find a line starting with `**marker**:` (or `**marker:**`) and grab the
    following bullet list (until blank line or next bold field).
    """
    lines = body.splitlines()
    result: list[str] = []
    capturing = False
    marker_re = re.compile(
        rf"^\s*\*\*{re.escape(marker)}\*\*\s*:?\s*$|^\s*\*\*{re.escape(marker)}:\*\*\s*$",
        re.IGNORECASE,
    )
    inline_re = re.compile(
        rf"^\s*\*\*{re.escape(marker)}\*\*\s*:?\s*(.+)$|^\s*\*\*{re.escape(marker)}:\*\*\s*(.+)$",
        re.IGNORECASE,
    )
    for ln in lines:
        if not capturing:
            if marker_re.match(ln):
                capturing = True
                continue
            inline = inline_re.match(ln)
            if inline:
                val = (inline.group(1) or inline.group(2) or "").strip()
                if val.lower() not in {"none", "n/a", ""} and not val.startswith("see "):
                    result.append(val)
                return [s for s in result if s]
            continue
        if not ln.strip():
            break
        m = LIST_ITEM_RE.match(ln)
        if m:
            result.append(m.group(1).strip())
        elif ln.lstrip().startswith("**"):
            break
        else:
            if result:
                result[-1] += " " + ln.strip()
    return [s for s in result if s]


def extract_section(body: str, names: list[str] | str) -> str:
    """Return the body of the first matching `### name` or `#### name`
    sub-section. `names` is a list of synonyms to try in order."""
    if isinstance(names, str):
        names = [names]
    for name in names:
        pat = re.compile(
            rf"^#{{3,4}}\s+{re.escape(name)}\s*$\n(.*?)(?=^#{{2,4}}\s|\Z)",
            re.MULTILINE | re.DOTALL | re.IGNORECASE,
        )
        m = pat.search(body)
        if m:
            return m.group(1).strip()
    # Fallback: bold-marker section (file 10): `**Name:**` ... up to next bold marker / blank line.
    for name in names:
        pat = re.compile(
            rf"^\*\*{re.escape(name)}\*\*\s*:?\s*$\n?(.*?)(?=^\*\*[A-Za-z][^*]*\*\*\s*:?\s*$|\Z)",
            re.MULTILINE | re.DOTALL | re.IGNORECASE,
        )
        m = pat.search(body)
        if m:
            return m.group(1).strip()
        pat2 = re.compile(
            rf"^\*\*{re.escape(name)}:?\*\*\s*(.+?)(?=^\*\*[A-Za-z][^*]*\*\*\s*:?\s*|\Z)",
            re.MULTILINE | re.DOTALL | re.IGNORECASE,
        )
        m = pat2.search(body)
        if m:
            return m.group(1).strip()
    return ""


def normalise_path(p: str) -> str:
    """Strip the absolute prefix and leading/trailing junk."""
    p = p.strip().rstrip(",.")
    p = re.sub(r"^[`'\"]|[`'\"]$", "", p)
    abs_prefix = "/Users/will/dev/nunchi/roko/roko/"
    if p.startswith(abs_prefix):
        p = p[len(abs_prefix):]
    # strip parenthetical descriptions like "(line 236)"
    p = re.split(r"\s+\(", p, 1)[0].strip()
    p = p.rstrip("`'\"")
    return p


def parse_files_to_modify(body: str) -> list[str]:
    """Extract a write-scope list from any of:
        **Files to Modify**:    (format A — most files)
        **Files:**              (format B — file 10)
        ### Files               (format C — file 16)
    """
    raw = extract_list_after_marker(body, "Files to Modify")
    if not raw:
        raw = extract_list_after_marker(body, "Files")
    if not raw:
        section = extract_section(body, ["Files", "Files to Modify"])
        if section:
            for ln in section.splitlines():
                m = LIST_ITEM_RE.match(ln)
                if m:
                    raw.append(m.group(1).strip())

    paths: list[str] = []
    seen: set[str] = set()
    for item in raw:
        # An item may have backticks or trailing notes: "`crates/…/foo.rs` (lines 12-14)"
        m = re.search(r"`([^`]+)`", item)
        cand = m.group(1) if m else item
        cand = normalise_path(cand)
        if not cand or cand.startswith("none"):
            continue
        # Drop entries that aren't file-like (e.g. "see also …")
        if "/" not in cand and not cand.endswith((".rs", ".toml", ".md", ".sh", ".py", ".json", ".yaml", ".yml")):
            continue
        if cand in seen:
            continue
        seen.add(cand)
        paths.append(cand)
    return paths


DEP_NUM_RE = re.compile(r"\b(\d+\.\d+(?:\.\d+)?)\b")


def parse_deps(body: str, current_num: str) -> list[str]:
    head = SECTION_RE.split(body, maxsplit=1)[0]
    m = re.search(r"\*\*Depends On\*\*\s*:\s*(.+?)$", head, re.MULTILINE)
    if not m:
        return []
    raw = m.group(1).strip()
    if raw.lower() in {"none", "n/a", "-", ""}:
        return []
    found = DEP_NUM_RE.findall(raw)
    return [n for n in found if n != current_num]


def parse_priority(meta: dict[str, str]) -> str:
    raw = meta.get("priority", meta.get("priority ", "??")).strip()
    m = re.search(r"P\d", raw, re.IGNORECASE)
    return m.group(0).upper() if m else "??"


def parse_effort(meta: dict[str, str]) -> str:
    raw = (
        meta.get("estimated effort")
        or meta.get("effort")
        or meta.get("estimated_effort")
        or "?"
    ).strip()
    return raw or "?"


def parse_verification(body: str) -> list[str]:
    """Pull the acceptance/verification list. Accepts both checkbox-style
    (`- [ ] x`) and plain bullets (`- x`). File 16's `### Acceptance` and
    file 10's `**Acceptance:**` use plain bullets."""
    block = extract_section(
        body,
        ["Verification Criteria", "Acceptance", "Acceptance Criteria"],
    )
    items: list[str] = []
    for ln in block.splitlines():
        m = re.match(r"\s*-\s*\[\s*\]\s*(.+?)\s*$", ln)
        if m:
            items.append(m.group(1).strip())
            continue
        m = re.match(r"\s*-\s+(.+?)\s*$", ln)
        if m:
            items.append(m.group(1).strip())
    return items


def parse_context(body: str) -> str:
    return extract_section(body, ["Context", "Problem", "Background", "What"])


def parse_implementation(body: str) -> str:
    return extract_section(
        body,
        ["Implementation Steps", "Steps", "Fix", "Implementation"],
    )


def parse_design(body: str) -> str:
    return extract_section(body, ["Design Guidance", "Design", "Notes", "Design Notes"])


def parse_deps_meta(body: str) -> list[str]:
    """File 10 puts `**Depends on:**` at the END of the task body, so we
    need to scan the whole body for it, not just the prelude."""
    nums: list[str] = []
    for m in re.finditer(
        r"\*\*Depends\s*on\*\*\s*:?\s*(.+?)(?:\n|$)|\*\*Depends\s*on:\*\*\s*(.+?)(?:\n|$)",
        body,
        re.IGNORECASE,
    ):
        raw = (m.group(1) or m.group(2) or "").strip()
        if raw.lower() in {"none", "n/a", "-", "nothing", ""}:
            continue
        nums.extend(DEP_NUM_RE.findall(raw))
    return nums


# ---------------------------------------------------------------------------
# Source -> Task list
# ---------------------------------------------------------------------------


def parse_source_file(path: Path) -> list[Task]:
    text = path.read_text()
    tasks: list[Task] = []
    for source_num, title, body in split_tasks(text):
        meta = extract_meta(body)
        # Combine prelude and meta-anywhere deps so file 10's trailing
        # `**Depends on:**` is honoured.
        deps_anywhere = parse_deps_meta(body)
        deps_prelude = parse_deps(body, source_num)
        deps = list(dict.fromkeys(deps_prelude + deps_anywhere))
        task = Task(
            source_file=path.name,
            source_num=source_num,
            title=title.strip(),
            priority=parse_priority(meta),
            effort=parse_effort(meta),
            write_scope=parse_files_to_modify(body),
            deps_source_nums=[d for d in deps if d != source_num],
            context=parse_context(body),
            implementation=parse_implementation(body),
            design=parse_design(body),
            verification_criteria=parse_verification(body),
            body_lines=body.splitlines(),
        )
        tasks.append(task)
    return tasks


def collect_all_tasks() -> list[Task]:
    tasks: list[Task] = []
    for fname, _, _, _ in PREFIX_TABLE:
        path = SOURCE_DIR / fname
        if not path.exists():
            print(f"[warn] missing source: {path}", file=sys.stderr)
            continue
        tasks.extend(parse_source_file(path))
    return tasks


def task_index_by_source_num(tasks: Iterable[Task]) -> dict[str, Task]:
    return {t.source_num: t for t in tasks}


# ---------------------------------------------------------------------------
# Emit prompt
# ---------------------------------------------------------------------------


PROMPT_TEMPLATE = """\
# {batch_id}: {title}

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#{anchor}`](../ISSUE-TRACKER.md#{anchor})
- Source: `tmp/solutions/roko/tasks/{source_file}` — Task {source_num}
- Priority: **{priority}**
- Effort: {effort}
- Depends on: {deps_str}

When this batch lands, the commit message MUST contain the trailer:

```
tracker: {batch_id} done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

{context}

## Exact Changes

{implementation}

{design_block}\
## Write Scope

{write_scope_block}

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/{source_file}` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

{verify_checklist}\

## Verify Recipe

```bash
{verify_recipe}
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: {batch_id} done"
```

## Acceptance Criteria

{acceptance_block}\

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
"""


def render_deps(deps_source_nums: list[str], src_num_index: dict[str, Task]) -> str:
    if not deps_source_nums:
        return "**none**"
    parts: list[str] = []
    for n in deps_source_nums:
        dep = src_num_index.get(n)
        if dep:
            parts.append(f"`{dep.batch_id}` (source {n})")
        else:
            parts.append(f"`{n}` (source not found in this generation)")
    return ", ".join(parts)


def render_write_scope(scope: list[str]) -> str:
    if not scope:
        return "_None — this is a documentation/verification-only batch._"
    return "\n".join(f"- `{p}`" for p in scope)


def render_verify_recipe(task: Task) -> str:
    crit = [
        c
        for c in task.verification_criteria
        if not any(x in c for x in ("cargo clippy", "cargo test", "cargo check", "cargo build", "cargo fmt"))
    ]
    body_lines = ["# Spot-check with ripgrep / git on the touched files."]
    body_lines.append("# Do NOT run cargo — the merge-back pipeline does that.")
    body_lines.append("git diff --stat")
    return "\n".join(body_lines)


def render_verify_checklist(task: Task) -> str:
    """Render the Verification Criteria as a `[ ]` checklist that the
    agent can self-tick before committing."""
    crit = [
        c
        for c in task.verification_criteria
        if not any(x in c for x in ("cargo clippy", "cargo test", "cargo check", "cargo build", "cargo fmt"))
    ]
    if not crit:
        return (
            "- [ ] The change matches the Implementation Steps above.\n"
            "- [ ] No files outside Write Scope were touched.\n"
        )
    return "\n".join(f"- [ ] {c}" for c in crit) + "\n"


def render_acceptance(task: Task) -> str:
    items = ["All Verify checkboxes pass on inspection."]
    items.extend(c for c in task.verification_criteria if "cargo" not in c)
    items.extend(
        [
            "No files outside the Write Scope are modified.",
            "Commit message contains `tracker: {bid} done` trailer.".format(bid=task.batch_id),
            "Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.",
        ]
    )
    seen: set[str] = set()
    out: list[str] = []
    for it in items:
        key = it.strip().lower()
        if key in seen:
            continue
        seen.add(key)
        out.append(f"- {it}")
    return "\n".join(out) + "\n"


def render_prompt(task: Task, src_num_index: dict[str, Task]) -> str:
    deps_str = render_deps(task.deps_source_nums, src_num_index)
    write_scope_block = render_write_scope(task.write_scope)
    design_block = ""
    if task.design.strip():
        design_block = f"## Design Guidance\n\n{task.design}\n\n"
    verify_recipe = render_verify_recipe(task)
    verify_checklist = render_verify_checklist(task)
    acceptance_block = render_acceptance(task)

    return PROMPT_TEMPLATE.format(
        batch_id=task.batch_id,
        title=task.title,
        anchor=task.tracker_anchor,
        source_file=task.source_file,
        source_num=task.source_num,
        priority=task.priority,
        effort=task.effort,
        deps_str=deps_str,
        context=task.context.strip() or "_(no context section in source)_",
        implementation=task.implementation.strip() or "_(no implementation section in source — read source task)_",
        design_block=design_block,
        write_scope_block=write_scope_block,
        verify_checklist=verify_checklist,
        verify_recipe=verify_recipe,
        acceptance_block=acceptance_block,
    )


# ---------------------------------------------------------------------------
# Emit batches.toml
# ---------------------------------------------------------------------------


def render_batches_toml(tasks: list[Task], src_num_index: dict[str, Task]) -> str:
    lines: list[str] = [
        "# =============================================================================",
        "# solutions runner — batches.toml",
        "# Generated by tmp/runners/solutions/bin/generate-prompts.py — DO NOT HAND-EDIT",
        f"# {len(tasks)} batches across {len(PREFIX_TABLE)} source files",
        "# =============================================================================",
        "",
    ]
    by_prefix: dict[str, list[Task]] = defaultdict(list)
    for t in tasks:
        prefix, _, _ = prefix_for(t.source_file)
        by_prefix[prefix].append(t)

    for fname, prefix, phase_label, _ in PREFIX_TABLE:
        bucket = by_prefix.get(prefix, [])
        if not bucket:
            continue
        lines.extend(
            [
                "# -----------------------------------------------------------------------------",
                f"# {prefix} — {phase_label} ({len(bucket)} batches)",
                f"# Source: tmp/solutions/roko/tasks/{fname}",
                "# -----------------------------------------------------------------------------",
                "",
            ]
        )
        for t in bucket:
            deps = []
            for n in t.deps_source_nums:
                dep = src_num_index.get(n)
                if dep:
                    deps.append(dep.batch_id)
            scope_toml = ", ".join(f'"{p}"' for p in t.write_scope) or ""
            deps_toml = ", ".join(f'"{d}"' for d in deps) or ""
            lines.extend(
                [
                    "[[batch]]",
                    f'id = "{t.batch_id}"',
                    f'title = "{toml_escape(t.title)}"',
                    f'group = "{prefix}"',
                    f'priority = "{t.priority}"',
                    f'effort = "{toml_escape(t.effort)}"',
                    f'source_file = "tmp/solutions/roko/tasks/{t.source_file}"',
                    f'source_num = "{t.source_num}"',
                    f"deps = [{deps_toml}]",
                    f"scope = [{scope_toml}]",
                    'verify = "quick"',
                    "",
                ]
            )
    return "\n".join(lines)


def toml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


# ---------------------------------------------------------------------------
# Emit ISSUE-TRACKER.md
# ---------------------------------------------------------------------------

TRACKER_ROW_RE = re.compile(
    r'\| <a id="[^"]+"></a> (\[\s*\]|\[x\]|\[~\]) `([A-Z][A-Z0-9_]+_\d+)`'
    r'(?:\s+<!--\s*(.+?)\s*-->)?',
)


def load_tracker_state(path: Path) -> dict[str, tuple[str, str | None]]:
    """Parse existing ISSUE-TRACKER.md for per-batch checkbox state.

    Returns ``batch_id -> (status, optional_note)`` where status is one of
    ``"open"``, ``"done"``, ``"progress"``.
    """
    if not path.exists():
        return {}
    text = path.read_text()
    out: dict[str, tuple[str, str | None]] = {}
    for m in TRACKER_ROW_RE.finditer(text):
        box, bid, note = m.group(1), m.group(2), m.group(3)
        note = note.strip() if note else None
        if box == "[x]":
            out[bid] = ("done", note)
        elif box == "[~]":
            out[bid] = ("progress", note)
        else:
            out[bid] = ("open", note)
    return out


def format_row_checkbox(
    bid: str, anchor: str, state: dict[str, tuple[str, str | None]]
) -> str:
    st, note = state.get(bid, ("open", None))
    suffix = f"  <!-- {note} -->" if note else ""
    if st == "done":
        return f'| <a id="{anchor}"></a> [x] `{bid}`{suffix} '
    if st == "progress":
        return f'| <a id="{anchor}"></a> [~] `{bid}`{suffix} '
    return f'| <a id="{anchor}"></a> [ ] `{bid}` '


TRACKER_HEADER = """\
# Solutions Runner — Issue Tracker

> Auto-generated by `bin/generate-prompts.py`. Re-run after editing source.
> **Checkbox state** (`[ ]`, `[~]`, `[x]`, and `<!-- ... -->` tails on rows)
> is **preserved** from the previous `ISSUE-TRACKER.md` when you regenerate.
> Titles, priorities, and batch IDs always follow the source task files.
> Use `bin/sync-tracker.py` to flip rows from commit trailers after merges.

This is the comprehensive checklist of every task in the
`tmp/solutions/roko/` plan corpus. Each row maps 1:1 to a prompt under
`prompts/<BATCH_ID>.prompt.md` and a `[[batch]]` in `batches.toml`.

**Status legend**: `[ ]` open · `[~]` in progress · `[x]` landed.

When a batch lands, ensure the commit message has trailer
`tracker: <BATCH_ID> done <sha>`. Then run `python3 bin/sync-tracker.py --apply`
to flip the checkbox here.

---

## How to find a row

- By batch ID: search for `<a id="<batch-id-lowercase>">`
  (e.g. `<a id="stab-07">`).
- By source: rows are grouped under `## <Prefix> — <Phase>` matching
  `tmp/solutions/roko/tasks/<NN>-...md`.
- By phase: see `STATUS.md` for current per-phase counts.

---

"""


def render_tracker(
    tasks: list[Task],
    state: dict[str, tuple[str, str | None]],
) -> tuple[str, list[tuple[str, str, int, int]]]:
    """Return (markdown, status_rows) where each status row is
    ``(prefix, phase_label, open_count, total_count)``.
    """
    out: list[str] = [TRACKER_HEADER]

    by_prefix: dict[str, list[Task]] = defaultdict(list)
    for t in tasks:
        prefix, _, _ = prefix_for(t.source_file)
        by_prefix[prefix].append(t)

    status_rows: list[tuple[str, str, int, int]] = []

    # Aggregate counts at the top
    out.append("## Summary\n")
    out.append("| Prefix | Phase | Source file | Open | Total |")
    out.append("|---|---|---|---:|---:|")
    grand_open = 0
    grand_total = 0
    for fname, prefix, phase_label, _ in PREFIX_TABLE:
        bucket = by_prefix.get(prefix, [])
        if not bucket:
            continue
        total = len(bucket)
        open_n = sum(
            1 for t in bucket if state.get(t.batch_id, ("open", None))[0] != "done"
        )
        status_rows.append((prefix, phase_label, open_n, total))
        grand_open += open_n
        grand_total += total
        out.append(
            f"| `{prefix}` | {phase_label} | `tmp/solutions/roko/tasks/{fname}` | {open_n} | {total} |"
        )
    out.append(f"| **Total** | | | **{grand_open}** | **{grand_total}** |")
    out.append("")

    # Per-prefix sections
    for fname, prefix, phase_label, _ in PREFIX_TABLE:
        bucket = by_prefix.get(prefix, [])
        if not bucket:
            continue
        out.append(f"## {prefix} — {phase_label}\n")
        out.append(f"Source: `tmp/solutions/roko/tasks/{fname}` ({len(bucket)} tasks)\n")
        out.append("| ID | Pri | Effort | Title | Deps |")
        out.append("|---|---|---|---|---|")
        for t in bucket:
            deps = []
            for n in t.deps_source_nums:
                # Render dep as batch_id if available
                dep_b = next(
                    (
                        f"`{x.batch_id}`"
                        for x in tasks
                        if x.source_num == n
                    ),
                    f"`{n}`",
                )
                deps.append(dep_b)
            deps_str = ", ".join(deps) or "—"
            anchor = t.tracker_anchor
            cell = format_row_checkbox(t.batch_id, anchor, state)
            row = (
                f"{cell}"
                f"| {t.priority} | {esc(t.effort)} "
                f"| {esc(t.title)} | {deps_str} |"
            )
            out.append(row)
        out.append("")

    return "\n".join(out) + "\n", status_rows


def esc(s: str) -> str:
    return s.replace("|", "\\|")


# ---------------------------------------------------------------------------
# Emit STATUS.md
# ---------------------------------------------------------------------------


def render_status(status_rows: list[tuple[str, str, int, int]]) -> str:
    lines = [
        "# Status Snapshot",
        "",
        "> Generated by `bin/generate-prompts.py`. Open counts match preserved",
        "> checkbox state in `ISSUE-TRACKER.md` (``[x]`` = closed). Run",
        "> `bin/sync-tracker.py --apply` after merges to sync from git trailers.",
        "",
        "| Prefix | Phase | Open | Total | % done |",
        "|---|---|---:|---:|---:|",
    ]
    grand_open = 0
    grand_total = 0
    for prefix, phase_label, open_n, total in status_rows:
        pct = 0.0 if total == 0 else 100.0 * (total - open_n) / total
        grand_open += open_n
        grand_total += total
        lines.append(f"| `{prefix}` | {phase_label} | {open_n} | {total} | {pct:.0f}% |")
    overall_pct = 0.0 if grand_total == 0 else 100.0 * (grand_total - grand_open) / grand_total
    lines.append(f"| **Total** | | **{grand_open}** | **{grand_total}** | **{overall_pct:.0f}%** |")
    lines.append("")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--check", action="store_true",
                   help="dry-run; report what would be written")
    p.add_argument("--only", default="",
                   help="comma-separated prefixes to limit (e.g. STAB,CONF)")
    args = p.parse_args(argv)

    only = {x.strip() for x in args.only.split(",") if x.strip()}

    tasks = collect_all_tasks()
    if not tasks:
        print("[error] no tasks parsed", file=sys.stderr)
        return 1

    src_num_index = task_index_by_source_num(tasks)

    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)

    written = 0
    skipped = 0
    for t in tasks:
        prefix, _, _ = prefix_for(t.source_file)
        if only and prefix not in only:
            skipped += 1
            continue
        prompt_text = render_prompt(t, src_num_index)
        target = PROMPTS_DIR / f"{t.batch_id}.prompt.md"
        if args.check:
            if not target.exists() or target.read_text() != prompt_text:
                print(f"[would write] {target}")
            written += 1
        else:
            target.write_text(prompt_text)
            written += 1

    if not only:
        if args.check:
            print(f"[would write] {BATCHES_TOML}")
            print(f"[would write] {TRACKER_MD}")
            print(f"[would write] {STATUS_MD}")
        else:
            prev_state = load_tracker_state(TRACKER_MD)
            tracker_md, status_rows = render_tracker(tasks, prev_state)
            BATCHES_TOML.write_text(render_batches_toml(tasks, src_num_index))
            TRACKER_MD.write_text(tracker_md)
            STATUS_MD.write_text(render_status(status_rows))
            if prev_state:
                done_n = sum(1 for s, _ in prev_state.values() if s == "done")
                prog_n = sum(1 for s, _ in prev_state.values() if s == "progress")
                if done_n or prog_n:
                    print(
                        f"[info] preserved tracker state: {done_n} done, {prog_n} in progress "
                        f"({len(prev_state)} rows parsed)"
                    )

    print(
        f"[done] {written} prompts {'(dry-run)' if args.check else 'written'}, "
        f"{skipped} skipped (filter)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
