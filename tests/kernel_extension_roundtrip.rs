//! The layout-3 kernel-extension acceptance proof: the golden's class-B constructor
//! and trait impls, its class-D trace/object-name enums and nested-match `name`
//! methods, and its class-C const/module/associated-const stub items decode into the
//! grown CoreLogos vocabulary and re-encode byte-exact against the frozen
//! `spirit_generated.rs` golden bytes. Negative tests hold the vocabulary boundary:
//! a construct just outside the closed set fails loudly and typed.

use core_logos::{Const, CoreItem, Expression, ImplItem, IntegerRepresentation, TypeReference};
use name_table::{NameResolver, NameTable};
use textual_rust::error::Error;
use textual_rust::{DecodeAtomically, RustSource};

const SPIRIT_GOLDEN: &str = include_str!("fixtures/spirit_generated.rs");

/// Parse one top-level item from a snippet.
fn only_item(source: &str) -> syn::Item {
    let items = RustSource::new(source).parse_items().expect("parses");
    assert_eq!(items.len(), 1, "one top-level item");
    items.into_iter().next().unwrap()
}

/// Decode every golden item in source order so the table accumulates names exactly
/// as the corpus survey does, and return the decoded CoreLogos value of the first
/// item whose canonical golden text contains `marker`, asserting it round-trips
/// byte-exact against the real golden bytes.
fn round_trip_golden_item(marker: &str, table: &mut NameTable) -> CoreItem {
    let items = RustSource::new(SPIRIT_GOLDEN)
        .parse_items()
        .expect("golden parses");
    let mut found = None;
    for item in &items {
        let canonical = RustSource::canonical_item(item);
        let Ok(core) = item.decode_atomically(table) else {
            continue;
        };
        if !canonical.as_str().contains(marker) {
            continue;
        }
        let reencoded =
            RustSource::project_item(&core, table as &dyn NameResolver).expect("project");
        assert_eq!(
            reencoded.as_str(),
            canonical.as_str(),
            "round-trip diverged for the golden item matching `{marker}`",
        );
        assert!(
            SPIRIT_GOLDEN.contains(canonical.as_str().trim_end()),
            "the canonical text matching `{marker}` is not present verbatim in the golden",
        );
        if found.is_none() {
            found = Some(core);
        }
    }
    found.unwrap_or_else(|| panic!("no golden item matched `{marker}`"))
}

/// Class B: the `FromStr` impl carries a `type Err = …;` associated type and a body
/// with a `parse::<Self>()` turbofish, and round-trips byte-exact.
#[test]
fn class_b_from_str_impl_round_trips_with_associated_type_and_turbofish() {
    let mut table = NameTable::new();
    let core = round_trip_golden_item("impl std::str::FromStr for Input", &mut table);
    let CoreItem::ImplBlock(impl_block) = core else {
        panic!("a trait impl block");
    };
    assert!(
        impl_block
            .items
            .iter()
            .any(|item| matches!(item, ImplItem::AssociatedType(_))),
        "the FromStr impl carries an associated type",
    );
}

/// Class B: the `Display` impl's `fmt` takes `&mut std::fmt::Formatter<'_>` (mutable
/// reference plus a `'_` lifetime argument) and round-trips byte-exact.
#[test]
fn class_b_display_impl_round_trips_with_mut_formatter() {
    let mut table = NameTable::new();
    let _ = round_trip_golden_item("impl std::fmt::Display for Input", &mut table);
}

/// Class D: the `SignalObjectName::name` method is a nested match (an outer match arm
/// whose body is another match) returning `&'static str`, and round-trips byte-exact.
#[test]
fn class_d_nested_match_name_method_round_trips() {
    let mut table = NameTable::new();
    let core = round_trip_golden_item("impl SignalObjectName", &mut table);
    let CoreItem::ImplBlock(impl_block) = core else {
        panic!("an inherent impl block");
    };
    let ImplItem::Method(name_method) = &impl_block.items[0] else {
        panic!("the name method");
    };
    let Expression::Match(outer) = &name_method.body.tail_expression else {
        panic!("the body is a match");
    };
    assert!(
        matches!(outer.arms[0].body, Expression::Match(_)),
        "the outer match arm body is itself a match — a nested match",
    );
}

/// Class D: the `TraceEvent` newtype-plus-impl round-trips byte-exact.
#[test]
fn class_d_trace_event_impl_round_trips() {
    let mut table = NameTable::new();
    let _ = round_trip_golden_item("impl TraceEvent", &mut table);
}

/// Class C: the `short_header` const module round-trips byte-exact, and its consts
/// carry hexadecimal integer literals as stringless value-plus-representation data.
#[test]
fn class_c_short_header_module_round_trips_with_hex_consts() {
    let mut table = NameTable::new();
    let core = round_trip_golden_item("pub mod short_header", &mut table);
    let CoreItem::Module(module) = core else {
        panic!("a module");
    };
    let first_const = module
        .items
        .iter()
        .find_map(|item| match item {
            CoreItem::Const(const_item) => Some(const_item),
            _ => None,
        })
        .expect("the module carries consts");
    let Expression::IntegerLiteral(literal) = &first_const.value else {
        panic!("a const with an integer-literal value");
    };
    assert!(
        matches!(
            literal.representation,
            IntegerRepresentation::Hexadecimal { minimum_digits: 16 }
        ),
        "the short-header consts are 16-digit hexadecimal literals",
    );
}

/// Class C: the top-level `SIGNAL_SHORT_HEADER_BYTE_COUNT` const round-trips
/// byte-exact with a decimal literal.
#[test]
fn class_c_top_level_const_round_trips_decimal() {
    let mut table = NameTable::new();
    let core = round_trip_golden_item("const SIGNAL_SHORT_HEADER_BYTE_COUNT", &mut table);
    let CoreItem::Const(Const { value, .. }) = core else {
        panic!("a top-level const");
    };
    assert!(
        matches!(
            value,
            Expression::IntegerLiteral(literal) if matches!(literal.representation, IntegerRepresentation::Decimal)
        ),
        "the byte-count const is a decimal literal",
    );
}

/// Class C: the `SignalOperationHeads` impl's `HEADS` associated const has a
/// `&'static [&'static str]` slice type and a `&[…]` array value, and round-trips
/// byte-exact.
#[test]
fn class_c_associated_const_array_round_trips() {
    let mut table = NameTable::new();
    let core = round_trip_golden_item(
        "impl signal_frame::SignalOperationHeads for Input",
        &mut table,
    );
    let CoreItem::ImplBlock(impl_block) = core else {
        panic!("a trait impl");
    };
    let ImplItem::AssociatedConst(heads) = &impl_block.items[0] else {
        panic!("the HEADS associated const");
    };
    // The type is a reference to a slice of references to str.
    assert!(
        matches!(&heads.type_reference, TypeReference::Reference(reference)
            if matches!(&*reference.referent, TypeReference::Slice(_))),
        "HEADS is typed `&'static [&'static str]`",
    );
    // The value is a reference to an array literal.
    assert!(
        matches!(&heads.value, Expression::Reference(reference)
            if matches!(&*reference.referent, Expression::Array(_))),
        "HEADS's value is `&[…]`",
    );
}

/// The vocabulary boundary holds: a construct just outside the closed set fails
/// loudly and typed, and nothing interns across the battery.
#[test]
fn out_of_vocabulary_kernel_constructs_fail_loudly_and_typed() {
    let mut table = NameTable::new();

    type ErrorMatches = fn(&Error) -> bool;
    let cases: &[(&str, ErrorMatches)] = &[
        // A float literal body — integer and string literals are modeled, floats are not.
        (
            "impl A { fn f(&self) -> f64 { 1.5 } }",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("literal")),
        ),
        // A suffixed integer literal.
        (
            "const N: u8 = 0u8;",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("suffixed")),
        ),
        // A binary integer literal — only decimal and lowercase hexadecimal are modeled.
        (
            "const N: u8 = 0b10;",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("binary")),
        ),
        // An uppercase-hexadecimal literal — the modeled hex form is lowercase.
        (
            "const N: u64 = 0xABCD;",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("uppercase")),
        ),
        // A module carrying a non-const item — only const-carrying modules are modeled.
        (
            "pub mod m { pub enum E { A } }",
            |e| matches!(e, Error::UnsupportedItem { construct } if construct.contains("non-const")),
        ),
        // A reference type in wire-data field position stays out of subset (the
        // reference relaxation is function-signature and const-type position only).
        ("pub struct Held { pub value: &'static u8 }", |e| {
            matches!(e, Error::UnsupportedType { .. })
        }),
        // A file module (`mod name;`) has no inline items to carry.
        (
            "pub mod bare;",
            |e| matches!(e, Error::UnsupportedItem { construct } if construct.contains("inline body")),
        ),
    ];

    for (source, is_expected) in cases {
        let item = only_item(source);
        let error = match item.decode_atomically(&mut table) {
            Ok(_) => panic!("`{source}` must be out of the kernel vocabulary, but decoded"),
            Err(error) => error,
        };
        assert!(
            is_expected(&error),
            "`{source}` produced the wrong error: {error:?}",
        );
    }

    assert!(
        table.is_empty(),
        "no out-of-vocabulary case interned a name"
    );
}
