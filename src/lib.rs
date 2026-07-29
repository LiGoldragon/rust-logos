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
//! Universal encoded-ID chains never acquire a textual encoding here. The
//! caller supplies a checked rustc-safe opaque token for each complete chain,
//! and [`FixtureRustNameProjectionTable`] preserves that fixture association.

mod codec;
mod error;
mod fixture_vocabulary;
mod identifier;

pub use codec::RustLogos;
pub use error::{Error, RustIdentifierRefusal};
pub use fixture_vocabulary::{
    FixtureRustRule, FixtureRustVocabulary, FixtureRustVocabularyIds, ReferencedTypePosition,
};
pub use identifier::{FixtureRustEmittedIdentifier, FixtureRustNameProjectionTable};
