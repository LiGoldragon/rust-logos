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
//!   trait implemented per CoreLogos node kind. Each node projects the Rust tokens
//!   it already is; a [`NameResolver`](name_table::NameResolver) is threaded to the
//!   identifier leaves. A single `prettyplease` pass owns all formatting
//!   ([`RustSource::project_item`](project::RustSource::project_item)). Projection
//!   may synthesize exactly five things and nothing else — the five-synthesis rule,
//!   documented on [`project`].
//! - **Reader** ([`read`]): [`ReadRust`] maps the in-subset `syn`
//!   AST to CoreLogos, interning names through a
//!   [`NameInterner`](name_table::NameInterner). Every out-of-subset construct is a
//!   typed loud [`Error`] naming the construct — the reader never guesses and never
//!   skips.
//!
//! ## The subset boundary
//!
//! The two-way subset is exactly what CoreLogos models: the four item kinds
//! (newtype, named-field struct, enum, type alias) over the witnessed
//! attribute/visibility/generic/type vocabulary. Trait definitions, impl blocks,
//! free methods, `use` re-exports, modules, macros, unions, const generics, and
//! arbitrary expression/statement bodies are **out of subset** and fail loudly.
//! This is the documented growth point: the trait/impl body vocabulary is where the
//! subset grows, if and when Logos models method bodies as data.
//!
//! ## The codec is the acceptance oracle
//!
//! The schema-rust goldens gate the codec both directions
//! ([`codec::Coverage`]): `decode(golden) → CoreLogos → encode` reproduces the
//! golden bytes for the in-subset items, and (from `core-logos`'s golden-pair
//! fixtures) `encode → decode` reproduces the CoreLogos value with a stable content
//! identity — the hash never moves through text.

pub mod codec;
pub mod error;
pub mod project;
pub mod read;

pub use codec::{Coverage, DecodeAtomically, InSubsetItem, ItemOutcome};
pub use error::Error;
pub use project::{GENERATED_HEADER, ProjectRust, RustSource};
pub use read::ReadRust;
