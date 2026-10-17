+++
id = "gap-505edb"
kind = "gap"
title = "roko learn sections reads section-outcomes.jsonl, which Graph runs never write, instead of the section bandit's posteriors"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4c.13"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-cli/src/commands/learn.rs::cmd_learn_sections", "crates/roko-learn/src/section_effect.rs::SectionBandit"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad9e03"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_sections_shows_the_bandit_posteriors' crates/roko-cli/ && cargo test -p roko-cli learn_sections_shows_the_bandit_posteriors"
+++

## Problem

`roko learn sections` ("Show per-section prompt pass-rate statistics") prints "Section outcomes: no data" on every Graph
workspace:
- it reads `.roko/learn/section-outcomes.jsonl`, which Graph runs do not write;
- the section bandit's state, the thing that decides which prompt sections a Graph attempt gets, lives in
  `.roko/learn/section-bandit.json` (per-section Beta posteriors for the included and excluded arms, with
  opportunities).

Seen in round v6 (`tmp/hermes-roko-v6/learning-research/evidence/zero-default/03-p3-heading-anchors-after/
learn-sections.txt`). In the same workspace the bandit had dropped `domain_context`, the section that carries lessons,
for a whole task, and no CLI shows why.

## Why it matters

Goal `tooling` / `learning`. L-sec is one of the loops on by default on every Graph prompt. Its state and its
exclusion odds should be inspectable without opening JSON by hand.

## Where

- `crates/roko-cli/src/commands/learn.rs::cmd_learn_sections`: reads the old outcome log.
- `crates/roko-learn/src/section_effect.rs::SectionBandit`: the posteriors and `exclusion_probability`.

## Current state

The bandit is saved at each run's end. Nothing prints it.

## Plan

1. Make `roko learn sections` read the bandit: per section, the included and excluded posteriors (mean and count), the
   opportunities, the current exclusion probability, and whether it is pinned (`[sections] pinned` or built in).
   Keep the old outcome log as a second block only when it has rows.
2. `--json` gives the same.
3. Test `learn_sections_shows_the_bandit_posteriors` over a fixture `section-bandit.json`.

## Done when

The `[[verify]]` command passes, and the command prints the bandit's state for a workspace that ran Graph plans.
