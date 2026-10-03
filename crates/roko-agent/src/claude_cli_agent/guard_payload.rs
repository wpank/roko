//! The guard script packed for a hook command line (1223).
//!
//! The `--settings` argument carries the guard script once per hook, and
//! the script alone is about 75 KB. Linux refuses one argument longer than
//! `MAX_ARG_STRLEN` (128 KiB) with `E2BIG`, so each hook carries the script
//! zlib-compressed and base64-encoded, which Python's standard `zlib` and
//! `base64` unpack. The encoder writes a zlib stream (RFC 1950) holding one
//! DEFLATE block (RFC 1951) with the fixed Huffman codes, from greedy LZ77
//! matches: about a third of the script's size, with no compression crate.

/// How far back a match may start: DEFLATE's window.
const WINDOW: usize = 32 * 1024;
/// The shortest match DEFLATE encodes.
const MIN_MATCH: usize = 3;
/// The longest match DEFLATE encodes.
const MAX_MATCH: usize = 258;
/// How many earlier positions with the same hash a match search tries.
const MAX_CHAIN: usize = 256;
/// Bits of the hash of the three bytes at a position.
const HASH_BITS: u32 = 15;
/// No earlier position.
const NONE: usize = usize::MAX;

/// The shortest length of each length code, 257 to 285.
const LENGTH_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// The extra bits of each length code.
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// The shortest distance of each distance code, 0 to 29.
const DISTANCE_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049,
    3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// The extra bits of each distance code.
const DISTANCE_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// `data` zlib-compressed and base64-encoded, for Python's
/// `zlib.decompress(base64.b64decode(...))`.
pub(super) fn zlib_base64(data: &[u8]) -> String {
    base64(&zlib(data))
}

/// `data` as a zlib stream: the header (DEFLATE with a 32 KiB window, no
/// preset dictionary), the DEFLATE data, and the Adler-32 of `data`.
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.extend(deflate_fixed(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

/// The Adler-32 checksum of `data`.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

/// `data` in standard base64, padded.
fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let mut group = 0_u32;
        for (index, &byte) in chunk.iter().enumerate() {
            group |= u32::from(byte) << (16 - 8 * index);
        }
        for index in 0..=chunk.len() {
            let sextet = (group >> (18 - 6 * index)) & 63;
            out.push(char::from(ALPHABET[sextet as usize]));
        }
        for _ in chunk.len()..3 {
            out.push('=');
        }
    }
    out
}

/// `data` as one final DEFLATE block with the fixed Huffman codes.
fn deflate_fixed(data: &[u8]) -> Vec<u8> {
    let mut out = BitWriter::default();
    // BFINAL = 1 (the last block), then BTYPE = 01 (fixed codes).
    out.bits(1, 1);
    out.bits(1, 2);
    let mut chains = Chains::new(data.len());
    let mut pos = 0;
    while pos < data.len() {
        let (length, distance) = chains.longest_match(data, pos);
        let step = if length >= MIN_MATCH {
            write_match(&mut out, length, distance);
            length
        } else {
            write_symbol(&mut out, u32::from(data[pos]));
            1
        };
        for at in pos..pos + step {
            chains.insert(data, at);
        }
        pos += step;
    }
    // End of block.
    write_symbol(&mut out, 256);
    out.finish()
}

/// Append literal/length symbol `symbol` (0 to 287) in its fixed Huffman
/// code: 8 bits from 0b0011_0000 for 0 to 143, 9 bits from 0b1_1001_0000
/// for 144 to 255, 7 bits from 0 for 256 to 279, and 8 bits from
/// 0b1100_0000 for 280 to 287.
fn write_symbol(out: &mut BitWriter, symbol: u32) {
    match symbol {
        0..=143 => out.code(0b0011_0000 + symbol, 8),
        144..=255 => out.code(0b1_1001_0000 + symbol - 144, 9),
        256..=279 => out.code(symbol - 256, 7),
        _ => out.code(0b1100_0000 + symbol - 280, 8),
    }
}

/// Append a match of `length` bytes (3 to 258) that starts `distance` bytes
/// back (1 to 32768): its length symbol and extra bits, then its distance
/// code (five bits) and extra bits.
fn write_match(out: &mut BitWriter, length: usize, distance: usize) {
    let code = code_for(&LENGTH_BASE, length);
    write_symbol(out, 257 + code as u32);
    out.bits((length - LENGTH_BASE[code]) as u32, LENGTH_EXTRA[code]);
    let code = code_for(&DISTANCE_BASE, distance);
    out.code(code as u32, 5);
    out.bits(
        (distance - DISTANCE_BASE[code]) as u32,
        DISTANCE_EXTRA[code],
    );
}

/// The index of the last of `bases` that is at most `value`.
fn code_for(bases: &[usize], value: usize) -> usize {
    bases.iter().rposition(|&base| base <= value).unwrap_or(0)
}

/// Bits in DEFLATE's order: each byte is filled from its least significant
/// bit.
#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    pending: u64,
    count: u32,
}

impl BitWriter {
    /// Append the low `count` bits of `value`, least significant first, as
    /// DEFLATE writes block headers and extra bits.
    fn bits(&mut self, value: u32, count: u32) {
        self.pending |= u64::from(value) << self.count;
        self.count += count;
        while self.count >= 8 {
            self.out.push(self.pending as u8);
            self.pending >>= 8;
            self.count -= 8;
        }
    }

    /// Append the `len`-bit Huffman code `code`, most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        self.bits(code.reverse_bits() >> (32 - len), len);
    }

    /// The bytes written, the last one padded with zero bits.
    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.out.push(self.pending as u8);
        }
        self.out
    }
}

/// Earlier positions by the hash of the three bytes there: the latest
/// position per hash, and for each position the one before it with the
/// same hash.
struct Chains {
    head: Vec<usize>,
    prev: Vec<usize>,
}

impl Chains {
    fn new(len: usize) -> Self {
        Self {
            head: vec![NONE; 1 << HASH_BITS],
            prev: vec![NONE; len],
        }
    }

    /// The hash of the three bytes at `pos`, when there are three.
    fn hash(data: &[u8], pos: usize) -> Option<usize> {
        let bytes = data.get(pos..pos + MIN_MATCH)?;
        let value = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        Some((value.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize)
    }

    /// Record position `pos`.
    fn insert(&mut self, data: &[u8], pos: usize) {
        if let Some(hash) = Self::hash(data, pos) {
            self.prev[pos] = self.head[hash];
            self.head[hash] = pos;
        }
    }

    /// The longest match for the bytes at `pos` among the recorded
    /// positions, as (length, distance); a length under [`MIN_MATCH`] means
    /// none.
    fn longest_match(&self, data: &[u8], pos: usize) -> (usize, usize) {
        let Some(hash) = Self::hash(data, pos) else {
            return (0, 0);
        };
        let ahead = &data[pos..data.len().min(pos + MAX_MATCH)];
        let mut best = (0, 0);
        let mut candidate = self.head[hash];
        for _ in 0..MAX_CHAIN {
            if candidate == NONE || pos - candidate > WINDOW {
                break;
            }
            let length = data[candidate..]
                .iter()
                .zip(ahead)
                .take_while(|(earlier, later)| earlier == later)
                .count();
            if length > best.0 {
                best = (length, pos - candidate);
                if length == ahead.len() {
                    break;
                }
            }
            candidate = self.prev[candidate];
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Python's reading of `encoded`: `zlib.decompress(base64.b64decode(...))`.
    fn python_unpack(encoded: &str) -> Vec<u8> {
        let mut child = Command::new("python3")
            .arg("-c")
            .arg(
                "import base64,sys,zlib\n\
                 sys.stdout.buffer.write(zlib.decompress(base64.b64decode(sys.stdin.read())))",
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn python3");
        child
            .stdin
            .take()
            .expect("python3 stdin")
            .write_all(encoded.as_bytes())
            .expect("write the payload");
        let output = child.wait_with_output().expect("wait for python3");
        assert!(
            output.status.success(),
            "python3 could not unpack it: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// `len` bytes from a xorshift generator seeded with `seed`.
    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn adler32_matches_the_reference_value() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn base64_matches_the_rfc_4648_vectors() {
        for (data, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(data.as_bytes()), encoded, "{data:?}");
        }
    }

    /// Python gets back every input: empty and tiny ones, the longest
    /// matches (a long run), all 256 literals (noise), matches at the far
    /// end of the window, and the guard script itself.
    #[test]
    fn python_unpacks_what_zlib_base64_packs() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", Vec::new()),
            ("one byte", b"a".to_vec()),
            ("three bytes", b"abc".to_vec()),
            ("a run", vec![b'r'; 100_000]),
            ("noise", noise(11, 70_000)),
            ("the window apart", noise(7, WINDOW).repeat(3)),
            ("past the window", noise(5, WINDOW + 1).repeat(2)),
            ("the guard", super::super::GUARD_SCRIPT.as_bytes().to_vec()),
        ];
        for (name, data) in cases {
            assert!(python_unpack(&zlib_base64(&data)) == data, "{name}");
        }
    }

    #[test]
    fn the_guard_compresses_to_under_half_its_size() {
        let guard = super::super::GUARD_SCRIPT.as_bytes();
        let compressed = zlib(guard);
        assert!(
            compressed.len() < guard.len() / 2,
            "{} bytes compressed to {}",
            guard.len(),
            compressed.len()
        );
    }
}
