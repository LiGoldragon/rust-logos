//! The crate-boundary error type.
//!
//! Every failure that crosses the codec boundary is a typed variant that names the
//! offending construct. The decode direction never guesses and never skips: an
//! out-of-subset construct produces a loud, named error rather than a silent
//! omission, so the principled subset is enforced by construction rather than by
//! convention. Name-projection and syn-parse failures come from the layers below.

use name_table::NameTableError;
use thiserror::Error;

/// A failure at the `textual-rust` codec boundary.
#[derive(Debug, Clone, Error)]
pub enum Error {
    /// A top-level item kind the CoreLogos algebra does not model — a trait
    /// definition, an impl block, a free function, a `use` re-export, a module, a
    /// macro invocation, a union, a const, or a static. The four modeled kinds are
    /// newtype, named-field struct, enum, and type alias.
    #[error(
        "out-of-subset item: {construct} is not one of the four modeled item kinds (newtype, struct, enum, alias)"
    )]
    UnsupportedItem { construct: &'static str },

    /// A tuple struct with a field count other than one. Only the single-field
    /// tuple newtype (`struct Name(Wrapped);`) is modeled.
    #[error(
        "out-of-subset item: a tuple struct with {field_count} fields (only the single-field tuple newtype is modeled)"
    )]
    MultiFieldTupleStruct { field_count: usize },

    /// A unit struct (`struct Name;`). A named-field struct with an empty body
    /// (`struct Name {}`) is modeled; the unit form is a distinct construct that
    /// does not round-trip to it.
    #[error(
        "out-of-subset item: a unit struct (`struct Name;`); only named-field structs are modeled"
    )]
    UnitStruct,

    /// An attribute outside the modeled vocabulary. The modeled vocabulary is a
    /// bare dotted tool path, a `derive(...)` group, a `cfg_attr(...)` wrapping an
    /// inner attribute, and a namespaced helper `derive` attribute.
    #[error("out-of-subset attribute: {rendered}")]
    UnsupportedAttribute { rendered: String },

    /// A `cfg_attr` whose predicate is not a bare `feature = \"...\"`. Only the
    /// feature predicate is modeled.
    #[error("out-of-subset cfg predicate: {rendered} (only `feature = \"...\"` is modeled)")]
    UnsupportedConfigurationPredicate { rendered: String },

    /// A visibility outside the modeled set (`pub`, `pub(crate)`, `pub(in path)`,
    /// or none). A `pub(self)`/`pub(super)` shorthand or any other restricted form
    /// is not modeled.
    #[error("out-of-subset visibility: {rendered}")]
    UnsupportedVisibility { rendered: String },

    /// A type expression outside the modeled set. The modeled set is a bare path
    /// and a generic application over paths; references, tuples, trait objects,
    /// slices, and function pointers are not modeled.
    #[error("out-of-subset type: {construct}")]
    UnsupportedType { construct: &'static str },

    /// A generic parameter outside the modeled set (a plain type parameter with
    /// path bounds, or a lifetime parameter). A const generic is a by-design
    /// growth point the surveyed goldens exercise but the Core does not yet carry.
    #[error("out-of-subset generic parameter: {construct}")]
    UnsupportedGenericParameter { construct: &'static str },

    /// A `where` clause on a declaration. The witnessed wire declarations carry
    /// none; a modeled declaration states its bounds inline on the parameter.
    #[error("out-of-subset construct: a `where` clause on a declaration")]
    WhereClause,

    /// A generic bound that is not a plain trait path (a lifetime bound, a
    /// `?Sized` relaxation, or a higher-ranked bound).
    #[error("out-of-subset generic bound: {construct}")]
    UnsupportedGenericBound { construct: &'static str },

    /// A name-projection failure: an identifier the resolver does not know (a torn
    /// Core/NameTable pair).
    #[error("name projection failed: {0}")]
    Name(#[from] NameTableError),

    /// A failure parsing Rust text into the syn AST the decode direction reads.
    #[error("rust parse failed: {0}")]
    Parse(String),

    /// A failure parsing the token stream the encode direction assembled back into
    /// a syn item for the single prettyplease pass. An internal invariant break:
    /// a well-formed projection always re-parses.
    #[error("projected token stream did not re-parse as a rust item: {0}")]
    Project(String),
}
