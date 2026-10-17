+++
id = "gap-f8a6ea"
kind = "gap"
title = "The plan generator's prompt does not state the generated-plan limits (3 read_files, one line range, anchored symbols), so first drafts fail validation and each costs a planner call"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "b17f08741"
source = "gap-e1fdb3 live check, 2026-10-16 (8 drafts failed validation; the docs request failed after 3 calls)"
discovered_from = "gap-e1fdb3"
anchors = ["crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt", "crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-e1fdb3", "bug-09cc5f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generator_prompt_states_the_generated_plan_limits' crates/roko-cli/src && cargo test -p roko-cli --lib -- generator_prompt_states_the_generated_plan_limits"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T21:16:22Z"
commit = "b17f08741"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:32Z"
forced = false
evidence = "b17f08741: the generator's system prompt has a \"Generated-plan limits\" section, filled by plan_generate.rs::render_generated_plan_limits from PlanExecutionPolicy::generated itself (no hand-written numbers): at most 3 context.read_files entries per task (PLAN_BUDGET_READ_FILES), exactly 1 verify step (PLAN_VERIFY_OWNER), one START-END range of at most 240 lines per `lines`, never a list such as 1-25,320-332 (PLAN_CONTEXT_RANGE, PLAN_CONTEXT_RANGE_BUDGET), at most 24576 bytes of declared context (PLAN_BUDGET_CONTEXT), and symbol anchors that must appear in a declared read_files file (PLAN_CONTEXT_SYMBOL). The checklist's hand-written \"No task requires reading more than 3 files\" now points at the section. The validation retry already quotes the validator's message (plan step 3), unchanged. Checked: cargo test -p roko-cli --lib generator_prompt_states_the_generated_plan_limits passes (each limit with the policy's value, the placeholder filled, the hand-written limit gone); the plan_generate:: and plan_authoring:: lib tests pass (112); cargo clippy -p roko-cli --no-deps -- -D warnings is clean; nightly fmt. Not run: the live rerun of gap-e1fdb3's five requests in Done when (no paid calls in this round)."
+++

## Problem

A generated plan must fit `PlanExecutionPolicy::generated`, which uses the FAST limits: at most 3
`context.read_files`, one verify step, ranges of at most 240 lines, and 24 KiB of declared context. Its `lines`
values must also be single ranges, and its symbol anchors must sit in a declared `read_files` entry. The generator's
system prompt states the task ceiling and the tier size limits. It does not state these limits, so the planner's
first drafts break them. Each break costs a full planner call ("Plan TOML extraction failed ... retrying with
stricter prompt"), and after three a generation fails.

Seen in gap-e1fdb3's live check (2026-10-16, roko at main 5e32264ce, claude-opus-5-5, Max login). Eight planner
drafts failed validation across seven generations:
- `PLAN_BUDGET_READ_FILES [T1]: context.read_files has 7 entries; limit is 3` (CLI-flag request), and the same at 5
  entries (data-change request);
- `PLAN_CONTEXT_RANGE [T1]: invalid line range `1-25,320-332`: range contains more than one separator` (refactor
  request), and `1-60,193-231,975-1044` (serve-route spec);
- `PLAN_TIER_SIZE [T1]: focused task is over its tier's size limits: max_loc = 80 (limit 50), 464-word description
  (limit 350)` (docs request);
- twice, prose taken as the TOML block (`TOML parse error at line 1`), and once `duplicate key `task``. After the
  third, the docs request's first generation ended `Plan generation failed after retries: no valid tasks.toml was
  produced`.

Every generation that produced a draft (six of the seven) needed at least one such retry.

## Why it matters

Each rejected draft costs a frontier-planner call: $0.28–0.51 API-equivalent and up to a minute and a half here. A generation that fails
after three calls writes nothing. The limits are known before the call, so a first draft can meet them.

## Where

- `crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt` and `render_plan_template_guidance`, which
  state the task ceiling and tier sizes.
- `crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy::generated`, the limits the validator applies, and
  `plan_policy.rs`'s `PLAN_CONTEXT_RANGE` and `PLAN_CONTEXT_SYMBOL` checks.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the extraction and validation retries.

## Plan

1. Render the generated policy's limits into the generator prompt from `PlanExecutionPolicy::generated` itself (not
   hand-written numbers): the read_files, verify-step, range and context-byte limits, one range per `lines`, and
   symbols anchored in a declared read_files entry.
2. Test that the prompt states each limit with the policy's value.
3. Optionally, retry with the validator's message in the prompt, so that a retry fixes the named rule.

## Done when

- [ ] `generator_prompt_states_the_generated_plan_limits` passes (the `[[verify]]` command).
- [ ] A live rerun of gap-e1fdb3's five requests needs fewer validation retries than the eight seen on 2026-10-16.

## Notes

- Evidence: `tmp/status-quo/2026-10-16-live/small-live.md` (item 3).
