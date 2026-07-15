# textual-rust

The **TextualRust form codec**: the two-way bridge between Rust TEXT and the
stringless [`core-logos`](https://github.com/LiGoldragon/core-logos) algebra. It
reads a principled subset of Rust *into* Logos through `syn`, and projects Logos
out as byte-exact `prettyplease` Rust.

Logos is 1-to-1 with Rust at the Core, and text-free. TextualRust is the Textual
form that gives that Core a Rust surface. This crate is the **foreign raw layer**
where `syn`, `prettyplease`, and `quote` are allowed; the Core sibling `core-logos`
depends on none of them.

## The two directions

- **Writer** (`src/project.rs`) — `ProjectRust` is one domain trait implemented per
  CoreLogos node kind (the verb-belongs-to-noun discipline). Each node projects the
  Rust tokens it already is; a read-only `NameResolver` is threaded to the
  identifier-bearing leaves. A single `prettyplease` pass owns all formatting.
- **Reader** (`src/read.rs`) — `ReadRust` maps the in-subset `syn` AST to CoreLogos,
  interning names through a `NameInterner`. Decode leans on `syn` and never
  re-implements Rust's grammar. Every out-of-subset construct is a **typed loud
  error naming the construct** — the reader never guesses and never skips.

## The five-synthesis rule

CoreLogos → Rust adds no meaning. Projection may synthesize exactly five things and
nothing else, each with one home:

1. **Dotted → `::`** on paths (`PathNode::project`).
2. **Delimiter re-sugaring** — the `(…)` of a newtype body, the `{…}` of a
   struct/enum body, the `<…>` of a generic application/parameter list, and the
   `(…)` of a derive list, each chosen by the owning node's kind.
3. **Stored-identifier realization** from the NameTable, at the two leaves.
4. **Formatting** — delegated entirely to the one `prettyplease` pass.
5. **The `// @generated` header** — a fixed module literal.

Everything semantic (which derives, which visibility, struct-vs-newtype, attribute
order, cfg predicates, field names, wrapped types) is present in CoreLogos and
transcribed, never materialized.

## The formatting-authority split

`prettyplease` is the **sole formatting authority**. The `project` methods own token
*structure and content* (which tokens, in what order, `::` materialized, names
realized); `prettyplease` owns *bytes* (whitespace, wrapping, indentation). Because
the schema-rust goldens are themselves `prettyplease` output, byte-exactness comes
from not competing with it. The one place this crate emits a comma the goldens'
width decisions depend on — the trailing comma inside a `derive(...)` list — is
emitted always; `prettyplease` keeps it when the derive wraps and strips it when the
derive stays on one line, so a single emission is byte-exact in both layouts.

## The subset boundary

The two-way subset is exactly what CoreLogos models: the four item kinds — newtype,
named-field struct, enum, type alias — over the witnessed
attribute/visibility/generic/type vocabulary. **Out of subset, failing loudly:**
trait definitions, impl blocks, free functions, `use` re-exports, modules, macros,
unions, const generics, reference/tuple/other non-path types, tuple newtypes with a
visibility-qualified field, and arbitrary expression/statement bodies. This is the
documented growth point: the trait/impl body vocabulary is where the subset grows,
if and when Logos models method bodies as data.

## The goldens are the acceptance oracle

`tests/fixtures/` holds byte-exact copies of schema-rust emission goldens (see
`tests/fixtures/PROVENANCE.md`), kept pristine as the oracle. The harness
(`tests/goldens_roundtrip.rs`) proves, over the copied corpus:

- **encode∘decode = identity on prettyplease-canonical text** — every in-subset
  golden item `decode → CoreLogos → encode` reproduces its byte-exact golden text.
  Coverage over the copied corpus: **153 in-subset items round-tripped byte-exact**
  (38 newtypes, 30 structs, 48 enums, 37 aliases) versus **304 out-of-subset
  frontier items** (254 impl blocks, 19 trait definitions, 14 modules, plus smaller
  edges). The impl/trait frontier is exactly the daemon-runtime and formulaic-impl
  surface CoreLogos does not model as data.
- **decode∘encode = identity on CoreLogos** — `core-logos`'s golden-pair fixtures
  `encode → decode` reproduce the CoreLogos value with a stable content identity;
  the hash does not move through text (`tests/core_roundtrip.rs`).
- **atomic decode** — a failed decode leaves the NameTable byte-identical, and each
  out-of-subset construct fails with the typed variant that names it
  (`tests/decode_atomicity.rs`).

## Building

```
nix flake check      # build, test, clippy, fmt, doc — the gate
cargo test           # inner-loop tests
```

Published as `0.1.0`. Consumes `core-logos`, `name-table`, and `content-identity`
as pinned git dependencies.
