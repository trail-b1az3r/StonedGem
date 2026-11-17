# StonedGem

A modular browser platform in Rust, without Chromium or Qt. It will be the
base of **OyNIx 5.4.0** (and later PowerOyNix): Servo for web content, Iced
for the browser window, Slint for extension windows, and a Rust extension
runtime. OyNIx moves onto it one piece at a time.

**Status:** early. What exists today is the set of engine-independent
services already moved out of OyNIx — and OyNIx uses them:

| Crate | What it does |
|---|---|
| [`stonedgem-url`](crates/stonedgem-url) | Address-bar logic: typed text → URL or search, `!bangs`, display forms, tracking-parameter removal that never touches sign-ins |
| [`stonedgem-poweroynix`](crates/stonedgem-poweroynix) | `.poweroynix` signature checking (Ed25519 over SHA3-256 file hashes) |
| [`stonedgem-vault`](crates/stonedgem-vault) | Encrypted password store, CSV import, Firefox passwords and bookmarks |

The engine, the chrome and the extension runtime are planned, not built.
Nothing is described as working until its code and tests show it.

## Documents

- [Architecture](docs/ARCHITECTURE.md) — layers, the engine decision, Iced/Slint, extensions
- [Migration plan](docs/MIGRATION.md) — how OyNIx moves over, phase by phase
- [Compatibility](docs/COMPATIBILITY.md) — what works, what's planned, what isn't supported
- [Z# and Z#++](docs/ZSHARP.md)
- [The original brief](prompt.md)

## Building

```bash
cargo test --workspace
```

Rust 1.80 or newer. The vault bundles SQLite (built by the `rusqlite`
crate), so a C compiler is needed. C++ code, when it arrives, targets C++23.

`tools/poweroynix.py` makes signing keys, signs and verifies `.poweroynix`
packages; it needs only Python 3.

## Licence

MIT. Servo, when it's added, is MPL-2.0.
