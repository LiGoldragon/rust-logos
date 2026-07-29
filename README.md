# rust-logos

`rust-logos` is the structural Rust TextualForm for Logos.

The first slice supports ordered, attribute-free tuple newtypes. It discovers
Rust `struct` blocks by inclusive cue and terminating semicolon, then decodes
the exact name and wrapped-type bounds through fully typed position records in
the shared structural evaluator. Emission reflects those same records and
produces compilable Rust without a syntax tree library or legacy renderer.

Names owned by Logos remain complete `VocabularyRoot::Universal` encoded-ID
chains. Production emission (`RustLogos::emit`) computes its own chain-to-text
scheme: `RustEncodedIdCodec` encodes the complete root-fronted encoded-ID
chain (format version byte, root tag byte, then each table-local `u16` in
big-endian order) as Base58BTC with a `z` multibase prefix, producing a
compact, injective, rustc-safe identifier with no depth cap. This is the
landed answer to DRR entry 7's "a textual version of [the encoded identity] -
some kind of textual binary encoding which is friendly to rustc"; DRR entry
17 rejected an earlier fixed-width-decimal proposal and confirmed the exact
codec is implementation matter, not itself a psyche ruling — the ruled
direction is only that emitted Rust encode the complete identity in a
compact, readable textual-binary form accepted by rustc. The older
fixture-only path — callers supplying opaque, rustc-safe tokens through
`RustNameProjectionTable` — remains available for fixture witnesses; it is
not the production naming path.

Rust-owned words are read through a sealed, lookup-only
`VocabularyRoot::Rust` vocabulary. `struct` and `pub` are exact immutable
entries. This crate exposes no Rust-vocabulary allocation, rename, removal, or
rebinding operation.

The public contract is intentionally narrow:

- `RustNewtypeVocabulary::seal` validates caller-issued Rust-root identities
  and seals the typed structuretree.
- `RustLogos::decode` performs two-pass, source-bounded decoding into
  `core_logos::WholeLogos`.
- `RustLogos::emit` structurally emits the ordered `WholeLogos` newtypes.
- `RustEmittedIdentifier` validates the conservative ASCII Rust identifier
  subset; `RustNameProjectionTable` rejects identity or token collisions.

The durable gate is `cargo test --test slice_one`; the Nix flake also checks
formatting, clippy, documentation, build, and tests.
