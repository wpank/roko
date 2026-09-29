# 25.01 -- Tab Architecture

> Depth file for [25-TUI.md](../../25-TUI.md) section 2.

---

## Tab Enum

The `Tab` enum in `crates/roko-cli/src/tui/tabs.rs` defines 11 tabs in display order:

```rust
pub enum Tab {
    Dashboard,   // F1
    Plans,       // F2
    Agents,      // F3
    Git,         // F4
    Logs,        // F5
    Config,      // F6
    Inspect,     // F7
    Marketplace, // F8
    Atelier,     // F9
    Learning,    // F10
    Providers,   // '-' key
}
```

## F-Key Mapping

Each tab is bound to a function key via `Tab::fkey()` and resolved via
`Tab::from_key()`. The roundtrip is tested: `from_key(tab.fkey()) == Some(tab)` for
all 11 tabs. `Tab::next()` and `Tab::prev()` cycle with wrap-around.

## Named Surface Integration

The `Tab::v2_surfaces()` method maps legacy tabs to the E37 named-surface system:

| Tab | Surfaces |
|---|---|
| Dashboard | Workbench, Inbox |
| Plans | Canvas, Flows |
| Agents | Agents |
| Config | System |
| Inspect | Knowledge |
| Git, Logs, Marketplace, Atelier, Learning, Providers | (none) |

This ensures StateHub projections flow correctly to both TUI rendering and
HTTP/SSE consumers.

## View Modules

Each tab has a corresponding view module in `crates/roko-cli/src/tui/views/`:

- `dashboard_view.rs`, `plans_view.rs`, `agents_view.rs`, `git_view.rs`
- `logs_view.rs`, `config_view.rs`, `context_view.rs`, `marketplace_view.rs`
- `atelier_view.rs`, `learning_view.rs`, `providers_view.rs`

## Snapshot Keys

Each tab has a snapshot key for the headless capture engine: `f01` through `f10`
plus `providers`. Used in filenames like `f01-dashboard.txt`.
