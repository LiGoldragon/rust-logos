//! # textual-rust
//!
//! The **TextualRust form codec**: the two-way bridge between Rust TEXT and the
//! stringless [`core_logos`] algebra. It reads a principled subset of Rust *into*
//! Logos through [`syn`], and projects Logos out as byte-exact [`prettyplease`]
//! Rust. This crate is the **foreign raw layer** where `syn`, `prettyplease`, and
//! `quote` are allowed; the Core sibling `core-logos` stays text-free.
//!
//! ## The two directions
//!
//! - **Writer** ([`project`]): [`ProjectRust`] is one domain
//!   trait implemented per EncodedLogos node kind. Each node projects the Rust tokens
//!   it already is; a [`NameResolver`](name_table::NameResolver) is threaded to the
//!   identifier leaves. A single `prettyplease` pass owns all formatting
//!   ([`RustSource::project_item`](project::RustSource::project_item)). Projection
//!   may synthesize exactly five things and nothing else — the five-synthesis rule,
//!   documented on [`project`].
//! - **Reader** ([`read`]): [`ReadRust`] maps the in-subset `syn`
//!   AST to EncodedLogos, interning names through a
//!   [`NameInterner`](name_table::NameInterner). Every out-of-subset construct is a
//!   typed loud [`Error`] naming the construct — the reader never guesses and never
//!   skips.
//!
//! ## The subset boundary
//!
//! The two-way subset is exactly what EncodedLogos models: the data item kinds
//! (newtype, named-field struct, enum) plus impl blocks and functions
//! whose bodies are the closed Tier-1 expression vocabulary, the brace-group `use`
//! import (`use <base>::{<names>};` — the module prelude's NOTA import), and — from
//! the layout-3 kernel extension — associated types and consts in impls, `const`
//! items, and const-carrying inline modules, over the witnessed
//! attribute/visibility/generic/type vocabulary. Trait definitions, non-brace-group
//! `use` shapes (bare, glob, rename), non-const modules, macros, unions, const
//! generics, and Tier-2 statement bodies (`let` bindings, early `return`, struct
//! literals) remain **out of subset** and fail loudly. This is the documented growth
//! point: the subset grows further when Logos models the Tier-2 bodies or trait
//! definitions as data.
//!
//! ## The codec is the acceptance oracle
//!
//! The schema-rust goldens gate the codec both directions
//! ([`codec::Coverage`]): `decode(golden) → EncodedLogos → encode` reproduces the
//! golden bytes for the in-subset items, and (from `core-logos`'s golden-pair
//! fixtures) `encode → decode` reproduces the EncodedLogos value with a stable content
//! identity — the hash never moves through text.

pub mod codec;
pub mod error;
pub mod project;
pub mod read;

pub use codec::{Coverage, DecodeAtomically, InSubsetItem, ItemOutcome};
pub use error::Error;
pub use project::{GENERATED_HEADER, ProjectRust, RustSource};
pub use read::ReadRust;
