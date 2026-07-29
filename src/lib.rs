//! # rust-logos
//!
//! Structural Rust decoding and emission for Logos.
//!
//! Slice One supports ordered, attribute-free tuple newtypes. Pass one uses
//! `raw-discovery`'s cue-to-termination model, with strings and comments opaque
//! and every block source-bounded. Pass two evaluates fully typed Rust position
//! records through `structural-codec`'s shared evaluator. Emission reflects the
//! same records and writes fixed Rust punctuation directly.
//!
//! Universal encoded-ID chains never acquire a textual encoding here. The
//! caller supplies a checked rustc-safe opaque token for each complete chain,
//! and [`RustNameProjectionTable`] preserves that association.

mod codec;
mod error;
mod identifier;
mod vocabulary;

pub use codec::RustLogos;
pub use error::{Error, RustIdentifierRefusal};
pub use identifier::{RustEmittedIdentifier, RustNameProjectionTable};
pub use vocabulary::{
    DeclarationNamePosition, DeclarationNameRule, PublicKeywordPosition, PublicKeywordRule,
    ReferencedTypePosition, ReferencedTypeRule, RustNewtypeRule, RustNewtypeVocabulary,
    RustNewtypeVocabularyIds, StructKeywordPosition, StructKeywordRule,
};
