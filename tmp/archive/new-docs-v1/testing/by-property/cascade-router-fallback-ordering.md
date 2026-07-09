# Cascade Router Fallback Ordering

> If the primary model is unavailable, the CascadeRouter always has a valid fallback. The routing chain never terminates with no available model.

**Crate**: `roko-agent`
**Test type**: Unit test
**Enforcement**: `CascadeRouter::route`
**Last reviewed**: 2026-04-19

---

## Statement

For all task contexts T and all availability states A (where at least one model is available):

`CascadeRouter::route(T, A)` returns `Ok(model)` — never `Err(NoModelAvailable)` if any model is available.

---

## Why It Matters

The CascadeRouter's 3-stage design (Static → Confidence → UCB) ensures cost-optimal model selection. If the fallback chain could exhaust all options without finding an available model, the orchestrator would fail tasks that could have been completed with a cheaper model.

---

## Fallback Chain

1. **Static tier**: selects the configured default model for the task type.
2. **Confidence tier**: if confidence is low, escalates to a higher-capability model.
3. **UCB tier**: among equivalent models, selects the one with best estimated quality/cost.

If a model at tier N is unavailable, the router falls back to tier N+1 or a sibling model within the same tier.

The final fallback is always the most capable (and most expensive) model in the configured fleet.

---

## Property Test

```rust
proptest! {
    #[test]
    fn cascade_router_always_has_fallback(
        task_context in arb_task_context(),
        unavailable_count in 0usize..4, // at most 4 of 5 backends unavailable
    ) {
        let all_backends = vec![
            Backend::ClaudeCLI,
            Backend::AnthropicAPI,
            Backend::OpenAICompat,
            Backend::CursorACP,
            Backend::Ollama,
        ];
        let unavailable: Vec<_> = all_backends[..unavailable_count].to_vec();
        let availability = Availability::all_except(&unavailable);

        let router = CascadeRouter::default();
        let result = router.route(&task_context, &availability);

        prop_assert!(result.is_ok(),
            "CascadeRouter must find a model when {} of {} are available",
            5 - unavailable_count, 5);
    }
}
```

---

## Related Properties

- [safety-pipeline-ordering.md](safety-pipeline-ordering.md)

## See also

- [../by-subsystem/subsystem-agent.md](../by-subsystem/subsystem-agent.md)
