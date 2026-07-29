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
inclusive cue terminated by `;`; `enum` is an inclusive cue terminated after
its configured balanced brace boundary. Delimiter children are recursive,
strings and comments are opaque, and every node carries exact source bounds.

Pass two is expectation-driven. Complete archived position records describe
the fixture vocabulary:

- newtype and enumeration items;
- unit and positional tuple variants;
- positional tuple fields;
- identity references and recursive unary type applications;
- fixed visibility, keyword, delimiter, comma, and terminator positions.

Each record implements `StructureRecord` and is consumed by the one shared
`StructuralEvaluator`. `OrderedSequence` links the mixed word and boundary
positions of one complete Rust record. Declaration positions require
translator-issued assignments. Reference positions use lookup-only resolution.
Typed alternatives are sealed only when conservative disjointness proves them;
vector order never selects meaning.

The one Rust-specific `RustLogos` object selects the expected item type from
the pass-one cue, then reifies or reflects the complete shared-evaluator value.
It does not tokenize general Rust, concatenate source, select structural
alternatives by order, mint identities, or walk a separate syntax tree.

## Names

The structuretree and fixed words are addressed through immutable Rust-root
encoded IDs supplied by the caller. The component validates their root and
exact spelling while sealing, then retains only a read-only fixed-word view.

Universal names are not emitted by resolving their human spelling. The caller
must associate each complete encoded-ID chain with an opaque
`FixtureRustEmittedIdentifier`. `FixtureRustNameProjectionTable` validates the
token and rejects mappings in which one identity has two tokens or two
identities share one token. These caller-supplied projections are explicitly
fixture-only: no algorithm derives a production token from the chain.

## Emission

Every semantic token, delimiter, and punctuation position is reflected through
the shared evaluator. The fixture breadth emits complete structural records
such as:

```text
pub struct <opaque-name> ( Vec < u64 > , ) ;
pub enum <opaque-name> { <unit> , <payload> ( u64 , Vec < u64 > , ) , }
```

Item and wrapped-newtype-field visibility remain typed data. Enumeration tuple
fields are positional and private. Attributes, generics, named fields, and
other Rust items are refused.

Acceptance is behavioral: generated Rust is compiled and run by the public
test, including exhaustive unit/payload behavior and a Vector application
newtype. Byte identity with any legacy renderer is not a contract.
