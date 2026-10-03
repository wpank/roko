//! `roko showcase`: the operator side of the public showcase (S11).
//!
//! `passphrase new` prints a fresh passphrase once, then its Argon2id PHC string, the value of
//! the `ROKO_SHOWCASE_PASSPHRASE_HASH` deploy secret. `passphrase hash` hashes a passphrase read
//! from stdin, for rotation (S11 §4.3). Both write to stdout only: neither puts the passphrase or
//! its hash on disk or in the log.

use std::io::Read as _;

use anyhow::{Result, anyhow};
use argon2::password_hash::rand_core::{OsRng, RngCore as _};
use argon2::password_hash::{PasswordHasher as _, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use clap::Subcommand;

use crate::{EXIT_FAILURE, EXIT_SUCCESS};

/// The public showcase's operator commands.
#[derive(Debug, Subcommand)]
pub(crate) enum ShowcaseCmd {
    /// Make or hash the login passphrase.
    Passphrase {
        #[command(subcommand)]
        cmd: PassphraseCmd,
    },
}

/// `roko showcase passphrase`.
#[derive(Debug, Subcommand)]
pub(crate) enum PassphraseCmd {
    /// Print a new 100-bit passphrase once, then its Argon2id PHC string.
    New,
    /// Read a passphrase from stdin and print its Argon2id PHC string.
    Hash,
}

/// Argon2id memory cost in KiB: 19 MiB, OWASP's minimum (S11 §4.3).
const MEMORY_KIB: u32 = 19 * 1024;
/// Argon2id passes over that memory.
const PASSES: u32 = 2;
/// Argon2id lanes.
const LANES: u32 = 1;

/// The passphrase lengths, in bytes, that the showcase login accepts (S11 §4.3).
const LOGIN_BYTES: std::ops::RangeInclusive<usize> = 12..=256;

/// A new passphrase's characters: Crockford's base32 in lower case, which leaves out i, l, o and
/// u, so it reads back without mistakes. A random byte modulo 32 picks one uniformly.
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
/// A new passphrase's groups, and characters per group: 20 characters of 5 bits, 100 bits.
const GROUPS: usize = 4;
const GROUP_CHARS: usize = 5;

pub(crate) fn cmd_showcase(cmd: ShowcaseCmd) -> Result<i32> {
    let ShowcaseCmd::Passphrase { cmd } = cmd;
    match cmd {
        PassphraseCmd::New => {
            let passphrase = new_passphrase();
            let hash = hash_passphrase(&passphrase)?;
            println!("{passphrase}");
            println!("{hash}");
        }
        PassphraseCmd::Hash => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let passphrase = one_line(&input);
            if passphrase.is_empty() {
                eprintln!("no passphrase on stdin");
                return Ok(EXIT_FAILURE);
            }
            if !LOGIN_BYTES.contains(&passphrase.len()) {
                eprintln!(
                    "warning: the showcase login takes 12 to 256 bytes; this passphrase has {}",
                    passphrase.len()
                );
            }
            println!("{}", hash_passphrase(passphrase)?);
        }
    }
    Ok(EXIT_SUCCESS)
}

/// `input` without the line ending `echo` or a terminal adds.
fn one_line(input: &str) -> &str {
    let line = input.strip_suffix('\n').unwrap_or(input);
    line.strip_suffix('\r').unwrap_or(line)
}

/// A new passphrase from the operating system's generator: four groups of five characters,
/// `xxxxx-xxxxx-xxxxx-xxxxx`, 100 bits.
fn new_passphrase() -> String {
    let mut bytes = [0_u8; GROUPS * GROUP_CHARS];
    OsRng.fill_bytes(&mut bytes);
    let chars: Vec<char> = bytes
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte % 32)]))
        .collect();
    chars
        .chunks(GROUP_CHARS)
        .map(|group| group.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// The Argon2id PHC string of `passphrase` with a fresh salt:
/// `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`.
fn hash_passphrase(passphrase: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = hasher()?
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|error| anyhow!("hash the passphrase: {error}"))?;
    Ok(hash.to_string())
}

/// Argon2id with the showcase's parameters.
fn hasher() -> Result<Argon2<'static>> {
    let params = Params::new(MEMORY_KIB, PASSES, LANES, None)
        .map_err(|error| anyhow!("Argon2 parameters: {error}"))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

#[cfg(test)]
mod tests {
    use argon2::password_hash::{PasswordHash, PasswordVerifier as _};

    use super::*;

    /// 9321: a hash verifies the passphrase it was made from and no other, and each hash has a
    /// salt of its own.
    #[test]
    fn passphrase_hash_round_trips() {
        let passphrase = new_passphrase();
        let phc = hash_passphrase(&passphrase).expect("hash");
        let parsed = PasswordHash::new(&phc).expect("a PHC string");
        let verifier = Argon2::default();
        assert!(verifier.verify_password(passphrase.as_bytes(), &parsed).is_ok());
        assert!(verifier.verify_password(b"another passphrase", &parsed).is_err());
        assert_ne!(hash_passphrase(&passphrase).expect("hash"), phc);
        assert_eq!(one_line("pw\r\n"), "pw");
    }

    /// The PHC string names Argon2id, version 19, and S11's parameters; a new passphrase is four
    /// groups of five base32 characters, a length the login accepts.
    #[test]
    fn passphrase_hash_uses_the_showcase_parameters() {
        let phc = hash_passphrase("pw").expect("hash");
        assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
        let parsed = PasswordHash::new(&phc).expect("a PHC string");
        assert_eq!(parsed.algorithm.as_str(), "argon2id");
        assert_eq!(parsed.version, Some(19));

        let passphrase = new_passphrase();
        let groups: Vec<&str> = passphrase.split('-').collect();
        assert_eq!(groups.len(), GROUPS, "{passphrase}");
        for group in &groups {
            assert_eq!(group.len(), GROUP_CHARS, "{passphrase}");
            assert!(group.bytes().all(|byte| ALPHABET.contains(&byte)), "{passphrase}");
        }
        assert!(LOGIN_BYTES.contains(&passphrase.len()));
        assert_ne!(new_passphrase(), passphrase);
    }

    /// `roko showcase passphrase new` and `roko showcase passphrase hash` reach this module.
    #[test]
    fn showcase_passphrase_commands_parse() {
        use clap::Parser as _;

        for word in ["new", "hash"] {
            let argv = ["roko", "showcase", "passphrase", word];
            let cli = crate::Cli::try_parse_from(argv).expect("parse");
            let Some(crate::Command::Showcase { cmd }) = cli.command else {
                panic!("{argv:?} is not `roko showcase`");
            };
            let ShowcaseCmd::Passphrase { cmd } = cmd;
            match cmd {
                PassphraseCmd::New => assert_eq!(word, "new"),
                PassphraseCmd::Hash => assert_eq!(word, "hash"),
            }
        }
    }
}
