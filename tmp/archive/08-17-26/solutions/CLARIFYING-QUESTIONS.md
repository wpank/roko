# Clarifying Questions

Date: 2026-04-28

I proceeded with assumptions in the recommendation docs so this did not block the audit. These
are the decisions that would materially change the implementation order.

## Highest-Impact Questions

1. Is the first parity milestone specifically no-args interactive `roko`, or should `roko run`
   and `roko plan run` be included in the first milestone too?

   Recommended answer: no-args interactive `roko` first. Pull `roko run` and `plan run` into
   the next milestone after chat proves the session model.

2. Should Claude CLI be the preferred default even when `ZAI_API_KEY`, `ANTHROPIC_API_KEY`, or
   `OPENAI_API_KEY` are present?

   Recommended answer: yes, unless the user explicitly configures a different default provider.
   Mori parity is primarily Claude CLI parity, and current auth detection prefers ZAI first.

3. Can no-args `roko` stop starting background `serve` by default until server auth/CORS/PTY
   routes are hardened?

   Recommended answer: yes. This is the fastest way to unblock local chat parity without mixing
   security work into every chat batch.

4. For the first milestone, is API-provider tool use required, or is API-provider system prompt
   plus history enough?

   Recommended answer: system prompt plus history is enough for M0 if Claude CLI is available.
   API tool use should use existing provider adapters in M1, not a new hand-rolled loop.

5. What should the default interactive role be?

   Recommended answer: start as an implementer-style local coding agent when the user asks for
   code work, but use a read-oriented default until an edit/write intent is clear. This avoids
   giving write tools too eagerly.

6. Is `--dangerously-skip-permissions` acceptable for local Claude CLI parity?

   Recommended answer: only for local workspaces after an explicit config/default decision. Mori
   uses it heavily for agent roles, but Roko should avoid making that invisible.

7. Which Mori UI features are mandatory in the first user-visible pass?

   Recommended answer: streaming text, visible tool output, session continuity, and a clear
   status bar. Defer plan tree, queue, DAG, git graph, and board views until the runtime is
   behaving correctly.

8. Should `solution-ACTUAL.md` be treated as the canonical plan, or should the refined docs
   supersede it?

   Recommended answer: keep `solution-ACTUAL.md` as the diagnosis, but use
   `MY-TAKE-SHORTEST-PATH.md` and `MORI-PARITY-BATCH-PLAN.md` as the implementation plan.

## Assumptions I Used

- Primary user goal is local Mori-like usability, not cloud gateway deployment.
- Claude CLI is available and should be the fastest happy path.
- Existing Roko provider adapters are preferable to more raw provider code in the CLI.
- It is acceptable to defer full runtime convergence if the default chat experience becomes real.
- Documentation should guide the next implementation pass, not change code yet.

## Decisions Needed Before Implementation

- Default provider priority for no-args `roko`.
- Whether background serve remains enabled in M0.
- First-milestone scope: chat only versus chat plus one-shot/run.
- Default role/tool policy.
- Whether API streaming is M1 or later.
- Whether the refined plan should replace or merely annotate `solution-ACTUAL.md`.

## New Questions After Demo Audit

1. Should the first parity milestone include the demo UI's PRD/plan workflow, or is the first
   milestone strictly no-args chat?

   Recommended answer: include a narrow PRD/plan grounding and validation pass. The demo showed
   Roko can generate plausible but wrong implementation plans, which would undermine the agent
   workflow even after chat improves.

2. Should generated plans be allowed to create new crates automatically?

   Recommended answer: no, not by default. Require explicit PRD frontmatter such as
   `allow_new_crates: true`, and otherwise reject new workspace crates when root `Cargo.toml`
   already exists.

3. Should successful subprocess completion count as successful learning for PRD/plan generation?

   Recommended answer: no. For artifact-generation episodes, positive learning should require
   artifact validation success.

4. How much raw Claude stream data should Roko preserve?

   Recommended answer: store sidecars by default for PRD/plan generation and maybe behind a
   retention limit for interactive chat. The current fingerprints are not enough to audit bad
   artifacts later.
