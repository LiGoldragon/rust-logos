//! # rust-logos
//!
//! Structural Rust decoding and emission for Logos.
//!
//! The bounded fixture slice supports ordered, attribute-free tuple newtypes
//! and non-generic enumerations with unit or positional tuple variants. Pass one uses
//! `raw-discovery`'s cue-to-termination model, with strings and comments opaque
//! and every block source-bounded. Pass two evaluates fully typed Rust position
//! records through `structural-codec`'s shared evaluator. Emission reflects the
//! same complete item records.
//!
//! Production emission names every Universal identity by the canonical
//! Base58BTC textual encoding of its complete root-fronted encoded-ID chain.
//! Rust-owned vocabulary keeps its immutable Rust spelling. The older
//! caller-projected surface remains available only for fixture witnesses.

mod codec;
mod error;
mod fixture_vocabulary;
mod identifier;

pub use codec::RustLogos;
pub use error::{EncodedIdCodecRefusal, Error, RustIdentifierRefusal};
pub use fixture_vocabulary::{
    FixtureRustRule, FixtureRustVocabulary, FixtureRustVocabularyIds, ReferencedTypePosition,
};
pub use identifier::{
    BASE58BTC_MULTIBASE_PREFIX, ENCODED_ID_FORMAT_VERSION, FixtureRustEmittedIdentifier,
    FixtureRustNameProjectionTable, RustEncodedIdCodec,
};
