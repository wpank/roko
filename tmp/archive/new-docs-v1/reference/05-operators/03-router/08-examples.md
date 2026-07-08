# Router Examples

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Example: CascadeRouter with Static and UCB

```rust
// source: crates/roko-agent/src/router.rs
let router = CascadeRouter {
    static_router: StaticRouter {
        rules: vec![
            RoutingRule {
                kind_match: Some(Kind::Task),
                min_confidence: Some(0.8),
                action: Action { kind: ActionKind::ExecuteTask, ..Default::default() },
            },
        ],
    },
    confidence_router: ConfidenceRouter::default(),
    ucb_router: UCBRouter::new(vec![
        ActionKind::ExecuteTask,
        ActionKind::RetrieveContext,
        ActionKind::AskClarification,
    ]),
};
```
<!-- source: crates/roko-agent/src/router.rs -->
