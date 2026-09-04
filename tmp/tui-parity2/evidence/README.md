# TUI parity evidence

These directories contain full-frame **plain-text** captures produced through the shipping
`App::draw` path. Each directory has all ten top-level tabs plus a schema-v2 `manifest.json`.

| Directory | Terminal | Effects intent |
|---|---:|---|
| `static-clean-80x24` | 80x24 | reduced-motion usability baseline |
| `static-clean-120x40` | 120x40 | reduced-motion usability baseline |
| `static-clean-200x60` | 200x60 | reduced-motion usability baseline |
| `static-full-120x40` | 120x40 | explicit Full-preset path |

Representative command:

```sh
ROKO_REDUCED_MOTION=1 roko screenshot \
  --dir tmp/tui-parity2/evidence/static-clean-120x40 \
  --width 120 --height 40
```

The captures verify cell layout, hierarchy, reachability, wrapping, clipping, footer preservation,
and panic-free rendering. Plain text intentionally strips color/style attributes, so identical
clean/Full text does not prove that their terminal appearance is identical. Palette/effect fidelity
requires the open ANSI/PNG work documented in `../32-SCREENSHOT-HARNESS-STATUS.md`.

The 80x24 matrix exposed both stale-toast obstruction and an Agents split-index panic. The checked-in
frames are regenerated after those fixes and are expected to contain exactly the requested number
of display rows per file.
