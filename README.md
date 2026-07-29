# rust-logos

`rust-logos` is the structural Rust TextualForm for Logos.

The first slice supports ordered, attribute-free tuple newtypes. It discovers
Rust `struct` blocks by inclusive cue and terminating semicolon, then decodes
the exact name and wrapped-type bounds through fully typed position records in
the shared structural evaluator. Emission reflects those same records and
produces compilable Rust without a syntax tree library or legacy renderer.

Names owned by Logos remain complete `VocabularyRoot::Universal` encoded-ID
chains. This crate defines no chain-to-text scheme. Callers provide opaque,
rustc-safe tokens through `RustNameProjectionTable`; the table validates each
token and retains its association to the complete chain.

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
