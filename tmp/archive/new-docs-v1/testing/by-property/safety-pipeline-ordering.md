# Safety Pipeline Step Ordering

> The 7 steps of the agent safety pipeline always execute in the prescribed order. No step can be skipped or reordered.

**Crate**: `roko-agent`
**Test type**: Unit test
**Enforcement**: `SafetyPipeline::run`
**Last reviewed**: 2026-04-19

---

## Statement

For all agent calls, the safety pipeline steps execute in this order and exactly this order:
1. Role authorization check
2. Pre-call content validation
3. Context injection (safety context added to prompt)
4. LLM call
5. Response validation
6. Post-call content check
7. Provenance recording + audit log

No step may be skipped. A failure at step N stops the pipeline; steps N+1..7 are not executed — except step 7 (audit log), which always executes even on failure.

---

## Why It Matters

The safety guarantee depends on ordering. If role authorization (step 1) were skipped or ran after the LLM call (step 4), unauthorized roles could query the LLM before being rejected. If provenance recording (step 7) were optional, some LLM calls would be unauditable.

---

## Property Test

```rust
#[test]
fn safety_pipeline_steps_always_in_order() {
    let mut step_log: Vec<u8> = Vec::new();
    let pipeline = SafetyPipeline::new_with_spy(|step: u8| step_log.push(step));

    pipeline.run(ctx.valid_agent_call()).unwrap();

    // Steps 1-7 must appear in order
    assert_eq!(step_log, vec![1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn safety_pipeline_audit_runs_even_on_failure() {
    let mut step_log: Vec<u8> = Vec::new();
    let pipeline = SafetyPipeline::new_with_spy(|step| step_log.push(step));

    // Role auth will fail
    let _ = pipeline.run(ctx.unauthorized_agent_call());

    // Step 1 failed, but step 7 (audit) must still run
    assert!(step_log.contains(&1), "step 1 must run");
    assert!(step_log.contains(&7), "step 7 (audit) must always run");
    assert!(!step_log.contains(&4), "step 4 (LLM call) must not run after step 1 failure");
}
```

---

## Related Properties

- [cascade-router-fallback-ordering.md](cascade-router-fallback-ordering.md)

## See also

- [../by-subsystem/subsystem-agent.md](../by-subsystem/subsystem-agent.md)
