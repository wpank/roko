# Whitepaper figures

The whitepaper's three figures, as SVG. A section includes one as `![Figure N: …](figures/figN-<slug>.svg)` and
prints its caption under the image (`docs/whitepaper/README.md`, "Figures").

| Figure | Section | Source | To change it |
|---|---|---|---|
| `fig1-architecture.svg` | §3 | Maintained by hand; `fig1-architecture.txt` is the text version | Edit the SVG |
| `fig2-golden-path.svg` | §4 | Maintained by hand; `fig2-golden-path.txt` is the text version | Edit the SVG |
| `fig3-status-matrix.svg` | §9 | Generated from `docs/whitepaper/data/mechanisms.toml` | Run `python3 tools/status_matrix.py`; never edit it |

The appendix's summary table is Figure 3's text version, so Figure 3 has no `.txt` file.

## Status marks

Every figure draws a tag the same way, so that it reads in greyscale and in print as well as in colour: a fill, a
pattern, and the tag's name beside it.

| Tag | Mark |
|---|---|
| WIRED | Solid green |
| PARTIAL | Amber stripes at 45° |
| BROKEN | Red cross-hatch |
| ORPHANED | Solid grey |
| BUILT-UNWIRED | Thin grey stripes at 135° |
| MISSING | No fill, dashed outline |
| DOCS-ONLY, REMOVED, UNPROVEN | No fill, dotted outline; REMOVED is also struck through |

The CSS class is the tag in lower case (`wired`, `built-unwired`). `SVG_STYLE` in `tools/status_matrix.py` holds the
patterns and rules; Figures 1 and 2 copy them and add the chip, box and arrow rules. Each figure has an opaque white
background, so it stays readable on a dark page.

## Keeping the marks true

- **Figure 3** is regenerated with the appendix. `python3 tools/status_matrix.py --check` fails when the committed
  SVG differs from what the matrix gives.
- **Figures 1 and 2:** a mark that stands for one matrix row carries `data-row` and `data-tag`, and each figure's
  root carries `data-pin`. The same `--check` fails when a mark's tag differs from its row, or a figure's pin from
  the matrix's pin. After a matrix refresh, change each mark it reports: the group's class, `data-tag`, and the tag
  in its label. Then change `data-pin` and the pin in the figure's note.
- **Figure 2's steps** carry the tags of §4.12's table, which are judgements over several rows, so no check covers
  them. Only step 9's two parts (IS2, IS3) are single rows.

## Rendering

The SVGs use SVG 1.1 shapes, patterns, markers and a CSS `<style>` block: no scripts, no web fonts, and no CSS
variables or media queries, so browsers and librsvg draw them alike. Text is Helvetica or a metric-compatible
substitute (Arial, Liberation Sans, Arimo); boxes leave room for a slightly wider face. When a `<tspan>` follows
other text, put the space before the `<tspan>`, never at its start: librsvg drops a `<tspan>`'s leading space, even
a no-break `&#160;`, so the PDF runs the words together (bug-1f098f).

`docs/whitepaper/build.sh` makes the PDF, and stops unless `pandoc`, `tectonic` and `rsvg-convert` (from librsvg)
are installed. It copies `figures/` into its temporary work directory and converts each `figures/*.svg` there with
`rsvg-convert --format pdf`, so no PDF is ever written beside the SVGs. Its Lua filter changes each image's path
from `.svg` to `.pdf`, so the LaTeX includes the converted figures. pandoc reads the sections with implicit figures
off (`--from markdown-implicit_figures`): an image stays inline, and the caption the section prints under it is the
figure's only caption.
