//! The impl-block acceptance proof: the six newtype inherent impls and their `From`
//! impls from the frozen `spirit_generated.rs` golden decode into CoreLogos and
//! re-encode byte-exact against the original golden bytes. Plus a synthetic proof of
//! the qualified-trait-call and free-function vocabulary the copied corpus only
//! exercises behind a `#[cfg]` (and so leaves out of subset).

use core_logos::{CoreItem, TypeReference};
use name_table::{Name, NameResolver, NameTable};
use textual_rust::{DecodeAtomically, RustSource};

const SPIRIT_GOLDEN: &str = include_str!("fixtures/spirit_generated.rs");

/// The six newtype wrappers whose inherent impl and `From` impl this slice proves
/// (`spirit_generated.rs` lines ~158–270).
const NEWTYPES: &[&str] = &[
    "Topic",
    "Topics",
    "Description",
    "Summary",
    "RecordIdentifier",
    "RecordSet",
];

/// The final segment of an impl block's self type, resolved to text — `Topic` for
/// both `impl Topic` and `impl From<String> for Topic`.
fn self_type_head_name(item: &CoreItem, names: &NameTable) -> Option<String> {
    let CoreItem::ImplBlock(impl_block) = item else {
        return None;
    };
    let head = match &impl_block.self_type {
        TypeReference::Path(path) => path,
        TypeReference::Application(application) => &application.head,
        _ => return None,
    };
    let segment = *head.segments.last()?;
    Some(names.resolve(segment).ok()?.as_str().to_string())
}

/// One parsed top-level item from a snippet.
fn only_item(source: &str) -> syn::Item {
    let items = RustSource::new(source).parse_items().expect("parses");
    assert_eq!(items.len(), 1, "one top-level item");
    items.into_iter().next().unwrap()
}

/// Decode one item, re-encode, and assert the re-encoding reproduces the item's
/// prettyplease-canonical text byte-for-byte.
fn assert_round_trips_byte_exact(item: &syn::Item, table: &mut NameTable) -> CoreItem {
    let canonical = RustSource::canonical_item(item);
    let core = item
        .decode_atomically(table)
        .expect("the item is in the Tier-1 subset");
    let reencoded = RustSource::project_item(&core, table as &dyn NameResolver).expect("project");
    assert_eq!(
        reencoded.as_str(),
        canonical.as_str(),
        "round-trip diverged:\n--- canonical ---\n{}\n--- reencoded ---\n{}",
        canonical.as_str(),
        reencoded.as_str(),
    );
    core
}

#[test]
fn the_six_newtype_impl_blocks_and_their_from_impls_round_trip_byte_exact() {
    let source = RustSource::new(SPIRIT_GOLDEN);
    let items = source.parse_items().expect("golden parses");
    let mut table = NameTable::new();

    let mut proven = 0usize;
    for item in &items {
        // Decode every item so the shared table accumulates names in source order,
        // exactly as the corpus survey does; only assert on the newtype impls.
        let Ok(core) = item.decode_atomically(&mut table) else {
            continue;
        };
        if !matches!(core, CoreItem::ImplBlock(_)) {
            continue;
        }
        let Some(name) = self_type_head_name(&core, &table) else {
            continue;
        };
        if !NEWTYPES.contains(&name.as_str()) {
            continue;
        }

        let reencoded =
            RustSource::project_item(&core, &table as &dyn NameResolver).expect("project");
        let canonical = RustSource::canonical_item(item);
        assert_eq!(
            reencoded.as_str(),
            canonical.as_str(),
            "round-trip diverged for the impl on {name}",
        );
        assert!(
            SPIRIT_GOLDEN.contains(canonical.as_str().trim_end()),
            "the canonical impl text for {name} is not present verbatim in the golden",
        );
        proven += 1;
    }

    assert_eq!(
        proven, 12,
        "the six newtype inherent impls plus their six From impls",
    );
}

/// The `From` impls carry the implemented trait as data; the inherent impls do not.
#[test]
fn the_newtype_impls_split_into_inherent_and_from_by_the_trait_slot() {
    let items = RustSource::new(SPIRIT_GOLDEN)
        .parse_items()
        .expect("golden parses");
    let mut table = NameTable::new();

    let mut inherent = 0usize;
    let mut trait_impls = 0usize;
    for item in &items {
        let Ok(CoreItem::ImplBlock(impl_block)) = item.decode_atomically(&mut table) else {
            continue;
        };
        let head = match &impl_block.self_type {
            TypeReference::Path(path) => path,
            TypeReference::Application(application) => &application.head,
            _ => continue,
        };
        let name = table
            .resolve(*head.segments.last().unwrap())
            .unwrap()
            .as_str()
            .to_string();
        if !NEWTYPES.contains(&name.as_str()) {
            continue;
        }
        if impl_block.implemented_trait.is_some() {
            trait_impls += 1;
        } else {
            inherent += 1;
        }
    }

    assert_eq!(inherent, 6, "six inherent impls");
    assert_eq!(trait_impls, 6, "six From impls");
}

/// The qualified-trait-call body `<Self as Trait>::method(self)` — witnessed in the
/// goldens only behind a `#[cfg]` (and so out of subset there) — round-trips
/// byte-exact on its own.
#[test]
fn a_qualified_trait_call_body_round_trips_byte_exact() {
    let item = only_item(
        "impl Widget {\n    fn rendered(&self) -> String {\n        <Self as Encoder>::encode(self)\n    }\n}",
    );
    let mut table = NameTable::new();
    let core = assert_round_trips_byte_exact(&item, &mut table);
    assert!(matches!(core, CoreItem::ImplBlock(_)));
}

/// A free function is a top-level `CoreItem::Function`, not only an impl member.
#[test]
fn a_free_function_round_trips_byte_exact() {
    let item = only_item("pub fn wrap(payload: Integer) -> Topic {\n    Topic::new(payload)\n}");
    let mut table = NameTable::new();
    let core = assert_round_trips_byte_exact(&item, &mut table);
    assert!(matches!(core, CoreItem::Function(_)));
}

/// A `match self` over variant patterns mapping to string literals — the
/// `SignalObjectName::name` shape — round-trips byte-exact on its own, proving the
/// match / pattern / string-literal vocabulary end to end.
#[test]
fn a_match_over_self_with_string_literal_arms_round_trips_byte_exact() {
    let item = only_item(
        "impl Route {\n    fn name(self) -> &'static str {\n        match self {\n            Self::Record(_) => \"Record\",\n            Self::Observe(_) => \"Observe\",\n        }\n    }\n}",
    );
    let mut table = NameTable::new();
    let _ = table.intern(Name::new("preexisting"));
    let core = assert_round_trips_byte_exact(&item, &mut table);
    assert!(matches!(core, CoreItem::ImplBlock(_)));
}
