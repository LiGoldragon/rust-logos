//! # rust-logos
//!
//! Structural Rust decoding and emission for Logos.
//!
//! The bounded fixture slice supports ordered tuple newtypes, structs,
//! non-generic enumerations, behavior traits, and associated-type trait
//! implementations. Whole Logos type declarations choose either plain emission
//! or the canonical Interface wire preamble. Pass one uses
//! `raw-discovery`'s cue-to-termination model, with strings and comments opaque
//! and every block source-bounded. Pass two evaluates fully typed Rust position
//! records through `structural-codec`'s shared evaluator. Emission reflects the
//! same complete item records. Trait receivers and positional parameter names
//! are Rust-assembly facts; methods project lowerCamel source names to snake_case
//! only at this boundary.
//!
//! Production emission names every Universal identity by the canonical
//! Base58BTC textual encoding of its complete root-fronted encoded-ID chain.
//! Rust-owned vocabulary is released only by the naming authority.

mod codec;
mod error;
mod fixture_vocabulary;
mod identifier;

pub use codec::{
    InterfaceRustEmission, InterfaceRustRoleIds, RustLogos, RustTypePath, RustTypePathResolver,
};
pub use error::{EncodedIdCodecRefusal, Error, RustIdentifierRefusal};
pub use fixture_vocabulary::ReferencedTypePosition;
pub use identifier::{
    BASE58BTC_MULTIBASE_PREFIX, ENCODED_ID_FORMAT_VERSION, FixtureRustEmittedIdentifier,
    FixtureRustNameProjectionTable, RustEncodedIdCodec,
};
