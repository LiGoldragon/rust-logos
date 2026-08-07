# rust-logos

`rust-logos` emits Rust source from lowered `core_logos::WholeLogos` values.

The caller supplies a read-only `name_table::NameView`. Every declaration,
reference, and interface-role name is resolved through that view; this crate
does not mint, encode, project, or otherwise derive names from an identity.
Imported type references may additionally use caller-supplied canonical Rust
paths through `RustTypePathResolver`.

`RustLogos` is a stateless emitter. It preserves the ordering and typed shape
of the supplied Whole Logos items, emitting supported structs, enums, traits,
implementations, and tables. Rust-specific
spelling translation is intentionally local to emission: `Vector` is emitted
as `Vec`, including when it is the head of a type application.

Run the checks with:

```sh
CARGO_BUILD_JOBS=2 cargo test --all-targets
```
