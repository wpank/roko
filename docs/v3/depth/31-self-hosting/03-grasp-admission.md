# 31-03 -- GRASP Regression-Gated Playbook Admission

> **Parent:** [31-SELF-HOSTING.md](../../31-SELF-HOSTING.md) section 4
> **Primary source:** GRASP (arXiv:2605.29668, May 2026)
> **Additional sources:** SiriuS (arXiv:2502.04780), SkillZip (arXiv:2608.11079),
> ReSkill (arXiv:2606.01619)
> **Implementation target:** `crates/roko-learn/src/playbook.rs`,
> `crates/roko-learn/src/playbook_rules.rs`

---

## 1. The Regression Problem

Roko's playbook rules are the system's primary mechanism for learning from
failures. When a task fails due to a recognizable pattern (e.g., "missing
`use` import for crate X"), a rule is created that fires on similar future
tasks and injects the corrective advice into the agent's prompt.

The current admission gate is simple: `support_count >= 5` and
`confidence >= min_confidence`. This checks that the pattern has been observed
enough times and that it has not been contradicted, but it does **not** check
whether the new rule degrades performance on existing trajectories.

This is the regression problem: a rule that helps on new failure cases may
interfere with tasks that were previously succeeding. The interference is
silent -- the task's gate pipeline does not distinguish between "failed for a
new reason" and "failed because a newly-injected playbook rule gave bad advice."

---

## 2. GRASP: Regression-Aware Skill and Planning

GRASP (arXiv:2605.29668, May 2026) provides the solution. The key insight is
that every candidate rule should be tested against a held-out probe set of
recent successful episodes before admission. Only rules with net positive
impact are admitted.

### 2.1 Results

On MedAgentBench (a medical agent benchmark):
- Baseline (no grounding): 40.6% task success
- GRASP (regression-gated grounding): 88.8% task success (+48.2 points)
- Naive grounding (all retrieved knowledge injected): 52.3% task success

The 36-point gap between GRASP and naive grounding demonstrates the
magnitude of the regression problem. Injecting knowledge without regression
testing is actively harmful compared to injecting nothing at all when the
knowledge base grows large enough.

### 2.2 Mechanism

GRASP's admission protocol:

1. **Probe sampling:** Stratified sample from recent successful episodes
2. **Counterfactual simulation:** For each probe episode, simulate whether
   the candidate rule would have fired, and if so, whether its advice would
   have changed the outcome
3. **Regression budget:** Count improvements and regressions; admit only if
   the net is positive with margin
4. **Rejection logging:** Rejected candidates are logged with their regression
   count for post-hoc analysis

---

## 3. Target Design for Roko

### 3.1 Admission Protocol

```
Candidate Rule R_new
    |
    v
1. Sample probe set P from recent successful episodes
   (stratified by role, complexity, crate)
   Size: max(20, 10% of recent successes)
    |
    v
2. For each episode e in P:
   a. Match R_new.triggers against e.context
   b. If match: simulate prompt assembly with R_new.body injected
   c. Estimate outcome change:
      - Would the additional instruction have confused the agent?
      - Does the rule contradict the task's actual approach?
      - HDC similarity between rule context and episode context
    |
    v
3. Compute regression budget:
   improvements = count(P where R_new helps or is neutral)
   regressions = count(P where R_new would hurt)
   margin = max(1, floor(0.1 * |P|))
    |
    v
4. Gate decision:
   if improvements > regressions + margin:
       ADMIT with confidence = 0.50
   else:
       REJECT, log to .roko/learn/rejected-rules.jsonl
```

### 3.2 Probe Set Construction

The probe set must be:

- **Recent:** Only episodes from the last 7 days (or last 100 episodes,
  whichever is smaller). Older episodes may reflect outdated codebase state.
- **Successful:** Only episodes where the gate pipeline passed. Failed
  episodes are handled separately (see SiriuS augmentation below).
- **Stratified:** Proportional representation by role, complexity tier, and
  crate. This prevents the probe set from being dominated by a single task
  type.
- **Minimum size:** At least 20 episodes. If fewer are available, defer
  admission until more evidence accumulates.

### 3.3 Regression Estimation

For each probe episode, the regression estimator answers: "If this rule had
been active when this task ran, would it have hurt?"

Three signals feed the estimation:

1. **Trigger overlap:** Does the rule's trigger match the episode's context?
   If not, no effect -- the rule would not have fired.
2. **Content conflict:** Does the rule's body contradict the approach that
   succeeded? (HDC similarity between rule body and successful output; low
   similarity suggests orthogonal advice, high similarity with negation
   suggests conflict.)
3. **Token budget pressure:** Would injecting the rule have caused truncation
   of other prompt sections that contributed to success?

### 3.4 Margin Calculation

The margin `max(1, floor(0.1 * |P|))` ensures that:

- For small probe sets (20 episodes), at least 1 extra improvement is required
- For large probe sets (100 episodes), at least 10 extra improvements are
  required
- The margin scales with evidence -- more data demands stronger evidence

---

## 4. SiriuS: Failed-Episode Augmentation

SiriuS (arXiv:2502.04780) provides a complementary insight: failed episodes
should not be discarded but repaired: a critic's feedback is used to regenerate and rephrase them into correct trajectories, which join the fine-tuning library as positive examples (Fig. 1, §2.2).

### 4.1 Application to Roko

In the target design, failed episodes participate in the probe set as
**negative probes**: a candidate rule that would have *prevented* a known
failure gets credit, while one that would have *caused* a new failure in the
probe set is penalized.

```
Probe Set P = P_success ∪ P_failure

For P_success: check for regressions (R_new hurts a passing task)
For P_failure: check for rescues (R_new would have prevented this failure)

Net impact = rescues + improvements - regressions
```

This double-sided evaluation gives credit for both preventing failures (the
rule's primary purpose) and not breaking successes (the regression gate).

### 4.2 Corrective Annotations

Each failed episode in the augmented probe set carries:

- The error signature from the gate failure
- The task context at the time of failure
- The agent's attempted approach (from the episode's output tokens)
- A structured "what went wrong" annotation (from the error enrichment
  pipeline in `crates/roko-learn/src/error_enrichment.rs`)

---

## 5. SkillZip: Unbounded Growth Prevention

SkillZip (arXiv:2608.11079) compresses a skill's text with a typed minimum-description-length (MDL) objective under a hard coverage constraint, including on every self-evolution patch (abstract, §V).

### 5.1 The Growth Problem

Without compression, the playbook store grows monotonically: every new failure
pattern potentially produces a new rule. At 100 tasks/day with a 38% first-
attempt failure rate, the store could accumulate 14,000 rules per year. Most
rules overlap semantically -- "add `use` for `HashMap`" and "add `use` for
`HashSet`" are the same lesson.

### 5.2 MDL Compression Protocol

When the playbook store exceeds a configured capacity (default: 500 rules):

1. **Identify merge candidates:** Rules with overlapping triggers AND
   Hamming distance < 0.3 on their HDC fingerprints (i.e., semantically
   similar).

2. **Compute MDL criterion:** The merged rule's description length (body
   tokens + trigger complexity) must be shorter than the sum of the two
   originals while preserving predictive accuracy on the probe set.

3. **Merge:** Combine trigger lists (union), generalize the body (replacing
   specific names with patterns), set confidence to the mean of the two
   originals.

4. **Validate:** Run the merged rule through the GRASP regression gate. If
   it fails, reject the merge and keep both originals.

### 5.3 Growth Bound

With MDL compression active, the store is bounded at approximately 500 rules.
When the store reaches capacity, every new admission triggers a compression
pass. Rules below the minimum confidence threshold (default 0.10) are pruned
first; then MDL merges are attempted.

---

## 6. ReSkill: Post-Admission Refinement

ReSkill (arXiv:2606.01619) provides iterative self-correction for admitted
rules. Where GRASP gates admission, ReSkill refines existing rules after
admission based on continued evidence.

### 6.1 Refinement Triggers

A rule enters the refinement queue when:

- Its confidence drops below 0.60 after an initial period above 0.70
  (declining performance)
- Its trigger false-positive rate exceeds 0.30 (firing too broadly)
- Its HDC fingerprint diverges from the cluster centroid of episodes it
  matches (semantic drift)

### 6.2 Refinement Protocol

1. Narrow trigger specificity (add crate or role filters)
2. Sharpen body text (remove ambiguous advice, add concrete examples)
3. Re-evaluate against GRASP regression gate
4. If improved: update in place with reset confidence at 0.60
5. If not improved: demote to observation-only (fires but does not inject)

---

## 7. Implementation Status

| Component | Status | Location |
|---|---|---|
| Basic playbook admission | Wired | `crates/roko-learn/src/playbook_rules.rs` |
| Confidence dynamics | Wired | Same, `validate()` / `contradict()` |
| GRASP regression gate | Target design | Not yet implemented |
| SiriuS augmentation | Target design | Hindsight adjustments exist but not integrated with admission |
| SkillZip compression | Target design | Not yet implemented |
| ReSkill refinement | Target design | Not yet implemented |
| Rejected rule logging | Target design | Not yet implemented |

---

## References

- GRASP: "GRASP: Grounding Retrieval-Augmented Skill and Planning for
  Medical Agents." arXiv:2605.29668, May 2026.
- SiriuS: "SiriuS: Self-Improving Multi-Agent Systems via Bootstrapped
  Reasoning." arXiv:2502.04780, February 2025.
- SkillZip: "SkillZip: Compressing Agent Skill Libraries via Minimum
  Description Length." arXiv:2608.11079, August 2026.
- ReSkill: "ReSkill: Iterative Self-Correction for Agent Skill Refinement."
  arXiv:2606.01619, June 2026.
