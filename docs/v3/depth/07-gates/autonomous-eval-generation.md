# Autonomous Evaluation Generation

> Depth file for [07-GATES.md](../../07-GATES.md) section 12.
> Source: `crates/roko-gate/src/generated_test_gate.rs`,
> `crates/roko-gate/src/property_test_gate.rs`,
> `crates/roko-gate/src/gate_generator.rs`

---

## 1. The Verification Gap

Hand-written tests verify what the human thought to test. Generated tests
verify what the *agent actually changed*. These are different things. An agent
asked to implement a rate limiter might introduce an off-by-one error that no
existing test covers, because no existing test was written for a rate limiter
that did not yet exist.

Autonomous evaluation generation closes this gap: the system creates its own
verification criteria -- test cases, property assertions, invariant checks --
without human intervention. For each task, targeted tests are generated that
specifically exercise the agent's expected output.

---

## 2. The Three-Stage Pipeline

The pipeline executes **before** the implementation agent starts work:

### Stage 1: Test Generation

A dedicated test-generation agent (not the implementation agent) reads the task
specification and generates test cases. These tests are:

- Targeted at the specific code being changed
- Written to fail before the implementation (red-green-refactor)
- Stored as immutable artifacts in the `ArtifactStore`

```
Task spec: "Implement pub fn check_rate(&self, client: &str) -> Result<()>"

Generated tests:
  - rate_limiter_allows_within_limit()
  - rate_limiter_rejects_over_limit()
  - rate_limiter_resets_after_window()
  - rate_limiter_independent_per_client()
```

### Stage 2: Test Validation

Generated tests are compiled and run against the *current* codebase (before
agent changes). Expected behavior by category:

| Test category | Expected pre-implementation result |
|---|---|
| New functionality | Should FAIL (feature does not exist yet) |
| Existing functionality being modified | Should PASS (baseline) |
| Tests that do not compile | Rejected (invalid test) |

Validation ensures the generated tests are meaningful. A test that passes
before the implementation was written tests nothing new. A test that does not
compile wastes gate budget on Rung 4.

### Stage 3: Test Registration

Validated tests register with the `GeneratedTestGate` (Rung 4). When the
implementation agent produces its code, Rung 4 runs these tests against the
new code. If the implementation is correct, the previously-failing tests now
pass:

```
Before implementation:
  Generated tests -> FAIL (expected: feature does not exist)

After implementation:
  Generated tests -> PASS (expected: feature now works)
```

This is the classic red-green-refactor cycle, automated: the system generates
the tests, the agent generates the implementation, the gate verifies alignment.

---

## 3. The Separation Principle

**Test generation and implementation are performed by different agents.**

This is the single most important architectural decision. If the implementation
agent generates its own tests, it generates tests that pass for its
implementation -- not tests that verify correctness. The implementation agent
is incentivized to make tests easy to pass. The test generation agent is
incentivized to make tests hard to pass (thorough).

This adversarial relationship improves verification quality:

```
Test agent:           "Here are the hardest tests I can think of"
Implementation agent: "Here is code that passes all those tests"
Gate pipeline:        "Confirmed -- the code passes the tests"
```

The Song et al. (ICLR 2025) Generation-Verification-Update framework
formalizes why this works: self-improvement succeeds when verification
capability exceeds generation capability. Using a separate, capable model for
test generation ensures the verifier is at least as sophisticated as the
generator.

---

## 4. Test Generation Strategies

### 4.1 Example-Based Tests

Concrete tests with specific inputs and expected outputs. The most common type:

```rust
#[test]
fn rate_limiter_allows_within_limit() {
    let limiter = RateLimiter::new(100, Duration::from_secs(60));
    assert!(limiter.check("client-1").is_ok());
}

#[test]
fn rate_limiter_rejects_over_limit() {
    let limiter = RateLimiter::new(1, Duration::from_secs(60));
    limiter.check("client-1").unwrap();
    assert!(limiter.check("client-1").is_err());
}
```

Easy to generate, easy to understand, clear pass/fail semantics.

### 4.2 Property-Based Tests

For tasks where invariants matter more than specific examples:

```rust
#[proptest]
fn rate_limiter_never_allows_over_limit(
    limit in 1..100u32,
    requests in 1..200u32,
) {
    let limiter = RateLimiter::new(limit, Duration::from_secs(60));
    let mut allowed = 0;
    for _ in 0..requests {
        if limiter.check("client").is_ok() {
            allowed += 1;
        }
    }
    prop_assert!(allowed <= limit);
}
```

Property tests exercise a wider input space. They are more expensive to run
(Rung 5) but catch edge cases that example tests miss.

### 4.3 Invariant Tests

Tests that verify the agent's changes do not break existing functionality:

```rust
// Pre-condition: passes before agent changes
// Post-condition: must still pass after agent changes
#[test]
fn existing_auth_still_works() {
    let auth = AuthService::new();
    assert!(auth.validate_token("valid-token").is_ok());
}
```

Invariant tests protect against regressions. They assert that the world the
agent inherits remains intact.

---

## 5. Gate Integration

Generated tests integrate with the gate pipeline through two gates:

### 5.1 GeneratedTestGate (Rung 4)

Runs example-based generated tests. Behaves like `TestGate` (Rung 2) but
operates on a different test suite -- the generated tests rather than the
project's existing tests. The `GeneratedCheck` struct carries the test source
and its expected-pre-implementation outcome:

```rust
pub struct GeneratedCheck {
    pub source: String,           // Test source code
    pub expected_pre: bool,       // Should it pass before implementation?
    pub artifact_hash: ContentHash, // Immutable identity in ArtifactStore
}
```

### 5.2 PropertyTestGate (Rung 5)

Runs property-based generated tests using proptest or quickcheck frameworks.
Property definitions are generated alongside example tests but exercise
randomized input spaces.

Both gates return standard `Verdict` objects, so they compose naturally with
the pipeline, ratchet, and adaptive thresholds.

### 5.3 VerifyChainGate (auxiliary)

Chains multiple generated checks into a single verification unit. A
`VerifyChainGate` passes only when all checks in its chain pass, providing
holistic verification for multi-faceted tasks.

---

## 6. GateGenerator and GeneratedCheck

The `GateGenerator` produces `GeneratedCheck` instances at runtime, enabling
the system to create task-specific verification on the fly:

```rust
pub struct GateGenerator {
    pub task_spec: String,
    pub source_roots: Vec<PathBuf>,
    pub build_system: BuildSystem,
}
```

Generation flow:

```
task_spec + source_roots
    -> GateGenerator.generate()
    -> Vec<GeneratedCheck>
    -> ArtifactStore.store(check.source) -> ContentHash
    -> GeneratedTestGate.register(checks)
    -> Rung 4 executes during pipeline
```

Content-addressing the generated test source in the `ArtifactStore` means the
implementation agent cannot modify the tests to make them pass. The hash is
the test's identity; any mutation changes the hash.

---

## 7. Immutable Verification Artifacts

Generated tests are stored as immutable artifacts before implementation
begins. This ensures:

1. **No tampering.** The implementation agent cannot modify the tests.
2. **Reproducibility.** The exact tests used for verification can be
   retrieved by hash for forensic replay.
3. **Auditability.** Content-addressed artifacts create a cryptographic chain
   from test generation through verification outcome.

The `ArtifactStore`'s append-only semantics (no delete, no update) make this
immutability structural, not just conventional.

---

## 8. The Cheap-Model Convergence Loop

For simple generated tests, a convergence loop with a cheap model reduces cost:

```
while not converged:
    cheap_model generates implementation attempt
    generated_tests run against attempt
    if all pass: converged = true
    else: feed errors back to cheap_model

if converged:
    submit to full gate pipeline
else (after N attempts):
    escalate to expensive model
```

If 60% of tasks are solvable by a cheap model, the overall cost drops by
approximately 50% while maintaining quality -- generated tests enforce the same
standard regardless of which model produced the code.

---

## 9. Dependency Detection and Mock Generation

For tasks involving external systems, the generation pipeline detects
dependencies and produces appropriate mocks:

| Dependency type | Mock strategy |
|---|---|
| File system | In-process mock or temp directory |
| Network (HTTP, gRPC) | Mock server with recorded responses |
| Database | In-memory store or fixture data |
| System clock | Fixed-time wrapper |
| Randomness | Deterministic seed |

Mock generation is part of the test generation stage, not the implementation
stage. The test agent produces both the tests and the mocks they need.

---

## 10. Quality Metrics

The autonomous eval generation system tracks its own quality:

| Metric | What it measures | Target |
|---|---|---|
| Generation success rate | % of generated tests that compile | > 95% |
| Test discrimination | % that fail before implementation | > 80% |
| False positive rate | % that fail on correct implementations | < 5% |
| Coverage improvement | Additional coverage over hand-written | > 20% |
| Cost per test | Token cost to generate one test | < $0.01 |

These metrics feed Loop 14 (meta-learning) in the evaluation lifecycle,
adjusting the generation strategy over time. A drop in discrimination means
the test generator is producing tests that are too easy; a rise in false
positives means the tests are brittle.

---

## 11. Relationship to the Evaluation Lifecycle

Autonomous eval generation operates at the Cognitive Speed tier (seconds to
minutes):

```
Machine speed:      Per-turn tool call data collection
Cognitive speed:    Test generation -> validation -> registration   <- here
Consolidation:      Analyze which generated tests caught real bugs
Retrospective:      Compare generated vs. hand-written test effectiveness
Meta:               Evaluate whether test generation improves outcomes
```

The consolidation loop reviews generated test outcomes after a batch of tasks.
Tests that consistently caught bugs are promoted as template patterns. Tests
that consistently false-alarmed are retired.

---

## 12. Verification commands

```bash
# Run generated test gate in isolation
cargo test -p roko-gate -- generated_test

# Run property test gate
cargo test -p roko-gate -- property_test

# Execute a plan with full rung ladder (includes Rungs 4-5)
cargo run -p roko-cli -- plan run plans/ --complexity complex

# Validate gate generator output
cargo test -p roko-gate -- gate_generator
```

---

## 13. Test criteria

| Test | Property |
|---|---|
| `generated_tests_fail_before_implementation` | Red-green: tests fail on pre-change code |
| `generated_tests_pass_after_implementation` | Green: tests pass on correct implementation |
| `invalid_generated_tests_rejected` | Non-compiling tests do not reach Rung 4 |
| `artifact_immutability_preserved` | Stored test hash matches content on retrieval |
| `property_tests_catch_invariant_violations` | Randomized inputs find edge cases |
| `mock_generation_covers_dependencies` | Tests with mocks run without external services |
| `verify_chain_requires_all_checks` | VerifyChainGate fails if any check fails |
| `gate_generator_produces_targeted_tests` | Generated tests reference task-specific symbols |
