---
plan: demo-parallel-integration
---

# Parallel sibling integration demo

This example is a small acceptance exercise for same-plan parallelism. Two
independent producer tasks start from the same Git base and write disjoint JSON
artifacts concurrently. A third task depends on both producers, reads both
accepted sibling outputs, and writes a combined result.

The completed package demonstrates that Roko preserves both parallel tips when
forming the accepted plan tip consumed by downstream work. All generated files
live under `demo/parallel-integration/`.
