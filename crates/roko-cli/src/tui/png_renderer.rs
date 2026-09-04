//! PNG rendering of terminal buffer snapshots.
//!
//! Converts text-mode TUI snapshots (plain text or ANSI-encoded terminal
//! buffers) into PNG images for pixel-level visual regression testing.
//!
//! The initial implementation uses a built-in 8x16 bitmap font to rasterize
//! each character cell. This avoids pulling heavy image/font dependencies
//! into the default build. When the `tui-png` feature is enabled, a future
//! iteration can swap in `fontdue` + `image` for higher-fidelity rendering
//! with configurable typefaces and anti-aliasing.
//!
//! # Feature gate
//!
//! All public types and functions in this module are gated behind
//! `#[cfg(feature = "tui-png")]`.

use std::path::Path;

use anyhow::{Context as _, Result};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Rendering parameters for terminal-to-PNG conversion.
#[derive(Debug, Clone)]
pub struct PngRendererConfig {
    /// Width of each character cell in pixels.
    pub cell_width: u32,
    /// Height of each character cell in pixels.
    pub cell_height: u32,
    /// Horizontal padding around the rendered grid (pixels).
    pub padding_x: u32,
    /// Vertical padding around the rendered grid (pixels).
    pub padding_y: u32,
    /// Default foreground color (R, G, B).
    pub foreground: [u8; 3],
    /// Default background color (R, G, B).
    pub background: [u8; 3],
}

impl Default for PngRendererConfig {
    fn default() -> Self {
        Self {
            cell_width: 8,
            cell_height: 16,
            padding_x: 8,
            padding_y: 8,
            // ROSEDUST TEXT color (165, 142, 158) over VOID black.
            foreground: [165, 142, 158],
            background: [0, 0, 0],
        }
    }
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// Rasterizes text terminal buffers into PNG images.
///
/// The renderer treats the input as a grid of character cells and paints each
/// cell using a minimal built-in bitmap font. ANSI SGR escape sequences are
/// stripped for now; a future pass can use [`super::ansi::parse_ansi_line`] to
/// preserve color information in the rasterized output.
pub struct PngRenderer {
    config: PngRendererConfig,
}

impl PngRenderer {
    /// Create a renderer with the given configuration.
    #[must_use]
    pub fn new(config: PngRendererConfig) -> Self {
        Self { config }
    }

    /// Create a renderer with default ROSEDUST-themed settings.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(PngRendererConfig::default())
    }

    /// Borrow the active rendering configuration.
    #[must_use]
    pub fn config(&self) -> &PngRendererConfig {
        &self.config
    }

    /// Render a terminal text buffer to a PNG file.
    ///
    /// `terminal_buffer` is the plain-text (or ANSI-encoded) content produced
    /// by the headless snapshot engine. Each line maps to one terminal row.
    ///
    /// The output PNG dimensions are derived from the grid size and the
    /// configured cell dimensions plus padding.
    ///
    /// # Errors
    ///
    /// Returns an error if the output path cannot be written or if the
    /// terminal buffer is empty.
    pub fn render_to_png(&self, terminal_buffer: &str, output_path: &Path) -> Result<PngOutput> {
        let lines = parse_grid(terminal_buffer);
        anyhow::ensure!(!lines.is_empty(), "terminal buffer is empty");

        let cols = lines.iter().map(|line| line.len()).max().unwrap_or(0);
        let rows = lines.len();

        let image_width = (cols as u32) * self.config.cell_width + 2 * self.config.padding_x;
        let image_height = (rows as u32) * self.config.cell_height + 2 * self.config.padding_y;

        // Allocate the raw pixel buffer (RGB, 3 bytes per pixel).
        let mut pixels = vec![0u8; (image_width * image_height * 3) as usize];

        // Fill background.
        for pixel in pixels.chunks_exact_mut(3) {
            pixel.copy_from_slice(&self.config.background);
        }

        // Rasterize each character cell using the built-in bitmap font.
        for (row_idx, line) in lines.iter().enumerate() {
            for (col_idx, &ch) in line.iter().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let glyph = builtin_glyph(ch);
                self.blit_glyph(
                    &mut pixels,
                    image_width,
                    col_idx as u32,
                    row_idx as u32,
                    &glyph,
                );
            }
        }

        // Write the raw pixels as a minimal PNG.
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory: {}", parent.display())
            })?;
        }

        write_minimal_png(output_path, image_width, image_height, &pixels)?;

        Ok(PngOutput {
            path: output_path.to_path_buf(),
            width: image_width,
            height: image_height,
            grid_cols: cols,
            grid_rows: rows,
        })
    }
}

impl PngRenderer {
    /// Paint a single glyph bitmap into the pixel buffer.
    fn blit_glyph(
        &self,
        pixels: &mut [u8],
        image_width: u32,
        col: u32,
        row: u32,
        glyph: &GlyphBitmap,
    ) {
        let x0 = self.config.padding_x + col * self.config.cell_width;
        let y0 = self.config.padding_y + row * self.config.cell_height;

        for (gy, glyph_row) in glyph.rows.iter().enumerate() {
            let py = y0 + gy as u32;
            if py >= image_width / 3 * image_width {
                // Safety: skip out-of-bounds rows (should not happen with
                // well-formed inputs).
                continue;
            }
            for gx in 0..self.config.cell_width.min(8) {
                if (glyph_row >> (7 - gx)) & 1 == 1 {
                    let px = x0 + gx;
                    let offset = ((py * image_width + px) * 3) as usize;
                    if offset + 2 < pixels.len() {
                        pixels[offset] = self.config.foreground[0];
                        pixels[offset + 1] = self.config.foreground[1];
                        pixels[offset + 2] = self.config.foreground[2];
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Metadata about a successfully rendered PNG.
#[derive(Debug, Clone)]
pub struct PngOutput {
    /// Path to the written PNG file.
    pub path: std::path::PathBuf,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Number of character columns in the source grid.
    pub grid_cols: usize,
    /// Number of character rows in the source grid.
    pub grid_rows: usize,
}

// ---------------------------------------------------------------------------
// Built-in bitmap font (minimal 8x16)
// ---------------------------------------------------------------------------

/// A single glyph rendered as an 8-pixel-wide bitmap.
struct GlyphBitmap {
    /// One byte per row; bit 7 is the leftmost pixel.
    rows: [u8; 16],
}

/// Return a minimal bitmap glyph for a character.
///
/// Printable ASCII characters get a crude but recognizable glyph. Everything
/// else falls back to a filled rectangle placeholder.
///
/// TODO(#151): Replace with `fontdue` glyph rasterization when image
/// dependencies are added.
fn builtin_glyph(ch: char) -> GlyphBitmap {
    // For the initial API surface we provide a simple "block" representation:
    // printable characters get a centered dot pattern, non-printable characters
    // get a filled block. This is intentionally low-fidelity -- the purpose is
    // to prove the rasterization pipeline, not to produce beautiful output.
    if ch.is_ascii_graphic() {
        // Crude: illuminate a small region in the center of the cell so that
        // character presence is visible in the rendered PNG. Each glyph is
        // identical, but the spatial layout of the text grid is preserved.
        GlyphBitmap {
            rows: [
                0b0000_0000,
                0b0000_0000,
                0b0000_0000,
                0b0000_0000,
                0b0011_1100,
                0b0111_1110,
                0b0111_1110,
                0b0111_1110,
                0b0111_1110,
                0b0111_1110,
                0b0111_1110,
                0b0011_1100,
                0b0000_0000,
                0b0000_0000,
                0b0000_0000,
                0b0000_0000,
            ],
        }
    } else {
        // Non-printable / non-ASCII: filled block placeholder.
        GlyphBitmap { rows: [0xFF; 16] }
    }
}

// ---------------------------------------------------------------------------
// Minimal PNG writer (no external crate)
// ---------------------------------------------------------------------------

/// Write raw RGB pixels as a minimal valid PNG file.
///
/// This avoids an `image` crate dependency. The output is uncompressed
/// (filter=None, zlib stored blocks) which is larger but correct and fast.
///
/// TODO(#151): Switch to `image::save_buffer` when the image crate is added.
fn write_minimal_png(path: &Path, width: u32, height: u32, rgb: &[u8]) -> Result<()> {
    use std::io::Write;

    let expected_len = (width * height * 3) as usize;
    anyhow::ensure!(
        rgb.len() == expected_len,
        "pixel buffer length mismatch: expected {expected_len}, got {}",
        rgb.len()
    );

    let mut file = std::fs::File::create(path)
        .with_context(|| format!("create PNG file: {}", path.display()))?;

    // PNG signature.
    file.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    // IHDR chunk.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type: RGB
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_png_chunk(&mut file, b"IHDR", &ihdr)?;

    // IDAT chunk: build uncompressed deflate stream wrapping filtered rows.
    // Each row is: filter_byte(0) + width * 3 bytes of RGB.
    let row_len = 1 + (width as usize) * 3;
    let raw_len = row_len * (height as usize);
    let mut idat_raw = Vec::with_capacity(raw_len);
    for y in 0..(height as usize) {
        idat_raw.push(0u8); // filter: None
        let row_start = y * (width as usize) * 3;
        let row_end = row_start + (width as usize) * 3;
        idat_raw.extend_from_slice(&rgb[row_start..row_end]);
    }

    // Wrap in a zlib stream using stored (uncompressed) blocks.
    let idat_compressed = zlib_store(&idat_raw);
    write_png_chunk(&mut file, b"IDAT", &idat_compressed)?;

    // IEND chunk.
    write_png_chunk(&mut file, b"IEND", &[])?;

    file.flush()?;
    Ok(())
}

/// Write a single PNG chunk: length (4 BE) + type (4) + data + CRC32 (4 BE).
fn write_png_chunk(
    writer: &mut impl std::io::Write,
    chunk_type: &[u8; 4],
    data: &[u8],
) -> Result<()> {
    let length = data.len() as u32;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(chunk_type)?;
    writer.write_all(data)?;

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    let crc = png_crc32(&crc_input);
    writer.write_all(&crc.to_be_bytes())?;
    Ok(())
}

/// CRC-32 as specified by the PNG specification (ISO 3309 / ITU-T V.42).
fn png_crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                if c & 1 != 0 {
                    c = 0xEDB8_8320 ^ (c >> 1);
                } else {
                    c >>= 1;
                }
            }
            t[n as usize] = c;
        }
        t
    });

    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc = table[((crc ^ u32::from(byte)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Wrap raw data in a minimal zlib stream using stored (uncompressed) blocks.
///
/// The format is: zlib header (2 bytes) + stored deflate blocks + Adler-32 (4 bytes).
fn zlib_store(data: &[u8]) -> Vec<u8> {
    // zlib header: CM=8 (deflate), CINFO=7 (32K window), FCHECK adjusted.
    let cmf: u8 = 0x78;
    let flg: u8 = 0x01; // FCHECK so that (CMF*256 + FLG) % 31 == 0
    let mut out = vec![cmf, flg];

    // Emit stored blocks. Each block can hold at most 65535 bytes.
    let mut offset = 0;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_len = remaining.min(65535);
        let is_final = offset + block_len >= data.len();
        out.push(if is_final { 0x01 } else { 0x00 }); // BFINAL + BTYPE=00 (stored)
        let len = block_len as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&data[offset..offset + block_len]);
        offset += block_len;
    }

    // Adler-32 checksum.
    let adler = adler32(data);
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

/// Adler-32 checksum used by zlib.
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ---------------------------------------------------------------------------
// Grid parsing
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences and split into a character grid.
fn parse_grid(buffer: &str) -> Vec<Vec<char>> {
    static ANSI_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = ANSI_RE
        .get_or_init(|| regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ANSI regex"));

    buffer
        .lines()
        .map(|line| {
            let stripped = re.replace_all(line, "");
            stripped.chars().collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_config_has_rosedust_colors() {
        let config = PngRendererConfig::default();
        assert_eq!(config.foreground, [165, 142, 158]);
        assert_eq!(config.background, [0, 0, 0]);
        assert_eq!(config.cell_width, 8);
        assert_eq!(config.cell_height, 16);
    }

    #[test]
    fn parse_grid_strips_ansi_and_splits_lines() {
        let input = "\x1b[31mhello\x1b[0m\nworld";
        let grid = parse_grid(input);
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0], vec!['h', 'e', 'l', 'l', 'o']);
        assert_eq!(grid[1], vec!['w', 'o', 'r', 'l', 'd']);
    }

    #[test]
    fn render_to_png_creates_valid_png_file() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.png");
        let renderer = PngRenderer::with_defaults();
        let result = renderer.render_to_png("AB\nCD", &output).unwrap();

        assert_eq!(result.grid_cols, 2);
        assert_eq!(result.grid_rows, 2);
        assert_eq!(result.width, 2 * 8 + 2 * 8); // 2 cols * cell_width + 2 * padding
        assert_eq!(result.height, 2 * 16 + 2 * 8); // 2 rows * cell_height + 2 * padding

        // Verify the file starts with PNG magic bytes.
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.len() > 8);
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn render_rejects_empty_buffer() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("empty.png");
        let renderer = PngRenderer::with_defaults();
        assert!(renderer.render_to_png("", &output).is_err());
    }

    #[test]
    fn png_crc32_matches_known_value() {
        // "IEND" chunk type CRC (empty data) is a well-known constant.
        let crc = png_crc32(b"IEND");
        assert_eq!(crc, 0xAE42_6082);
    }

    #[test]
    fn adler32_matches_known_value() {
        // adler32("Wikipedia") = 0x11E60398 (known test vector).
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn zlib_store_produces_valid_header() {
        let compressed = zlib_store(b"test");
        // zlib header: 0x78 0x01
        assert_eq!(compressed[0], 0x78);
        assert_eq!(compressed[1], 0x01);
        // Final stored block flag.
        assert_eq!(compressed[2], 0x01);
    }

    #[test]
    fn builtin_glyph_printable_is_not_all_zeros() {
        let glyph = builtin_glyph('A');
        assert!(glyph.rows.iter().any(|&row| row != 0));
    }

    #[test]
    fn builtin_glyph_nonprintable_is_filled() {
        let glyph = builtin_glyph('\u{200B}'); // zero-width space
        assert!(glyph.rows.iter().all(|&row| row == 0xFF));
    }
}
