# Plan: GemSig, signature format 3 for `.poweroynix`

Status: **implemented** (items 1–4 of “Work” below) in
`stonedgem-poweroynix` (`verify_info`, `gemprint`) and `tools/poweroynix.py`;
`sign` writes format 3 by default. Item 5, the ML-DSA co-signature, is still
waiting for a reviewed Rust implementation.

As built:

- The message is `OyNIx GemSig v3\n`, `id: …\n`, `version: …\n`
  (oynix.json's `id`, or `name`, and `version`), `commit: …\n`, then
  `<SHA3-512 hex>  <path>\n` per file. Ed25519 signs its SHA3-512 summary.
- `oynix.sig` also states `id` and `version`; if they differ from oynix.json
  the package is refused as “made for X, not Y”.
- GemPrint is the formula over exact big integers (see
  `crates/stonedgem-poweroynix/src/gemprint.rs` for every step), shown as
  four hex groups. Fixed test vectors pin it, and the tests check the Rust
  and Python versions give the same value for the same package.

## What was asked

A hash for verifying power extensions, built as

```
round(SHA3-256 × SHA3-256 × √(seed(0..3750) + 1))
  ÷ (1.5 × xoshiro256++) ÷ (current commit) × 0.37125²
  ÷ (rand(0..100) + 1)
→ a custom SHA3-like construction with a 512-bit key
```

## Why it can't be the check as written

A verifier works by **recomputing** the same value from the package on the
user's machine and comparing. So it must be deterministic, keep every bit
of the input, and be something nobody can produce for altered files.

| Part | Problem |
|---|---|
| `seed(0..3750)`, `xoshiro256++`, `rand(0..100)` | Random: a different value each run, so nothing can be recomputed. If the random values are stored in the package to make them repeatable, an attacker copies them too, so they add no protection. xoshiro256++ is a fast statistical generator, not a cryptographic one. |
| `SHA3-256 × SHA3-256`, `√`, `÷`, `round` | Arithmetic on hash outputs throws information away: many different files land on the same number. That's a built-in collision, the one property a verifier must not have. The values are 256-bit integers (their product is 512 bits), far beyond what floating point holds, so rounding loses even more. |
| `× 0.37125²` | A constant factor; it changes nothing. |
| `÷ current commit` | Ties the value to the OyNIx build: every signed extension would stop verifying on the next update. |
| "custom SHA3-like construction" | New cryptography nobody has analysed; its strength is unknown. Security comes from studied designs. |
| "512-bit **key**" used to verify | A keyed hash (MAC) is checked with the **same** secret key that makes it. Shipping that key inside OyNIx would let anyone extract it and sign whatever they like. Public verification needs a **signature**: a private key signs, a public key checks. |

## What GemSig does instead (format 3)

Same goals — longer hashes, a 512-bit key, something recognisably
OyNIx's own — from studied parts:

1. **SHA3-512 per file.** Every file's digest becomes SHA3-512 (512-bit),
   one line per file as today: `<128 hex digits>  <path>`.
2. **Domain separation.** The signed message starts with
   `OyNIx GemSig v3\n` plus the extension's id and version, so a signature
   for one package or format can never be replayed as another.
3. **Signature over a SHA3-512 summary.** The message is hashed with
   SHA3-512, then signed with **Ed25519**. Its signing key is a 512-bit
   (64-byte) keypair: 256-bit seed plus 256-bit public key.
4. **Optional post-quantum co-signature.** A second signature with
   **ML-DSA-65** (FIPS 204) over the same summary. Packages carry both, and
   OyNIx accepts a package only if every signature present verifies. This
   is the real way to "go beyond" Ed25519; it waits until a well-reviewed
   Rust implementation is available.
5. **The formula as GemPrint, display only.** Your formula, made
   deterministic, becomes the package's short visual fingerprint, shown next
   to "Verified by …":
   - its seeds come from the SHA3-512 summary itself (xoshiro256++ seeded
     from it, the 0..3750 and 0..100 draws taken from that generator);
   - the "commit" is the commit recorded **in the signature file** at signing
     time, so it doesn't change with OyNIx updates;
   - the arithmetic uses exact big integers, rounded once at the end;
   - the result is shown as eight words or a colour pattern.

   GemPrint is never what decides whether something installs; the signature
   does. It's for people comparing two packages at a glance.

## Compatibility

- Formats 1 and 2 keep verifying. Format 3 is what signers produce once
  it ships.
- `tools/poweroynix.py` gains format 3 (Python has SHA3-512 built in; the
  Ed25519 code is already there).
- `oynix.sig` gets `"format": 3`, `"id"`, `"version"`, `"commit"` (for
  GemPrint) and optionally `"mldsa"`.

## Work, when agreed

1. `stonedgem-poweroynix`: format 3 digest and verification, with tests
   (tamper, wrong id, wrong version, format-2 package still verifies).
2. Same in `tools/poweroynix.py`, plus a Rust↔Python cross-check test.
3. GemPrint as a separate function with fixed test vectors, so its output
   never drifts between versions.
4. OyNIx shows GemPrint in Settings › Extensions and on the install prompt.
5. ML-DSA co-signature behind a feature flag, once a reviewed crate is chosen.
