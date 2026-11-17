//! GemPrint: a package's short visual fingerprint, **display only**.
//!
//! It is the formula
//!
//! ```text
//! round(H·H·√(seed+1) ÷ (1.5·x) ÷ c × 0.37125² ÷ (r+1))
//! ```
//!
//! made deterministic so it can be recomputed anywhere:
//!
//! * `H` is SHA3-256 of the signed summary, as a 256-bit integer;
//! * xoshiro256++, seeded from the summary's first 32 bytes, draws
//!   `seed` ∈ 0..=3750, `x` (non-zero) and `r` ∈ 0..=100, in that order;
//! * `c` is the commit recorded in `oynix.sig` at signing time (its first 8
//!   bytes as hex, or of SHA3-256 of it when it isn't hex), so it never
//!   changes with browser updates;
//! * the arithmetic is exact: √ in 64-bit fixed point, 0.37125² as
//!   88209/640000, one half-up rounding at the end.
//!
//! The low 64 bits are shown as `a1b2-c3d4-e5f6-0718`. Whether a package
//! installs is decided only by its signature; GemPrint lets people compare
//! two packages at a glance. `tools/poweroynix.py` has the same function,
//! and the test vectors below pin both.

use num_bigint::BigUint;
use sha3::Digest;

/// xoshiro256++ (Blackman & Vigna). Not cryptographic; used only to make
/// GemPrint's draws repeatable.
pub struct Xoshiro256pp([u64; 4]);

impl Xoshiro256pp {
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let mut s = [0u64; 4];
        for (i, w) in s.iter_mut().enumerate() {
            *w = u64::from_le_bytes(seed[8 * i..8 * i + 8].try_into().unwrap());
        }
        if s == [0; 4] {
            s[0] = 1;
        }
        Xoshiro256pp(s)
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }
}

fn commit_number(commit: &str) -> u64 {
    let from_hex = if commit.len() >= 16 {
        (0..16).step_by(2).map(|i| u8::from_str_radix(commit.get(i..i + 2)?, 16).ok()).collect::<Option<Vec<u8>>>()
    } else {
        None
    };
    let bytes = from_hex.unwrap_or_else(|| sha3::Sha3_256::digest(commit.as_bytes())[..8].to_vec());
    u64::from_be_bytes(bytes[..8].try_into().unwrap()).max(1)
}

/// The formula's exact value for a signed summary (64 bytes) and commit.
pub fn value(summary: &[u8; 64], commit: &str) -> BigUint {
    let h = BigUint::from_bytes_be(&sha3::Sha3_256::digest(summary));
    let mut rng = Xoshiro256pp::from_seed(summary[..32].try_into().unwrap());
    let seed = rng.next_u64() % 3751;
    let x = rng.next_u64().max(1);
    let r = rng.next_u64() % 101;
    let c = commit_number(commit);
    let sqrt = (BigUint::from(seed + 1) << 128u32).sqrt();
    let num = &h * &h * sqrt * 2u32 * 88209u32;
    let den = (BigUint::from(3u32) * x * c * 640000u32 * (r + 1)) << 64u32;
    (num * 2u32 + &den) / (den * 2u32)
}

/// GemPrint as shown to people: `a1b2-c3d4-e5f6-0718`.
pub fn gemprint(summary: &[u8; 64], commit: &str) -> String {
    let low = value(summary, commit).iter_u64_digits().next().unwrap_or(0);
    let h = format!("{low:016x}");
    format!("{}-{}-{}-{}", &h[0..4], &h[4..8], &h[8..12], &h[12..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoshiro_matches_reference() {
        let mut seed = [0u8; 32];
        for (i, v) in [1u64, 2, 3, 4].iter().enumerate() {
            seed[8 * i..8 * i + 8].copy_from_slice(&v.to_le_bytes());
        }
        let mut r = Xoshiro256pp::from_seed(&seed);
        assert_eq!([r.next_u64(), r.next_u64(), r.next_u64()], [41943041, 58720359, 3588806011781223]);
    }

    #[test]
    fn fixed_vectors() {
        // Pinned; tools/poweroynix.py gives the same. Changing these means
        // every published GemPrint changes: don't.
        let s: [u8; 64] = sha3::Sha3_512::digest(b"GemPrint test vector").into();
        assert_eq!(gemprint(&s, "0123456789abcdef0123456789abcdef01234567"), "29e6-e4af-2c14-bcf6");
        assert_eq!(gemprint(&s, ""), "1c18-d6c9-2a77-ce05");
        assert_eq!(gemprint(&s, "v1.2"), "3500-0343-4223-92c4");
        assert_eq!(value(&s, "").bits(), 376);
    }
}
