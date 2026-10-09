#!/usr/bin/env bash
# build.sh — build the whitepaper PDF (gap-8117a8)
#
# Usage: docs/whitepaper/build.sh [OUTPUT.pdf]
#
# Joins the sections in reading order (00-abstract.md … 10-related-work.md) and then the references, leaving
# out the status header on line 1 of each file. pandoc writes the LaTeX (citeproc with references.bib),
# rsvg-convert from librsvg turns the SVG figures into PDF, and tectonic typesets the result.
#
# The default output is tmp/whitepaper/roko-whitepaper.pdf under the repository root, which is gitignored.
# The script needs no git metadata, so it also builds from an export:
#   git archive <tag> docs/whitepaper | tar -x -C <dir> && <dir>/docs/whitepaper/build.sh
#
# The build is reproducible. The PDF's dates come from SOURCE_DATE_EPOCH, which defaults to the commit time
# of the sources (from git in a checkout, or from the file times git archive sets in an export), so the
# same commit built with the same pandoc, tectonic and librsvg gives the same bytes. The script ends by
# printing the PDF's sha256.

set -euo pipefail

WP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${WP_DIR}/../.." && pwd)"
OUT="${1:-${ROOT}/tmp/whitepaper/roko-whitepaper.pdf}"

TITLE="Roko: A Cybernetic Harness for Multi-Agent Orchestration"
SECTIONS=(
    00-abstract.md
    01-introduction.md
    02-design-principles.md
    03-architecture.md
    04-orchestration.md
    05-cybernetic-mechanisms.md
    06-measured-trust.md
    07-in-use.md
    08-safety.md
    09-open-problems.md
    10-related-work.md
)

# ── Dependency check ──────────────────────────────────────────────────────────
for tool in pandoc tectonic rsvg-convert; do
    if ! command -v "$tool" &>/dev/null; then
        echo "ERROR: '$tool' is required but not installed." >&2
        echo "  macOS: brew install pandoc tectonic librsvg" >&2
        exit 2
    fi
done

# ── Build date ────────────────────────────────────────────────────────────────
if [[ -z "${SOURCE_DATE_EPOCH:-}" ]]; then
    if [[ "$(git -C "$ROOT" rev-parse --show-toplevel 2>/dev/null)" == "$(cd "$ROOT" && pwd -P)" ]]; then
        SOURCE_DATE_EPOCH="$(git -C "$ROOT" log -1 --format=%ct)"
    else
        SOURCE_DATE_EPOCH="$(stat -c %Y "${WP_DIR}/README.md" 2>/dev/null || stat -f %m "${WP_DIR}/README.md")"
    fi
fi
export SOURCE_DATE_EPOCH

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── Figures ───────────────────────────────────────────────────────────────────
# The LaTeX refers to figures/<name>.pdf: the filter below rewrites each .svg image.
cp -R "${WP_DIR}/figures" "${WORK}/figures"
for svg in "${WORK}"/figures/*.svg; do
    rsvg-convert --format pdf --output "${svg%.svg}.pdf" "$svg"
done

# ── LaTeX preamble ────────────────────────────────────────────────────────────
# Tables are set a size smaller than the text. PDF 1.7 matches the figures that rsvg-convert writes.
cat > "${WORK}/header.tex" <<'EOF'
\usepackage{etoolbox}
\usepackage{needspace}
\newcommand{\wptablefont}{\small}
\AtBeginEnvironment{longtable}{\wptablefont}
\setlength{\tabcolsep}{4pt}
\AtBeginDocument{\special{pdf:minorversion 7}}
EOF

# ── Layout filter ─────────────────────────────────────────────────────────────
cat > "${WORK}/layout.lua" <<'EOF'
local LINE_PT = 468        -- text width: letter paper with 1in margins
local COLSEP_PT = 4        -- \tabcolsep in header.tex
local CODE_CHAR_PT = 4.75  -- typewriter character at \small
local BREAK = "[/_%-%.:@]"

-- Width in points of text set at \small, erring wide.
local function text_pt(text)
  local pt = 0
  for _, code in utf8.codes(text) do
    local c = utf8.char(code)
    pt = pt + (c:match("%u") and 6.8 or c:match("[%l%d]") and 4.6 or 3.8)
  end
  return pt
end

-- Split text after each run of break characters, "a::b/c" gives "a::", "b/", "c", leaving no piece
-- shorter than three characters.
local function pieces(text)
  local out, start = {}, 1
  for i = 3, #text - 3 do
    if i - start >= 2 and text:sub(i, i):match(BREAK) and not text:sub(i + 1, i + 1):match(BREAK) then
      out[#out + 1] = text:sub(start, i)
      start = i + 1
    end
  end
  out[#out + 1] = text:sub(start)
  return out
end

local function with_breaks(parts, make)
  local out = {}
  for i, part in ipairs(parts) do
    if i > 1 then out[#out + 1] = pandoc.RawInline("latex", "\\allowbreak{}") end
    out[#out + 1] = make(part)
  end
  return out
end

-- 1. Long code spans and status tags (PARTIAL@a17d4dadd) may break after / _ - . : and @, so that paths
--    wrap in narrow table columns instead of running into the next one. Images point at the converted
--    figures.
local inlines = {
  Code = function(el)
    if utf8.len(el.text) < 16 then return nil end
    return with_breaks(pieces(el.text), function(p) return pandoc.Code(p, el.attr) end)
  end,
  Str = function(el)
    local tag, rest = el.text:match("^(.-%u@)(%x%x%x%x%x%x%x.*)$")
    if not tag then return nil end
    return with_breaks({ tag, rest }, pandoc.Str)
  end,
  Image = function(el)
    el.src = el.src:gsub("%.svg$", ".pdf")
    return el
  end,
}

-- Width in points of the widest piece a cell can't break, and of its whole text on one line. Words break
-- only after a hyphen (TeX never hyphenates the first word of a cell), and work item ids (gap-272448) are
-- kept whole.
local function measure(blocks)
  local min_pt, len_pt = 0, 0
  pandoc.Div(blocks):walk({
    Code = function(el)
      local w = utf8.len(el.text) * CODE_CHAR_PT
      min_pt, len_pt = math.max(min_pt, w), len_pt + w
    end,
    Str = function(el)
      if el.text:match("^%p*%l+%-%x%x%x%x%x%x%p*$") then
        min_pt = math.max(min_pt, text_pt(el.text))
      else
        for piece in el.text:gmatch("[^-]*-?") do min_pt = math.max(min_pt, text_pt(piece)) end
      end
      len_pt = len_pt + text_pt(el.text .. " ")
    end,
  })
  return min_pt, len_pt
end

-- 2. Tables that pandoc sets full width get column widths from their content. Every column is at least as
--    wide as its longest unbreakable piece; the rest of the width starts out in proportion to each
--    column's text and then moves between columns while that makes the table shorter.
local function layout(tbl, line_pt)
  local n = #tbl.colspecs
  local relative = false
  for _, spec in ipairs(tbl.colspecs) do
    if type(spec[2]) == "number" and spec[2] > 0 then relative = true end
  end
  if not relative or n < 2 then return nil end

  local min, max, total, rows = {}, {}, {}, {}
  for j = 1, n do min[j], max[j], total[j] = 12, 0, 0 end  -- no column narrower than 12pt
  local function visit(list, body)
    for _, row in ipairs(list) do
      local lens = {}
      for j, cell in ipairs(row.cells) do
        local lo, len = measure(cell.contents)
        min[j], lens[j] = math.max(min[j], lo), len
        if body then max[j], total[j] = math.max(max[j], len), total[j] + len end
      end
      if body then rows[#rows + 1] = lens end
    end
  end
  visit(tbl.head.rows, false)
  for _, body in ipairs(tbl.bodies) do visit(body.head, true); visit(body.body, true) end

  -- Start in proportion to each column's text, keeping every column between its minimum and its longest
  -- cell; repeat until no column is clamped.
  local avail = line_pt - 2 * n * COLSEP_PT
  local width, fixed = {}, {}
  for _ = 1, n do
    local free, weight = avail, 0
    for j = 1, n do
      if fixed[j] then free = free - width[j] else weight = weight + math.max(total[j], 1) end
    end
    local changed = false
    for j = 1, n do
      if not fixed[j] then
        local lo, hi = min[j], math.max(max[j], min[j])
        width[j] = free * math.max(total[j], 1) / weight
        if width[j] < lo then width[j], fixed[j], changed = lo, true, true
        elseif width[j] > hi then width[j], fixed[j], changed = hi, true, true end
      end
    end
    if not changed then break end
  end

  -- The height of the table, in lines: each row is as tall as its fullest cell.
  local function height()
    local h = 0
    for _, lens in ipairs(rows) do
      local tallest = 1
      for j = 1, n do tallest = math.max(tallest, (lens[j] or 0) / width[j]) end
      h = h + tallest
    end
    return h
  end
  -- Move width from one column to another while that lowers the height, in steps that halve down to a
  -- quarter point, and stop when no single move helps.
  local step, best = avail / 16, height()
  while step >= 0.25 do
    local moved = false
    for a = 1, n do
      for b = 1, n do
        if a ~= b and width[a] - step >= min[a] then
          width[a], width[b] = width[a] - step, width[b] + step
          local h = height()
          if h < best - 1e-9 then best, moved = h, true
          else width[a], width[b] = width[a] + step, width[b] - step end
        end
      end
    end
    if not moved then step = step / 2 end
  end

  local sum = 0
  for j = 1, n do sum = sum + width[j] end
  for j = 1, n do tbl.colspecs[j] = { tbl.colspecs[j][1], width[j] / sum } end
  return tbl
end

-- Lay out each file's tables for its text width: a Div's wp-width, in points, or LINE_PT.
local tables = {
  Div = function(div)
    if not div.attributes["wp-file"] then return nil end
    local line_pt = tonumber(div.attributes["wp-width"]) or LINE_PT
    return div:walk({ Table = function(tbl) return layout(tbl, line_pt) end })
  end,
}

-- 3. A link to one of the joined files (06-measured-trust.md) jumps to that file's first heading; a
--    link to any other repository file keeps its text and loses the link, which a PDF can't follow. Each
--    file arrives wrapped in a Div carrying its name, which is removed here. A heading directly above a
--    table keeps room for the table's first rows, so it never ends a page alone.
local function files(doc)
  local anchor = {}
  for _, blk in ipairs(doc.blocks) do
    local name = blk.t == "Div" and blk.attributes["wp-file"]
    if name then
      for _, inner in ipairs(blk.content) do
        if inner.t == "Header" then anchor[name] = inner.identifier; break end
      end
    end
  end
  doc = doc:walk({
    Link = function(el)
      if el.target:match("^%a[%w+.-]*:") or el.target:match("^#") then return nil end
      if anchor[el.target] then el.target = "#" .. anchor[el.target]; return el end
      return el.content
    end,
  })
  local flat = pandoc.Blocks({})
  for _, blk in ipairs(doc.blocks) do
    if blk.t == "Div" and blk.attributes["wp-file"] then flat:extend(blk.content) else flat:insert(blk) end
  end
  local blocks = pandoc.Blocks({})
  for i, blk in ipairs(flat) do
    if blk.t == "Header" and flat[i + 1] and flat[i + 1].t == "Table" then
      blocks:insert(pandoc.RawBlock("latex", "\\needspace{9\\baselineskip}"))
    end
    blocks:insert(blk)
  end
  doc.blocks = blocks
  return doc
end

return { inlines, tables, { Pandoc = files } }
EOF

# Print a file without its status header (line 1, "Status: …"), wrapped in a Div that names it and, when
# given, the text width in points its tables are laid out for.
section() {
    printf '::: {wp-file="%s"%s}\n' "$1" "${2:+ wp-width=\"$2\"}"
    awk 'NR == 1 && /^Status: / { next } { print }' "${WP_DIR}/$1"
    printf '\n:::\n\n'
}

# ── Build ─────────────────────────────────────────────────────────────────────
{
    for file in "${SECTIONS[@]}"; do
        section "$file"
    done
    printf '# References\n\n```{=latex}\n\\begingroup\\small\n```\n\n'
    printf '::: {#refs}\n:::\n\n'
    printf '```{=latex}\n\\endgroup\n```\n'
} | pandoc \
    --from markdown-implicit_figures \
    --to latex --standalone \
    --citeproc --bibliography "${WP_DIR}/references.bib" \
    --lua-filter "${WORK}/layout.lua" \
    --include-in-header "${WORK}/header.tex" \
    --metadata title="$TITLE" \
    --variable papersize=letter \
    --variable geometry:margin=1in \
    --variable colorlinks=true \
    --fail-if-warnings \
    --output "${WORK}/roko-whitepaper.tex"

# The .tex file's name reaches the PDF's /ID, so it stays the same whatever OUT is called. tectonic warns
# that the figures' PDF 1.7 is newer than its default output; the preamble raises the output to 1.7, so
# that warning is dropped and every other message is kept.
(cd "$WORK" && tectonic --chatter minimal roko-whitepaper.tex) 2>&1 \
    | { grep -v 'Trying to include PDF file with version (1.7)' || true; } >&2

mkdir -p "$(dirname "$OUT")"
cp "${WORK}/roko-whitepaper.pdf" "$OUT"
shasum -a 256 "$OUT"
