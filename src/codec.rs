//! The codec entry points: atomic per-item decode, per-item canonical rendering,
//! and the golden-corpus coverage survey.
//!
//! Decode interns through a [`NameTransaction`](name_table::NameTransaction) so a failed decode leaves the
//! committed [`NameTable`] byte-identical — the interning-atomicity law extended to
//! the Rust edge. The unit of byte-exactness is the *item*: an item's prettyplease
//! canonical form is the acceptance oracle, and a round-trip must reproduce it.

use core_logos::EncodedItem;
use name_table::{NameResolver, NameTable};

use crate::error::Error;
use crate::project::RustSource;
use crate::read::ReadRust;

/// Decode a single parsed item atomically — a verb on the `syn::Item` noun. On
/// success the interned names commit into the table; on failure the table is left
/// byte-identical, because interning ran through a rolled-back transaction.
pub trait DecodeAtomically {
    /// Read this item into EncodedLogos, committing its names only if the whole item
    /// is in subset. A single out-of-subset construct fails the whole item and
    /// leaks no name.
    fn decode_atomically(&self, table: &mut NameTable) -> Result<EncodedItem, Error>;
}

impl DecodeAtomically for syn::Item {
    fn decode_atomically(&self, table: &mut NameTable) -> Result<EncodedItem, Error> {
        table.try_intern(|transaction| self.read(transaction))
    }
}

impl RustSource {
    /// Parse this Rust text into its top-level items through `syn`. Nested items
    /// (inside a module) are not surfaced — the enclosing module is itself
    /// out-of-subset, so only the top level is a decode candidate.
    pub fn parse_items(&self) -> Result<Vec<syn::Item>, Error> {
        let file =
            syn::parse_file(self.as_str()).map_err(|error| Error::Parse(error.to_string()))?;
        Ok(file.items)
    }

    /// The prettyplease canonical text of one parsed `syn` item — the single-pass
    /// render of the item in isolation. Because the goldens are themselves
    /// prettyplease output, this reproduces the item's exact golden bytes, so it is
    /// the byte-exact acceptance form a round-trip is checked against.
    pub fn canonical_item(item: &syn::Item) -> Self {
        let file = syn::File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![item.clone()],
        };
        Self::new(prettyplease::unparse(&file))
    }
}

/// The outcome of surveying one item in a golden corpus: either an in-subset item
/// that decoded and re-encoded, or an out-of-subset item with the loud reason it
/// could not decode.
#[derive(Debug)]
pub enum ItemOutcome {
    /// An in-subset item: its EncodedLogos value, its canonical golden text, and the
    /// text produced by re-encoding the EncodedLogos value. Boxed because a decoded
    /// `EncodedItem` (an impl block carries whole method-body trees) dwarfs the
    /// out-of-subset error.
    InSubset(Box<InSubsetItem>),
    /// An out-of-subset item: the typed error naming the construct that stopped it.
    OutOfSubset(Error),
}

/// An in-subset item that decoded to EncodedLogos and re-encoded to text.
#[derive(Debug)]
pub struct InSubsetItem {
    /// The decoded stringless EncodedLogos value.
    pub core: EncodedItem,
    /// The item's prettyplease canonical text (its exact golden bytes).
    pub canonical: RustSource,
    /// The text produced by projecting `core` back to Rust.
    pub reencoded: RustSource,
}

impl InSubsetItem {
    /// Whether the round-trip reproduced the canonical golden text byte-for-byte.
    pub fn is_byte_exact(&self) -> bool {
        self.canonical == self.reencoded
    }
}

/// A survey of one golden module: every top-level item classified as in-subset
/// (decoded and re-encoded) or out-of-subset (a loud typed reason), with the
/// byte-exact round-trip evidence for the in-subset items.
#[derive(Debug)]
pub struct Coverage {
    /// One outcome per top-level item, in source order.
    pub outcomes: Vec<ItemOutcome>,
}

impl Coverage {
    /// Survey a golden module: parse its top-level items, decode each atomically,
    /// and for the in-subset items re-encode to the byte-exact canonical form. The
    /// table is threaded so in-subset names commit into the continuous space; an
    /// out-of-subset item leaves the table untouched.
    pub fn survey(source: &RustSource, table: &mut NameTable) -> Result<Self, Error> {
        let items = source.parse_items()?;
        let mut outcomes = Vec::with_capacity(items.len());
        for item in &items {
            match item.decode_atomically(table) {
                Ok(core) => {
                    let reencoded = RustSource::project_item(&core, table as &dyn NameResolver)?;
                    let canonical = RustSource::canonical_item(item);
                    outcomes.push(ItemOutcome::InSubset(Box::new(InSubsetItem {
                        core,
                        canonical,
                        reencoded,
                    })));
                }
                Err(reason) => outcomes.push(ItemOutcome::OutOfSubset(reason)),
            }
        }
        Ok(Self { outcomes })
    }

    /// The in-subset items, in source order.
    pub fn in_subset(&self) -> impl Iterator<Item = &InSubsetItem> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            ItemOutcome::InSubset(item) => Some(item.as_ref()),
            ItemOutcome::OutOfSubset(_) => None,
        })
    }

    /// The out-of-subset reasons, in source order.
    pub fn out_of_subset(&self) -> impl Iterator<Item = &Error> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            ItemOutcome::OutOfSubset(reason) => Some(reason),
            ItemOutcome::InSubset(_) => None,
        })
    }

    /// How many top-level items decoded into subset.
    pub fn in_subset_count(&self) -> usize {
        self.in_subset().count()
    }

    /// How many top-level items were the out-of-subset frontier.
    pub fn out_of_subset_count(&self) -> usize {
        self.out_of_subset().count()
    }

    /// Whether every in-subset item round-tripped byte-exact — the corpus gate.
    pub fn every_in_subset_item_is_byte_exact(&self) -> bool {
        self.in_subset().all(InSubsetItem::is_byte_exact)
    }
}
