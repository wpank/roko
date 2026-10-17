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

**Coverage** (gap-76fd1c, 2026-10-16: bug-f90bd0 reversed part of gap-46fd19's README-only simplification while
fixing LOG1 block C's vague H3 stream, so this section is no longer gap-c71dbb's "one family"). `parse_precise_spec`
reads F1's section layout ("## Acceptance criteria", "## Verify", "## Context", "## Scope", "## Non-goals"):
F1, F2, F3, F5 and F7 all render a `spec.precise.md` in exactly that layout today, so all five convert through it
unchanged. F4 renders one too, but from its own, differently headed template ("## Goal", "## Constraints" and
"## Visible check" rather than an inline Goal line, "## Scope" and "## Verify"; no "## Non-goals" at all; no
explicit `max_loc` sentence -- `parse_precise_spec_f4` covers it, dispatched on the manifest's `family`).
Together that is H3's full 48-instance, 6-family stream (P1-H3: F1-F5 and F7, 8 each). F6 also renders a
`spec.precise.md`, from a third, still different layout, but F6 is not part of H3 and so stays out of scope here.

API:
    parse_precise_spec(text: str) -> dict        # F1's TSS v1 fields; no "id" (also covers F2, F3, F5, F7)
    parse_precise_spec_f4(text: str) -> dict      # the same fields, from F4's own section layout; no "id"
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

# F4's own layout (module docstring): "- **ACn.** text" instead of "- ACn: text", and no "Change at most N
# lines" sentence at all, so its budget is fixed here rather than parsed (gap-76fd1c; unused by check_pair/
# degrade today -- see from_generator's own module, "Coverage" -- but kept non-null for a task that reads it).
_F4_AC_LINE = re.compile(r"^- \*\*(AC\d+)\.\*\*\s*(.+)$")
# A run of backtick-wrapped paths (comma-, "and"- or "or"-joined), stopping at the sentence's period -- not at
# a "." inside a path itself, such as `scripts/migrate_prefix.sh`'s extension.
_F4_CHANGE_ONLY = re.compile(r"Change only ((?:`[^`]+`(?:,\s*|\s+and\s+|\s+or\s+)?)+)\.")
_F4_COMMAND = re.compile(r"^`([^`]+)`")
_F4_MAX_LOC = 60


class ConversionError(ValueError):
    """The text or manifest is not in the layout this converter knows how to read."""


def _lines(body: str) -> list[str]:
    return [line for line in body.splitlines() if line.strip()]


def _join_bullets(body: str) -> list[str]:
    """Logical "- " bullet lines of `body`, with a wrapped continuation (a non-blank line that does not itself
    start a new "- " bullet, such as F4's long AC sentences) joined onto the bullet above it with a space."""
    joined: list[str] = []
    for line in _lines(body):
        if line.lstrip().startswith("- "):
            joined.append(line.strip())
        elif joined:
            joined[-1] += " " + line.strip()
    return joined


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


def parse_precise_spec_f4(text: str) -> dict:
    """A rendered `spec.precise.md` in F4's own section layout (module docstring, "Coverage") into the same
    TSS v1 `[[task]]` fields `parse_precise_spec` returns for F1's layout, minus `id`. Raises `ConversionError`
    on a layout this parser does not recognize."""
    preamble, sections = _split_sections(text)
    heading = preamble.strip()
    if not heading.startswith("# "):
        raise ConversionError(f"expected a top-level \"# title\" heading, found {heading!r}")
    title = heading[2:].strip()
    missing = [name for name in ("Goal", "Acceptance criteria", "Constraints", "Context", "Visible check")
              if name not in sections]
    if missing:
        raise ConversionError(f"missing section(s) of F4's spec layout: {', '.join(missing)}")

    goal = description = sections["Goal"].strip()
    if not goal:
        raise ConversionError("the Goal section is empty")

    acceptance = [f"{match.group(1)}: {match.group(2)}" for match in
                 map(_F4_AC_LINE.match, _join_bullets(sections["Acceptance criteria"])) if match]
    if not acceptance:
        raise ConversionError("no \"- **ACn.** ...\" line in the Acceptance criteria section")

    change_only = _F4_CHANGE_ONLY.search(sections["Constraints"])
    if not change_only:
        raise ConversionError("no \"Change only ...\" sentence in the Constraints section")
    files = re.findall(r"`([^`]+)`", change_only.group(1))
    if not files:
        raise ConversionError("the \"Change only ...\" sentence names no backticked file")

    context = [{"path": match.group(1), "why": match.group(2)}
              for match in map(_CONTEXT_LINE.match, _lines(sections["Context"])) if match]

    visible_body = sections["Visible check"].strip()
    command_match = _F4_COMMAND.match(visible_body)
    if not command_match:
        raise ConversionError("no leading `command` in the Visible check section")

    return {"title": title, "goal": goal, "description": description, "non_goals": [],
            "acceptance": acceptance, "files": files, "max_loc": _F4_MAX_LOC,
            "context": {"read_files": context},
            "verify": [{"phase": "test", "command": command_match.group(1),
                       "covers": [entry.split(":", 1)[0] for entry in acceptance], "expect": "fail_on_base"}]}


_PARSERS = {"F4": parse_precise_spec_f4}  # by manifest "family"; every other covered family uses parse_precise_spec


def task_from_instance(out_dir: Path) -> dict:
    """The TSS v1 `[[task]]` dict for one `generate()` output directory: `parse_precise_spec` (F4:
    `parse_precise_spec_f4`, by the manifest's own `family`) on its precise spec file (the manifest's own
    `spec.precise.path`, not a hardcoded name), with `id` set to the manifest's `instance_id`. Raises
    `ConversionError` if the parsed file list disagrees with the manifest's own `files_in_scope` -- every family
    that renders a `spec.precise.md` builds its "Files in scope" (or F4's "Change only ...") text from exactly
    that list, so a mismatch means this parser is reading a layout it does not actually understand."""
    out_dir = Path(out_dir)
    manifest = json.loads((out_dir / "task.json").read_text(encoding="utf-8"))
    spec_path = out_dir / manifest["spec"]["precise"]["path"]
    parse = _PARSERS.get(manifest["family"], parse_precise_spec)
    task = parse(spec_path.read_text(encoding="utf-8"))
    if task["files"] != manifest["files_in_scope"]:
        raise ConversionError(f"parsed Scope files {task['files']} != manifest files_in_scope "
                              f"{manifest['files_in_scope']} for {manifest['instance_id']}")
    return {"id": manifest["instance_id"], **task}
