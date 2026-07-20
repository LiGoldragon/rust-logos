//! The Tier-1 body boundary: a method or function body outside the closed
//! expression/pattern vocabulary fails loudly with the typed variant that names the
//! construct, and — because decode is atomic — the whole impl leaks no name.

use name_table::{Name, NameTable};
use textual_rust::error::Error;
use textual_rust::{DecodeAtomically, RustSource};

/// One parsed top-level item from a snippet.
fn only_item(source: &str) -> syn::Item {
    let items = RustSource::new(source).parse_items().expect("parses");
    assert_eq!(items.len(), 1, "one top-level item");
    items.into_iter().next().unwrap()
}

/// Each out-of-vocabulary body construct fails loudly with the typed variant that
/// names it, and nothing interns across the whole battery.
#[test]
fn out_of_vocabulary_bodies_fail_loudly_and_typed() {
    let mut table = NameTable::new(name_table::IdentifierNamespace::Logos);

    type ErrorMatches = fn(&Error) -> bool;
    let cases: &[(&str, ErrorMatches)] = &[
        // A `let` binding — a body is a single tail expression, not a statement list.
        (
            "impl A { fn f(&self) -> u8 { let value = self.0; value } }",
            |e| matches!(e, Error::UnsupportedStatement { .. }),
        ),
        // An early `return` as the tail expression.
        (
            "impl A { fn f(&self) -> u8 { return self.0 } }",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("return")),
        ),
        // A struct-literal construction `Self { … }`.
        (
            "impl A { fn f(self) -> A { A { value: self.0 } } }",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("struct literal")),
        ),
        // A named field access `self.value`.
        (
            "impl A { fn f(&self) -> u8 { self.value } }",
            |e| matches!(e, Error::UnsupportedExpression { construct } if construct.contains("named field")),
        ),
        // A binary-operator expression.
        ("impl A { fn f(&self) -> u8 { self.0 + self.0 } }", |e| {
            matches!(e, Error::UnsupportedExpression { .. })
        }),
        // A wildcard match arm — the vocabulary forbids catch-alls.
        (
            "impl A { fn f(&self) -> u8 { match self { _ => self.0 } } }",
            |e| matches!(e, Error::UnsupportedPattern { construct } if construct.contains("wildcard")),
        ),
        // A `&mut self` receiver.
        ("impl A { fn f(&mut self) -> u8 { self.0 } }", |e| {
            matches!(e, Error::UnsupportedFunctionSignature { .. })
        }),
        // A macro member in an impl block (associated types and consts are now in
        // subset, but a macro member is not).
        ("impl A { my_member_macro!(); }", |e| {
            matches!(e, Error::UnsupportedImplItem { .. })
        }),
        // An `impl Trait` type in field position stays out of subset (the signature
        // relaxation is function-position only).
        ("pub struct Held { pub value: impl Into<u8> }", |e| {
            matches!(e, Error::UnsupportedType { .. })
        }),
    ];

    for (source, is_expected) in cases {
        let item = only_item(source);
        let error = match item.decode_atomically(&mut table) {
            Ok(_) => panic!("`{source}` must be out of the Tier-1 vocabulary, but decoded"),
            Err(error) => error,
        };
        assert!(
            is_expected(&error),
            "`{source}` produced the wrong error: {error:?}",
        );
    }

    assert!(
        table.is_empty(),
        "no out-of-vocabulary case interned a name — decode is atomic",
    );
}

/// A partly-in-vocabulary impl (a good first method, then a class-B second method)
/// fails the whole item atomically: the committed table is byte-identical.
#[test]
fn a_partly_in_vocabulary_impl_leaks_no_name() {
    let item = only_item(
        "impl Widget { \
         fn good(&self) -> u8 { self.0 } \
         fn bad(&self) -> u8 { let value = self.0; value } \
         }",
    );

    let mut table = NameTable::new(name_table::IdentifierNamespace::Logos);
    table
        .intern(Name::new("Preexisting"))
        .expect("fixture name fits Logos namespace");
    let length_before = table.len();
    let identity_before = table.identity().expect("identity before");

    let error = item
        .decode_atomically(&mut table)
        .expect_err("the second method's `let` body is out of subset");
    assert!(
        matches!(error, Error::UnsupportedStatement { .. }),
        "got {error:?}"
    );

    assert_eq!(table.len(), length_before, "no names leaked");
    assert_eq!(
        table.identity().expect("identity after").bytes(),
        identity_before.bytes(),
        "the table is byte-identical after a failed decode",
    );
}
