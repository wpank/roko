# InsightStore, Tokenomics, And HDC Checklist

## Scope

Use this file for InsightStore data structures, pheromone/demurrage logic, token economics, and HTC/HDC precompile work.

## Implementation checklist

- [ ] Define InsightStore entry types against current neuro/knowledge terminology.
- [ ] Implement local or simulated scoring first.
  - pheromone weight with demurrage;
  - reputation-weighted quality;
  - similarity query behavior.
- [ ] Keep tokenomics separate from storage.
  - emission schedule;
  - multiplier rules;
  - decay/demurrage behavior.
- [ ] Implement HDC/HTC precompile behavior in pure functions before on-chain wrapping.
  - similarity;
  - top-k;
  - bind;
  - bundle.
- [ ] Align naming with `roko-primitives` and `roko-neuro` so the same vector semantics are reused everywhere.

## Verification checklist

- [ ] Similarity search results in chain land are comparable to local HDC behavior.
- [ ] Demurrage and reputation multipliers have unit tests with known fixtures.
- [ ] InsightStore query behavior is measurable in mirage or deterministic local tests.

## Acceptance criteria

- InsightStore vocabulary matches the rest of the workspace.
- Tokenomics, storage, and vector operations are independently testable.
- HDC behavior does not diverge across local and chain-oriented code.
