/**
 * roko's dot-matrix face: the 5x7 glyphs every roko surface draws its glass readings with. The table is the Hermes
 * plugin's own (round v8's design kit, roko-dots.js), so the portal draws the same dots.
 *
 * Each glyph is seven rows of five bits, as hex, top row first. A reading is two SVG paths: the unlit grid and the
 * lit glyphs, each dot a zero-length stroke with a round cap.
 */

const FONT: Record<string, string> = {
  ' ': '00000000000000', '!': '04040404040004', '"': '0A0A0000000000', '#': '0A0A1F0A1F0A0A', $: '040F140E051E04',
  '%': '18190204081303', '&': '0C12140815120D', "'": '0C040800000000', '(': '02040808080402', ')': '08040202020408',
  '*': '0004150E150400', '+': '0004041F040400', ',': '000000000C0408', '-': '0000001F000000', '.': '00000000000C0C',
  '/': '00010204081000', '0': '0E11131519110E', '1': '040C040404040E', '2': '0E11010204081F', '3': '1F02040201110E',
  '4': '02060A121F0202', '5': '1F101E0101110E', '6': '0608101E11110E', '7': '1F010204080808', '8': '0E11110E11110E',
  '9': '0E11110F01020C', ':': '000C0C000C0C00', '<': '02040810080402', '=': '00001F001F0000', '>': '08040201020408',
  '?': '0E110102040004', '@': '0E11171517100F', A: '0E1111111F1111', B: '1E11111E11111E', C: '0E11101010110E',
  D: '1C12111111121C', E: '1F10101E10101F', F: '1F10101E101010', G: '0E11101711110F', H: '1111111F111111',
  I: '0E04040404040E', J: '0702020202120C', K: '11121418141211', L: '1010101010101F', M: '111B1515111111',
  N: '11111915131111', O: '0E11111111110E', P: '1E11111E101010', Q: '0E11111115120D', R: '1E11111E141211',
  S: '0F10100E01011E', T: '1F040404040404', U: '1111111111110E', V: '11111111110A04', W: '1111111515150A',
  X: '11110A040A1111', Y: '1111110A040404', Z: '1F01020408101F', '[': '0E08080808080E', ']': '0E02020202020E',
  _: '0000000000001F', '|': '04040404040404', '~': '00000815020000', '·': '00000004000000', '×': '00110A040A1100',
  '—': '0000001F000000', '−': '0000001F000000', '→': '0004021F020400',
};

/** Punctuation takes only the columns it lights; digits and letters keep all five, so numbers never jitter. */
const NARROW = new Set([' ', '!', "'", '(', ')', ',', '.', ':', '[', ']', '|', '·', '"']);

/** Dot diameter and pitch in px for each size: S is 14px tall, M 20px, L 26px, XL 35px. */
export const DOT_SIZES = {
  XS: { dot: 1.15, pitch: 1.6 },
  S: { dot: 1.55, pitch: 2.15 },
  M: { dot: 2.15, pitch: 2.95 },
  L: { dot: 2.9, pitch: 3.85 },
  XL: { dot: 3.9, pitch: 5.1 },
} as const;

export type DotSize = keyof typeof DOT_SIZES;

interface Glyph {
  rows: number[];
  /** Columns the glyph takes. */
  width: number;
  /** Its first lit column. */
  x0: number;
}

const glyphs = new Map<string, Glyph>();

/** The glyph for `ch`: itself, its capital, or `·` for anything the face lacks. */
export function glyph(ch: string): Glyph {
  const key = FONT[ch] !== undefined ? ch : FONT[ch.toUpperCase()] !== undefined ? ch.toUpperCase() : '·';
  const cached = glyphs.get(key);
  if (cached) return cached;
  const hex = FONT[key]!;
  const rows = Array.from({ length: 7 }, (_, i) => parseInt(hex.slice(i * 2, i * 2 + 2), 16));
  let lo = 5;
  let hi = -1;
  for (const row of rows) {
    for (let c = 0; c < 5; c += 1) {
      if ((row >> (4 - c)) & 1) {
        lo = Math.min(lo, c);
        hi = Math.max(hi, c);
      }
    }
  }
  const g = NARROW.has(key)
    ? hi < 0
      ? { rows, width: 2, x0: 0 }
      : { rows, width: hi - lo + 1, x0: lo }
    : { rows, width: 5, x0: 0 };
  glyphs.set(key, g);
  return g;
}

function dot(x: number, y: number, size: DotSize): string {
  const s = DOT_SIZES[size];
  return `M${(x * s.pitch + s.dot / 2).toFixed(2)} ${(y * s.pitch + s.dot / 2).toFixed(2)}h0`;
}

/** A reading drawn as dots: its size in px, the unlit grid's path and each glyph's lit path. */
export interface DotReading {
  width: number;
  height: number;
  /** Stroke width: the dot's diameter. */
  dot: number;
  grid: string;
  /** One entry per glyph that lights any dot; `roll` when it differs from the reading shown before. */
  lit: { d: string; roll: boolean }[];
}

/**
 * `text` as dots at `size`. `prev`, the text shown before, marks the glyphs that changed, so they roll in.
 * Text is drawn in capitals.
 */
export function dotReading(text: string, size: DotSize, prev: string | null = null): DotReading {
  const s = DOT_SIZES[size];
  const str = text.toUpperCase();
  const before = prev === null ? null : prev.toUpperCase();
  const lit: DotReading['lit'] = [];
  let col = 0;
  Array.from(str).forEach((ch, i) => {
    const g = glyph(ch);
    let d = '';
    g.rows.forEach((row, r) => {
      for (let c = 0; c < g.width; c += 1) {
        if ((row >> (4 - (g.x0 + c))) & 1) d += dot(col + c, r, size);
      }
    });
    if (d) lit.push({ d, roll: before !== null && Array.from(before)[i] !== ch });
    col += g.width + 1;
  });
  const cols = Math.max(1, col - 1);
  let grid = '';
  for (let r = 0; r < 7; r += 1) for (let c = 0; c < cols; c += 1) grid += dot(c, r, size);
  return {
    width: Number((cols * s.pitch - (s.pitch - s.dot)).toFixed(2)),
    height: Number((7 * s.pitch - (s.pitch - s.dot)).toFixed(2)),
    dot: s.dot,
    grid,
    lit,
  };
}
