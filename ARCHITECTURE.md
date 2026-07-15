# textual-rust architecture

`textual-rust` is slice three, part two of the psyche-authorized language-family
proof of concept: the **TextualRust form codec**, the two-way bridge between Rust
text and the stringless `core-logos` algebra. This document records the durable
direction — the rulings the crate embodies and the boundaries it holds — for an
agent entering the repository.

## The one shape: a codec, not a one-way emitter

Logos is 1-to-1 with Rust at the Core, and projection is transcription, so the
Rust edge is inherently a **codec**. The writer (CoreLogos → Rust) and the reader
(Rust → CoreLogos) are inverses over the same principled subset. This crate is
where Rust text lives; the Core sibling `core-logos` is text-free and depends on no
`syn`, `prettyplease`, or `quote`. Keeping text here is what lets one Core be
viewed through many textual forms without any of them reaching into the Core.

## Verb belongs to noun, both directions

There is no free-function emitter and no free-function parser walker. The writer is
one domain trait, `ProjectRust`, implemented once per CoreLogos node kind; the
reader is one domain trait, `ReadRust`, implemented on the `syn` AST node being
read. Composition helpers are methods on the noun they concern (the attribute-vector
preamble, the list-attribute readers, the type-position path reader). Totality is
mechanical: the closed `CoreItem` enum is matched with no wildcard arm, and every
child slot is itself a projecting/reading node, so a missing case is a compile
error, not a silent skip.

## The five-synthesis rule (writer)

CoreLogos → Rust adds no meaning. Projection may synthesize exactly five things,
each with one home: dotted → `::` (`PathNode::project`); delimiter re-sugaring (the
body/argument brackets, chosen by node kind); stored-identifier realization (the two
identifier leaves); formatting (the single `prettyplease` pass); and the
`// @generated` header (a fixed module literal). Everything semantic is stored data
transcribed in order — visibility, the derive sets, struct-vs-newtype, attribute
order, cfg predicates, field names, wrapped types.

## The formatting-authority split

`prettyplease` is the sole formatting authority. The `project` methods produce a
faithful `proc_macro2::TokenStream` — token structure and content — and one
`prettyplease::unparse` per item owns every byte of whitespace, wrapping, and
indentation. Because the schema-rust goldens are themselves `prettyplease` output,
byte-exactness reduces to "does the Core carry the right data and does each node
emit the right tokens." The single subtlety is the trailing comma inside a
`derive(...)` list: `prettyplease` preserves it when the derive wraps to multiple
lines (the width-heavy goldens) and strips it when the derive stays on one line, so
the writer emits it always and is byte-exact in both layouts. This is the one place
the writer must know a `prettyplease` invariant; it is documented at the emission
site rather than reverse-engineered per item.

## The NameTable contact point is bidirectional

Stringlessness meets text at exactly two leaves — the identifier and each path
segment. Encode **resolves** `Identifier → &str` through a read-only `NameResolver`;
decode **interns** `&str → Identifier` through a `NameInterner`, allocating into the
one continuous identifier space. Decode interns through a `NameTransaction`, so a
failed decode alternative leaves the committed table byte-identical — the
interning-atomicity law extended to the Rust edge. The `.`↔`::` translation is
likewise two-way and lives in the one `PathNode` shape.

## The subset boundary, stated precisely

The two-way subset is exactly the constructs CoreLogos models as data:

- **In subset (byte-exact two-way):** tuple newtypes with an inherited-visibility
  field, named-field structs (fields carrying their own stored visibility), unit /
  tuple-payload / struct-payload enums, type aliases, generic parameters by kind
  (type parameters with path bounds, lifetime parameters), and the witnessed
  attribute vocabulary — a bare dotted tool path, a `derive(...)` group, a
  `cfg_attr(feature = "...", …)` wrapper, and a namespaced helper `derive`.
- **Out of subset (fails loudly, typed):** trait definitions, impl blocks, free
  functions, `use` re-exports, modules, macros, unions, const generics, `where`
  clauses, non-path types (references, tuples, trait objects, …), non-feature cfg
  predicates, name-value attributes, tuple newtypes with a visibility-qualified
  field, and enum variants with attributes or discriminants. Each is a distinct
  `Error` variant naming the construct; the decoder never guesses and never skips.

## The trait/impl frontier is the documented growth point

The out-of-subset frontier over the copied golden corpus is dominated by impl blocks
(the `From`/constructor/`Display` formulaic impls) and trait definitions — the
daemon-runtime and formulaic-impl surface whose method bodies are arbitrary Rust
logic Logos does not yet model as data. This is the honest edge, not a defect: the
subset grows precisely when (and if) Logos gains a body vocabulary. A new modeled
construct is one CoreLogos node plus one `project` arm plus one `read` arm, with no
central walker to thread.

## Verification and the acceptance oracle

The schema-rust goldens gate the codec both directions:

- `encode∘decode = identity` on prettyplease-canonical text — every in-subset golden
  item round-trips byte-exact against its original golden bytes.
- `decode∘encode = identity` on CoreLogos — the golden-pair fixtures round-trip with
  a stable content identity; the hash does not move through text.

Durable test evidence is owned by Nix: `nix flake check` runs build, test, clippy
(`-D warnings`), fmt, and doc as the green gate. Bare `cargo test` is inner-loop
evidence.

## Train status

`0.1.0`. Git-pins its dependencies (`core-logos`, `name-table`, `content-identity`)
at reviewed revisions — the green path. Colocated jj + git; published under
`LiGoldragon`.
