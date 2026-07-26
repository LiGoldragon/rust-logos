//! The decode subset boundary: out-of-subset constructs fail loudly with typed
//! errors naming the construct, and a failed decode leaves the NameTable
//! byte-identical — the interning-atomicity law extended to the Rust edge.

use name_table::{IdentifierNamespace, Name, NameTable};
use textual_rust::error::Error;
use textual_rust::{DecodeAtomically, RustSource};

/// Parse one top-level item from a snippet.
fn only_item(source: &str) -> syn::Item {
    let items = RustSource::new(source).parse_items().expect("parses");
    assert_eq!(items.len(), 1, "one top-level item");
    items.into_iter().next().unwrap()
}

/// A failed decode interns nothing: the committed table is byte-identical, even
/// when the item is partly in subset (a good field before an out-of-subset one).
#[test]
fn a_failed_decode_leaves_the_name_table_unchanged() {
    // The struct name and first field would intern if the decode were not rolled
    // back; the reference-typed second field is out of subset.
    let item = only_item("pub struct Bad { pub good: Thing, pub broken: &'static str }");

    let mut table = NameTable::new(IdentifierNamespace::Logos);
    table
        .intern(Name::new("Preexisting"))
        .expect("preexisting name interns");
    let length_before = table.len();
    let identity_before = table.identity().expect("identity before");

    let error = item
        .decode_atomically(&mut table)
        .expect_err("reference field is out of subset");
    assert!(
        matches!(error, Error::UnsupportedType { .. }),
        "got {error:?}"
    );

    assert_eq!(table.len(), length_before, "no names leaked");
    assert_eq!(
        table.identity().expect("identity after").bytes(),
        identity_before.bytes(),
        "the table is byte-identical after a failed decode",
    );
}

/// Each out-of-subset construct fails loudly with the typed variant that names it.
#[test]
fn out_of_subset_constructs_fail_loudly_and_typed() {
    let mut table = NameTable::new(IdentifierNamespace::Logos);

    type ErrorMatches = fn(&Error) -> bool;
    let cases: &[(&str, ErrorMatches)] = &[
        (
            "pub trait Foo {}",
            |e| matches!(e, Error::UnsupportedItem { construct } if construct.contains("trait")),
        ),
        // A bare `use path::Name;` is out of subset — only the brace-group import
        // (the module prelude's NOTA import) is modeled.
        ("pub use crate::path::Thing;", |e| {
            matches!(e, Error::UnsupportedUseTree { .. })
        }),
        ("pub use nota::*;", |e| {
            matches!(e, Error::UnsupportedUseTree { .. })
        }),
        // Impl blocks and functions are now in subset, and so are associated types
        // and consts, but a macro member is still out of subset, and an empty
        // function body is not a single tail expression.
        ("impl Foo { my_member_macro!(); }", |e| {
            matches!(e, Error::UnsupportedImplItem { .. })
        }),
        ("pub fn run() {}", |e| {
            matches!(e, Error::UnsupportedStatement { .. })
        }),
        ("pub struct Unit;", |e| matches!(e, Error::UnitStruct)),
        ("pub struct Pair(A, B);", |e| {
            matches!(e, Error::MultiFieldTupleStruct { field_count: 2 })
        }),
        ("pub struct WithConst<const N: usize> { pub a: A }", |e| {
            matches!(e, Error::UnsupportedGenericParameter { .. })
        }),
        ("#[doc = \"note\"] pub struct A { pub a: B }", |e| {
            matches!(e, Error::UnsupportedAttribute { .. })
        }),
        ("pub struct Ref { pub a: &'static B }", |e| {
            matches!(e, Error::UnsupportedType { .. })
        }),
        ("pub struct Whered<T> where T: Clone { pub a: T }", |e| {
            matches!(e, Error::WhereClause)
        }),
    ];

    for (source, is_expected) in cases {
        let item = only_item(source);
        let error = match item.decode_atomically(&mut table) {
            Ok(_) => panic!("`{source}` must be out of subset, but decoded"),
            Err(error) => error,
        };
        assert!(
            is_expected(&error),
            "`{source}` produced the wrong error: {error:?}",
        );
    }

    // Every rejection was atomic, so nothing leaked across the whole battery.
    assert!(table.is_empty(), "no out-of-subset case interned a name");
}
