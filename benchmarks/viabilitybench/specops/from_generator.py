"""Converts a family generator's `generate()` output into the manipulation-check module's TSS v1 `[[task]]`
shape (gap-c71dbb; S08.T8/S08.T9, task 3234's Plan item 2).

Every family's `gen.py generate()` writes `task.json` (the `vb.task/1` manifest: `instance_id`, `family`,
`ladder`, `files_in_scope`, and the precise spec file's path and hash) into its output directory. Some families
also render that spec file as markdown in S07's TSS v1 section layout (`manipulation_check.py`'s module
docstring): a `# title`, a `**Goal.**` line, a free-text description, then `## Acceptance criteria`,
`## Verify`, `## Context`, `## Scope` and `## Non-goals` sections, each in the fixed prose `check_pair`'s
degrade/score path can act on once it is pulled back out into fields. `parse_precise_spec` does that pull;
`task_from_instance` adds the manifest's own `instance_id` as the task's `id` and cross-checks the parsed file
list against the manifest's `files_in_scope`, which `render_spec` always builds the "Files in scope" bullets
from (a mismatch means this parser read the wrong layout, not that the files genuinely differ).

**Coverage.** Only a family whose `generate()` renders a `spec.precise.md` in this exact section layout
converts today: F1 (`families/f1_pyconv`, used below and by the round-trip test). F4 and F6 also render a
`spec.precise.md`, but from their own, differently headed templates ("## Constraints" and "## Visible check"
rather than "## Scope" and "## Verify", no explicit `max_loc`, no `## Non-goals" at all for F4) -- S08.T8/S08.T9
did not standardise one shape across families, so each needs its own section parser here, not attempted yet.
F2, F3, F5 and F7 render no `spec.precise.md` at all; their task lives in the instance's own rendered
`README.md` instead, deliberately (F5's `gen.py` module docstring: "this family also skips a separate
`template/` directory and `spec.precise.md` generation (gap-46fd19's package note)"), and README prose has no
separable acceptance/context/scope/non-goals sections to parse. H3's stream (P1-H3: F1-F5 and F7, 8 each) can
only run end to end from real generator output, not hand-built fixtures, once those families either grow a
convertible spec or this module grows a second conversion path for README-only families; neither is this
item's "one family" scope.

API:
    parse_precise_spec(text: str) -> dict        # TSS v1 fields parsed from a rendered spec.precise.md; no "id"
    task_from_instance(out_dir: Path) -> dict     # + "id" from task.json's instance_id; cross-checks files
"""

from __future__ import annotations

import json
import re
from pathlib import Path

_AC_LINE = re.compile(r"^- (AC\d+:.*)$")
_CONTEXT_LINE = re.compile(r"^- `([^`]+)`:\s*(.+)$")
_FILE_LINE = re.compile(r"^- `([^`]+)`$")
_COVERS = re.compile(r"covers (.+):")
_MAX_LOC = re.compile(r"Change at most (\d+) lines")
_GOAL = re.compile(r"^\*\*Goal\.\*\*\s*(.+?)\n\n(.*)\Z", re.DOTALL)


class ConversionError(ValueError):
    """The text or manifest is not in the layout this converter knows how to read."""


def _lines(body: str) -> list[str]:
    return [line for line in body.splitlines() if line.strip()]


def _split_sections(text: str) -> tuple[str, dict[str, str]]:
    """(preamble before the first "## " heading, {heading: body}) over a "# "/"## "-headed TSS v1 spec."""
    parts = re.split(r"(?m)^## (.+)$\n?", text)
    return parts[0], {parts[i].strip(): parts[i + 1].strip("\n") for i in range(1, len(parts), 2)}


def _title_goal_description(preamble: str) -> tuple[str, str, str]:
    heading, _, rest = preamble.partition("\n")
    if not heading.startswith("# "):
        raise ConversionError(f"expected a top-level \"# title\" heading, found {heading!r}")
    match = _GOAL.match(rest.strip())
    if not match:
        raise ConversionError("expected a \"**Goal.** ...\" line, then a blank line, then the description")
    goal, description = match.group(1).strip(), match.group(2).strip()
    return heading[2:].strip(), goal, description


def parse_precise_spec(text: str) -> dict:
    """A rendered `spec.precise.md` (F1's section layout; module docstring) into TSS v1 `[[task]]` fields, minus
    `id` (`task_from_instance` adds that from the manifest). Raises `ConversionError` on a layout this parser
    does not recognize, rather than silently guessing at a field."""
    preamble, sections = _split_sections(text)
    title, goal, description = _title_goal_description(preamble)
    missing = [name for name in ("Acceptance criteria", "Verify", "Context", "Scope", "Non-goals")
              if name not in sections]
    if missing:
        raise ConversionError(f"missing section(s) of a TSS v1 spec: {', '.join(missing)}")

    acceptance = [match.group(1) for match in map(_AC_LINE.match, _lines(sections["Acceptance criteria"])) if match]

    verify_body = sections["Verify"]
    covers_match = _COVERS.search(verify_body)
    covers = re.findall(r"AC\d+", covers_match.group(1)) if covers_match else []
    command = next((line.strip() for line in verify_body.splitlines() if line.startswith("    ")), None)
    if command is None:
        raise ConversionError("no indented verify command in the Verify section")

    context = [{"path": match.group(1), "why": match.group(2)}
              for match in map(_CONTEXT_LINE.match, _lines(sections["Context"])) if match]

    scope_body = sections["Scope"]
    loc_match = _MAX_LOC.search(scope_body)
    if not loc_match:
        raise ConversionError("no \"Change at most N lines\" sentence in the Scope section")
    files = [match.group(1) for match in map(_FILE_LINE.match, _lines(scope_body)) if match]

    non_goals = [line[2:].strip() for line in _lines(sections["Non-goals"]) if line.startswith("- ")]

    return {"title": title, "goal": goal, "description": description, "non_goals": non_goals,
            "acceptance": acceptance, "files": files, "max_loc": int(loc_match.group(1)),
            "context": {"read_files": context},
            "verify": [{"phase": "test", "command": command, "covers": covers, "expect": "fail_on_base"}]}


def task_from_instance(out_dir: Path) -> dict:
    """The TSS v1 `[[task]]` dict for one `generate()` output directory: `parse_precise_spec` on its precise
    spec file (the manifest's own `spec.precise.path`, not a hardcoded name), with `id` set to the manifest's
    `instance_id`. Raises `ConversionError` if the parsed file list disagrees with the manifest's own
    `files_in_scope` -- every family that renders a `spec.precise.md` builds its "Files in scope" bullets from
    exactly that list, so a mismatch means this parser is reading a layout it does not actually understand."""
    out_dir = Path(out_dir)
    manifest = json.loads((out_dir / "task.json").read_text(encoding="utf-8"))
    spec_path = out_dir / manifest["spec"]["precise"]["path"]
    task = parse_precise_spec(spec_path.read_text(encoding="utf-8"))
    if task["files"] != manifest["files_in_scope"]:
        raise ConversionError(f"parsed Scope files {task['files']} != manifest files_in_scope "
                              f"{manifest['files_in_scope']} for {manifest['instance_id']}")
    return {"id": manifest["instance_id"], **task}
