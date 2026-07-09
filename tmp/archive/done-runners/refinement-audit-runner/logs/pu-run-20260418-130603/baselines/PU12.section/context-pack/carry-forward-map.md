# Carry-Forward Map — 12 Interfaces

These findings are valid, but should usually be handed to later passes.

| Finding | Better Home | Why |
|---------|-------------|-----|
| build the web portal | later frontend pass | backend exists, frontend absent |
| build Spectre rendering | later visualization pass | no renderer or runtime state contract |
| build sonification | later audio pass | no audio stack |
| build A2UI schema/renderers | later generative-UI pass | design-only today |
| build ACP runtime / VS Code extension | later IDE integration pass | decision docs exist; runtime absent |
| fully prioritize 2,451-line UX proposal document | later product/design review pass | proposal curation, not parity |
| resolve runtime default port in code | later implementation pass | this batch only documents the split |

Working rule:

If the task stops being “make the interfaces docs accurately describe
the repo” and starts becoming “implement a new interface modality”,
capture the seam and defer it.
