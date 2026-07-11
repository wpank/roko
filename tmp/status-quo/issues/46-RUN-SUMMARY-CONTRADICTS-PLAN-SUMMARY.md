# Run summary contradicts plan summary

- Severity: high
- Area: reporting

The final `run.completed` event says 16 total, 9 completed, and 5 failed. Its only nested plan summary says 16 total, 0 completed, and 0 failed. Two tasks are unaccounted for in the global totals and there is no blocked/skipped count.

Consumers cannot determine success, progress, or resumable work from this event. Plan totals must reconcile exactly with global totals and include blocked, skipped, cancelled, and orphaned categories.

