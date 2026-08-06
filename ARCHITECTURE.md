# Architecture

## Boundary

`rust-logos` is a TextualForm boundary. Logos stays typed and stringless; Rust
text exists only while this component discovers or emits the interim
programming interface.

There is no Rust Capsule here. The input and output content is
`core_logos::WholeLogos`. Declarations carry complete Universal encoded-ID
chains. References may instead carry complete immutable Rust-vocabulary
chains after typed Nomos transformation.

## Two passes

Pass one is `raw-discovery` cue-to-termination discovery. `struct` is an
inclusive cue terminated by `;`; `enum` is an inclusive cue terminated after
its configured balanced brace boundary. Delimiter children are recursive,
strings and comments are opaque, and every node carries exact source bounds.

Pass two is expectation-driven. Complete archived position records describe
the sealed Rust vocabulary:

- newtype and enumeration items;
- unit variants and tuple variants with one or more positional fields;
- positional tuple fields;
- identity references and recursive nonempty type applications;
- fixed visibility, keyword, delimiter, comma, and terminator positions.

Each record implements `StructureRecord` and is consumed by the one shared
`StructuralEvaluator`. `OrderedSequence` links the mixed word and boundary
positions of one complete Rust record. Declaration positions require
translator-issued assignments. Reference positions use lookup-only resolution.
Declaration reification remains Universal-root strict; reference reification
preserves either root in the closed production set.
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

Universal names are not emitted by resolving their human spelling.
Production emission derives the token algorithmically: `RustEncodedIdCodec`
computes it directly from the complete encoded-ID chain (see "Production
naming" below). A separate fixture-only path still exists for the earlier
slice-one witness: the caller associates each complete encoded-ID chain with
an opaque `FixtureRustEmittedIdentifier`, and `FixtureRustNameProjectionTable`
validates the token and rejects mappings in which one identity has two
tokens or two identities share one token. That caller-supplied projection
path is explicitly fixture-only and is not how production names are derived.

## Production naming

`RustLogos::emit` is the production entry point. It resolves every Universal
identity to a name computed by `RustEncodedIdCodec::encode`: the payload is
the format version byte, the explicit production root tag byte, then every
table-local `u16` in the chain in big-endian order, rendered as Base58BTC
with a leading `z` multibase discriminator. The encoding is injective, has no
semantic depth cap, and an operational rename (a spelling-only edit in the
owning nametable) never changes it, because it is computed from the
identity's encoded-ID chain, not from any resolved spelling. Rust-root
identities keep their immutable Rust spelling, resolved through the caller's
allocated view (`EncodedNameResolver`), rather than being encoded by this
codec.

Per DRR entry 7, emitted Rust was ruled to use "a textual version of [the
encoded identity] - some kind of textual binary encoding which is friendly to
rustc". Per DRR entry 17, an earlier fixed-width-decimal proposal was
rejected by the psyche, and the exact codec was explicitly left as
implementation matter rather than a psyche ruling: "encode the complete
identity in a compact, readable textual-binary form accepted by rustc" is the
ruled direction; Base58BTC is this crate's implementation choice satisfying
it, not itself a separate ruling.

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
