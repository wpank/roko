# Whitepaper figures

The whitepaper's two figures, as SVG. A section includes one as an image on its own line,
`![Figure N: …](figures/<file>.svg)`, followed by a caption paragraph that starts `**Figure N:**`
(`docs/whitepaper/README.md`, "Figures").

| Figure | File | Section | What it shows | Drawn from |
|---|---|---|---|---|
| Figure 1 | `fig1-architecture.svg` | §3 | The layers: the surfaces (CLI, TUI, HTTP control plane, ACP, MCP), the loops and services, the substrate (signals, cells and graphs, the bus and the store) and the workers (provider adapters, then agent loops and API models) | The research paper's architecture figure, `f2-architecture.svg` |
| Figure 2 | `fig2-loops.svg` | §5 | The five loops as a ladder, from the tool-call loop (L0, seconds) at the bottom to the audit level (L4, weeks) at the top, each with its clock and goal. Arrows down carry references and settings, arrows up carry records, and planning feeds forward into the attempt and plan loops | The research paper's nested-loops figure, `f1-nested-loops.svg` |

Both are maintained by hand. To change one, edit its SVG, and keep it consistent with the research paper's figure it
was drawn from and with the section that includes it. Like the text, the figures carry no status marks.

## Rendering

The SVGs use SVG 1.1 shapes, patterns, markers and a CSS `<style>` block: no scripts, no web fonts, and no CSS
variables or media queries, so browsers and librsvg draw them alike. Text is Helvetica or a metric-compatible
substitute (Arial, Liberation Sans, Arimo); boxes leave room for a slightly wider face. Each figure has an opaque
white background, so it stays readable on a dark page, and reads in greyscale and in print as well as in colour.
When a `<tspan>` follows other text, put the space before the `<tspan>`, never at its start: librsvg drops a
`<tspan>`'s leading space, even a no-break `&#160;`, so the PDF runs the words together.

`docs/whitepaper/build.sh` makes the PDF, and stops unless `pandoc`, `tectonic` and `rsvg-convert` (from librsvg)
are installed. It copies `figures/` into its temporary work directory and converts each `figures/*.svg` there with
`rsvg-convert --format pdf`, so no PDF is ever written beside the SVGs. Its Lua filter changes each image's path
from `.svg` to `.pdf`, so the LaTeX includes the converted figures. pandoc reads the sections with implicit figures
off (`--from markdown-implicit_figures`): an image stays inline, and the caption the section prints under it is the
figure's only caption.
