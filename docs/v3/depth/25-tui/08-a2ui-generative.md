# 25.08 -- A2UI Generative Interfaces

> Depth file for [25-TUI.md](../../25-TUI.md) section 10.

---

## Motivation

Pre-designed dashboards work for known data shapes. But agents encounter novel
situations -- research findings, architectural proposals, debugging workflows,
domain-specific views -- that benefit from custom structured presentation. Without
A2UI, this information renders as flat text. With A2UI, agents create structured,
interactive components.

## Protocol

Agents emit JSONL with an `"a2ui"` key in their output stream. The host interface
(TUI/Web/CLI) renders using ROSEDUST styling. The agent describes *what* to show;
the renderer decides *how*.

### Component Types

| Type | Description | TUI | Web |
|---|---|---|---|
| `table` | Data table | Unicode box drawing | HTML table, glass panel |
| `progress` | Progress bar | `████░░░░` | Animated gradient bar |
| `chart` | Visualization | Braille/ASCII | Recharts/Nivo |
| `status` | Status list | Symbols `✓`/`✗`/`○` | Colored badges |
| `code` | Code block | Syntax colored | Prism.js |
| `callout` | Alert box | Bordered with icon | Glass panel |
| `tree` | Hierarchy | ASCII `├──`/`└──` | Collapsible tree |
| `kv` | Key-value | Aligned columns | Definition list |
| `diagram` | Simple diagram | ASCII box-and-arrow | SVG |
| `form` | Input form | Not supported | HTML form |
| `markdown` | Rich text | Terminal rendered | HTML rendered |

### ROSEDUST Inheritance

Components use semantic color names that resolve to ROSEDUST values:

| Name | ROSEDUST Color |
|---|---|
| `primary` | Rose |
| `success` | Jade/Sage |
| `warning` | Gold/Warning |
| `danger` | Ember |
| `info` | Sapphire/Teal |
| `muted` | Text dim |

Agents never emit raw color codes.

### Incremental Updates

Following RFC 6902 JSON Patch, streaming updates avoid resending entire
components:

```jsonl
{"a2ui": "progress", "id": "build", "label": "Building", "value": 0.0}
{"a2ui_patch": "build", "op": "replace", "path": "/value", "value": 0.5}
{"a2ui_patch": "build", "op": "replace", "path": "/value", "value": 1.0}
```

## Sandboxing

A2UI components are sandboxed:

- Render only within the agent's output viewport
- Cannot modify TUI chrome, overlay other agents, or trigger navigation
- Cannot execute arbitrary code
- All input schema-validated before rendering
- Unknown types rendered as raw JSON code blocks

### Resource Limits

| Limit | Value |
|---|---|
| Components per turn | 10 |
| Table rows | 50 |
| Chart data points | 100 |
| Code block lines | 200 |
| Callout content | 1000 chars |

## Accessibility

Automatic ARIA attribute injection per component type:
- Tables: `role="grid"` with `aria-label`
- Progress bars: `role="progressbar"` with `aria-valuenow`
- Status: `role="status"`, `aria-live="polite"`
- Danger callouts: `role="alert"`, `aria-live="assertive"`

## Industry Context

- Google A2UI (2025): catalog-based security, flat list with ID references
- Vercel json-render (2026): Zod-schema constraints for valid payloads

Roko differentiation: ROSEDUST inheritance, sandboxed viewport containment,
multi-renderer parity (TUI + Web + CLI from same JSONL).
