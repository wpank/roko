+++
id = "gap-ad9e03"
kind = "gap"
title = "The section bandit drops domain_context, the section that carries lessons, playbooks and episodes, so L-sec withholds L-know and L-play content outside their own arms"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-learn/section_effect"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#3.2"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-compose/src/role_prompts.rs::DROPPABLE_SECTIONS", "crates/roko-cli/src/dispatch/prompt_builder.rs::draw_sections", "crates/roko-learn/src/section_effect.rs::SectionBandit"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-550909"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bandit_never_drops_the_lessons_l_know_draws_in' crates/roko-cli/ && cargo test -p roko-cli bandit_never_drops_the_lessons_l_know_draws_in"
+++

## Problem

Lessons, playbooks and recalled episodes travel in the prompt section `domain_context`. The section bandit (L-sec) may
drop that section: it is in `DROPPABLE_SECTIONS`, and for its first 100 opportunities it is excluded with p between
0.05 and 0.2, later up to 0.5. When the bandit drops it for a chain, every attempt of that task runs without the lessons
and playbooks that L-know and L-play chose. Those two loops have their own withhold arms (h = 0.2) for measurement, so
this is a second, uncoordinated way to withhold the same content.

Seen in round v6, $0 scripted run with default arms (`tmp/hermes-roko-v6/learning-research/evidence/zero-default/`,
`findings/learning-research.md` §3.2):
- the third plan's chain had `domain_context` excluded on all 3 attempts (`decisions.jsonl` `decision_point:
  "sections"`, `source: "explore"`, inclusion p 0.8; `exposures.jsonl` knowledge, episode and playbook rows
  `excluded_reason: "bandit_excluded"`);
- the stored lesson never reached the cheap rung, which failed twice and climbed: $0.0133 for the task, against $0.0034
  for the previous task, which got the lesson.

With the default arms, about one chain in three runs without a matching lesson: bandit up to 20%, L-know's arm 20%,
all-off 3%.

## Why it matters

Goal `learning`. Lessons are the loop that visibly makes later runs cheaper (round v6 live: cost per verified task
$0.068 → $0.045 → $0.016). The bandit was built to learn whether optional sections help. A section that carries L-know
and L-play items is not optional in that sense, and dropping it also confounds L-know's and L-play's own measurement:
- in that run the tick still reported L-know and L-play reach 1.0;
- the census called L-play `dormant:unlogged` (bug-550909).

## Where

- `crates/roko-compose/src/role_prompts.rs::DROPPABLE_SECTIONS` lists `domain_context`;
  `crates/roko-compose/src/role_prompts.rs::PINNED_SECTIONS` holds the never-dropped ones.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::draw_sections`: the bandit's draw per droppable section.
- `crates/roko-learn/src/section_effect.rs::SectionBandit` (`exclusion_probability`, `decide`) and its constants
  `SECTION_EXCLUSION_FLOOR` and `SECTION_EXCLUSION_CAP_EARLY`.
- Workaround today: `[sections] pinned = ["domain_context"]` in `roko.toml`.

## Current state

Working as designed per S02.P1-10, but the design predates lessons and playbooks being L-know's and L-play's main
content in that section.

## Plan

Pick one. The first is recommended:
1. Render L-know's and L-play's items (and episode recall) in a section of their own that the bandit never drops; their
   own arms already randomize them for M2. The bandit keeps exploring the rest of `domain_context`.
2. Or keep one section, but skip the bandit's draw for `domain_context` whenever it carries L-know or L-play items on
   their learned arm.
3. Or pin `domain_context` by default (`PINNED_SECTIONS`), which removes it from L-sec entirely.

Then test `bandit_never_drops_the_lessons_l_know_draws_in`: a chain whose L-know arm is learned and whose bandit draw
excludes the section still gets the lesson in its prompt, and its exposure row shows no `bandit_excluded`.

## Done when

The `[[verify]]` command passes, and the round-v6 default-arms repro shows no `bandit_excluded` knowledge or playbook
exposures.

## Notes

- Keep L-know's and L-play's withhold arms: they are what lets M2 estimate β.
- Re-check `loops.toml` L-sec's notes and the census's section rows after the change.
