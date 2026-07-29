# Architecture

## Boundary

`rust-logos` is a TextualForm boundary. Logos stays typed and stringless; Rust
text exists only while this component discovers or emits the interim
programming interface.

There is no Rust Capsule here. The input and output content is
`core_logos::WholeLogos`, whose complete Universal encoded-ID chains are
preserved unchanged.

## Two passes

Pass one is `raw-discovery` cue-to-termination discovery. `struct` is an
inclusive cue, `;` terminates the top-level item, delimiter children are
recursive, strings and comments are opaque, and every node carries exact
source bounds.

Pass two is expectation-driven. Four actual typed position records describe
the Slice One vocabulary:

- fixed `struct` keyword;
- fixed `pub` keyword;
- declaration name;
- referenced wrapped type.

Each record implements `StructureRecord` and is consumed by the one shared
`StructuralEvaluator`. Declaration positions require translator-issued
assignments. Reference positions use lookup-only resolution. The two equal
lexical shapes are deliberately seated under different expected types; placing
them as alternatives under one type is conservatively refused as overlapping.

The one Rust-specific `RustLogos` object only orchestrates source bounds that
the shared evaluator's current general record vocabulary cannot yet express:
Rust adjacency (`Name(Type)`) and fixed punctuation. It does not tokenize
general Rust, select structural alternatives by order, mint identities, or
walk a separate syntax tree.

## Names

The structuretree and fixed words are addressed through immutable Rust-root
encoded IDs supplied by the caller. The component validates their root and
exact spelling while sealing, then retains only a read-only fixed-word view.

Universal names are not emitted by resolving their human spelling. The caller
must associate each complete encoded-ID chain with an opaque
`RustEmittedIdentifier`. `RustNameProjectionTable` validates the token and
rejects mappings in which one identity has two tokens or two identities share
one token. No algorithm derives the token from the chain.

## Emission

Every semantic token is reflected through the shared evaluator. `RustLogos`
then assembles only Rust punctuation and canonical spacing:

```text
pub struct <opaque-name>(<opaque-reference>);
```

Item and wrapped-field visibility remain typed data, so either `pub` position
may be absent. Attributes, generics, named fields, multiple tuple fields, and
other Rust items are refused.

Acceptance is behavioral: generated Rust is compiled and run by the public
test. Byte identity with any legacy renderer is not a contract.
