# Fine-Tuning Loop, Integration, And Acceptance

## Scope

Use this file for training-data extraction, Hub export, router updates, and final integration tests for the larger extensibility program.

## Implementation checklist

- [ ] Extract training data only from episodes with enough provenance.
  - prompt/task context;
  - model choice;
  - outcome and gate results;
  - privacy/redaction handling.
- [ ] Push data to external hubs only through an explicit exporter.
  - schema version;
  - redaction policy;
  - retry/failure behavior.
- [ ] Make router updates additive.
  - scan for new fine-tuned models;
  - validate metadata;
  - add new arms without breaking old scoring history.
- [ ] Run integration tests in vertical slices.
  - package install and load;
  - multi-profile startup;
  - multi-chain subscription;
  - foraging loop consuming chain events;
  - WorldGraph updates;
  - fine-tuned model discovery.

## Verification checklist

- [ ] Exported training records are schema-versioned and redaction-tested.
- [ ] Router can detect and register a newly published fine-tuned model in a test fixture.
- [ ] End-to-end slices are runnable without requiring every future crate to already exist.

## Acceptance criteria

- Training export has provenance and redaction built in.
- Router updates are observable and reversible.
- The system can demonstrate one credible end-to-end slice across installation, ingestion, learning, and routing.
