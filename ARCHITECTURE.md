# Architecture

## Boundary

`rust-logos` is a one-way textual emission boundary. Its input is a typed,
already-lowered `core_logos::WholeLogos` value plus a caller-owned read-only
`NameView`; its output is Rust source. The crate neither owns nor creates
names, metadata, or lowered semantic items.

## Emission

`RustLogos` walks the ordered Whole Logos items and emits each supported Rust
form. Type references resolve through the supplied `NameView`. If an external
reference has a canonical Rust path, `RustTypePathResolver` takes precedence.
Otherwise the resolved spelling is emitted directly, apart from the narrow
Rust translation `Vector` -> `Vec`. That translation applies to scalar,
parameter, and application-head references, while the arguments are emitted
recursively.

Interface-specific emission is kept behind `InterfaceRustEmission` and uses
the same read-only naming boundary. `InterfaceRustRoleIds` only identifies
the three roles to emit; it has no allocation or persistence behavior.

## Non-goals

This crate does not define a vocabulary root, an encoded-ID-to-Rust codec, or
a fixture projection table. It makes no object-identity claim for
`WholeLogos`; that carrier is semantic input, while archive compatibility is
owned and documented by `core-logos`.
