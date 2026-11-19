Title: P0 · Undertow foundation

Create a roko plan for phase 0 from docs/specs/phase-0-foundation.md.

Requirements:
- Do not run or execute the plan yet; only generate the plan.
- The plan must cover the full phase 0 scope from the spec: build system, core library parameter table, engine skeleton, plugin skeleton, and tools/check.sh.
- Respect the spec's constraints: core stays plain C++17 with no JUCE include; plugin is a thin wrapper around undertow::Engine; do not modify tests/spec/, third_party/, docs/, or the given tools scripts.
- Organize the work into sensible waves that match the spec's suggested dependencies: build system first, then parameter table and engine skeleton (can parallelize after build system), then plugin skeleton/check script.
- Ensure each task has concrete checks aligned to the acceptance checks in the spec.
- The generated plan should be titled exactly: P0 · Undertow foundation.
- Cap the plan-writing spend at $0.60.