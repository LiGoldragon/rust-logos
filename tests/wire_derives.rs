use std::collections::BTreeMap;

use core_logos::{
    WholeLogos, WholeLogosEnumeration, WholeLogosItem, WholeLogosNewtype, WholeLogosStruct,
    WholeLogosTraitImpl, WholeLogosTypeAttributes, WholeLogosTypeReference, WholeLogosVariant,
    WholeLogosVariantPayload, WholeLogosVisibility,
};
use name_table::{
    EncodedName, FilePlacement, ModulePlacement, NameAssociation, NameView, TextualMetadata,
    TextualName,
};
use rust_logos::RustLogos;

struct Names(BTreeMap<EncodedName, TextualMetadata>);

impl Names {
    fn with(entries: &[(EncodedName, &str)]) -> Self {
        Self(
            entries
                .iter()
                .map(|(encoded_name, spelling)| {
                    (
                        *encoded_name,
                        TextualMetadata::new(
                            TextualName::new(*spelling),
                            ModulePlacement::new(vec![]),
                            FilePlacement::new(vec![]),
                        ),
                    )
                })
                .collect(),
        )
    }
}

impl NameView for Names {
    fn association(&self, _: &EncodedName) -> Option<&NameAssociation> {
        None
    }
    fn textual_metadata(&self, encoded_name: &EncodedName) -> Option<&TextualMetadata> {
        self.0.get(encoded_name)
    }
}

fn name(seed: u8) -> EncodedName {
    EncodedName::from_archive_bytes([seed; 16])
}

#[test]
fn wire_attributes_emit_derive_preamble_on_struct() {
    let declaration = name(1);
    let field = name(2);
    let logos = WholeLogos::new(vec![WholeLogosItem::Struct(
        WholeLogosStruct::new(
            WholeLogosVisibility::Public,
            declaration,
            vec![WholeLogosTypeReference::Identity(field)],
        )
        .with_attributes(WholeLogosTypeAttributes::Wire),
    )]);
    let names = Names::with(&[(declaration, "Entry"), (field, "String")]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert_eq!(
        emitted,
        "#[derive(Clone, Debug, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]\npub struct Entry(String);"
    );
}

#[test]
fn wire_attributes_emit_derive_preamble_on_newtype() {
    let declaration = name(3);
    let wrapped = name(4);
    let logos = WholeLogos::new(vec![WholeLogosItem::Newtype(
        WholeLogosNewtype::new(
            WholeLogosVisibility::Public,
            declaration,
            WholeLogosVisibility::Private,
            WholeLogosTypeReference::Identity(wrapped),
        )
        .with_attributes(WholeLogosTypeAttributes::Wire),
    )]);
    let names = Names::with(&[(declaration, "Marker"), (wrapped, "String")]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert!(
        emitted.starts_with("#[derive(Clone, Debug, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]"),
        "expected derive preamble, got: {emitted}"
    );
    assert!(emitted.contains("pub struct Marker(String);"));
}

#[test]
fn wire_attributes_emit_derive_preamble_on_enumeration() {
    let declaration = name(5);
    let variant_a = name(6);
    let variant_b = name(7);
    let logos = WholeLogos::new(vec![WholeLogosItem::Enumeration(
        WholeLogosEnumeration::new(
            WholeLogosVisibility::Public,
            declaration,
            vec![
                WholeLogosVariant::new(variant_a, WholeLogosVariantPayload::Unit),
                WholeLogosVariant::new(variant_b, WholeLogosVariantPayload::Unit),
            ],
        )
        .with_attributes(WholeLogosTypeAttributes::Wire),
    )]);
    let names = Names::with(&[
        (declaration, "Kind"),
        (variant_a, "Decision"),
        (variant_b, "Principle"),
    ]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert!(
        emitted.starts_with("#[derive(Clone, Debug, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]"),
        "expected derive preamble, got: {emitted}"
    );
    assert!(emitted.contains("pub enum Kind {"));
}

#[test]
fn plain_attributes_emit_no_derives() {
    let declaration = name(10);
    let field = name(11);
    let logos = WholeLogos::new(vec![WholeLogosItem::Struct(WholeLogosStruct::new(
        WholeLogosVisibility::Public,
        declaration,
        vec![WholeLogosTypeReference::Identity(field)],
    ))]);
    let names = Names::with(&[(declaration, "Vocabulary"), (field, "String")]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert_eq!(emitted, "pub struct Vocabulary(String);");
}

#[test]
fn refusal_display_trait_emits_display_and_error() {
    let refusal_trait = name(20);
    let refusal_type = name(21);
    let field_type = name(22);

    let logos = WholeLogos::new(vec![
        WholeLogosItem::Struct(
            WholeLogosStruct::new(
                WholeLogosVisibility::Public,
                refusal_type,
                vec![WholeLogosTypeReference::Identity(field_type)],
            )
            .with_attributes(WholeLogosTypeAttributes::Wire),
        ),
        WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
            WholeLogosTypeReference::Identity(refusal_trait),
            WholeLogosTypeReference::Identity(refusal_type),
            Vec::new(),
        )),
    ]);
    let names = Names::with(&[
        (refusal_trait, "Refusal"),
        (refusal_type, "QueryRejected"),
        (field_type, "String"),
    ]);

    let emitter = RustLogos::new().with_refusal_display_trait(refusal_trait);
    let emitted = emitter.emit(&logos, &names).expect("emit");

    assert!(
        emitted.contains("impl std::fmt::Display for QueryRejected"),
        "expected Display impl, got: {emitted}"
    );
    assert!(
        emitted.contains("impl std::error::Error for QueryRejected"),
        "expected Error impl, got: {emitted}"
    );
}

#[test]
fn builtin_unit_translates_to_rust_unit() {
    let declaration = name(30);
    let unit = name(31);
    let logos = WholeLogos::new(vec![WholeLogosItem::Newtype(WholeLogosNewtype::new(
        WholeLogosVisibility::Public,
        declaration,
        WholeLogosVisibility::Private,
        WholeLogosTypeReference::Identity(unit),
    ))]);
    let names = Names::with(&[(declaration, "Marker"), (unit, "Unit")]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert_eq!(emitted, "pub struct Marker(());");
}

#[test]
fn builtin_boolean_translates_to_rust_bool() {
    let declaration = name(32);
    let boolean = name(33);
    let logos = WholeLogos::new(vec![WholeLogosItem::Newtype(WholeLogosNewtype::new(
        WholeLogosVisibility::Public,
        declaration,
        WholeLogosVisibility::Private,
        WholeLogosTypeReference::Identity(boolean),
    ))]);
    let names = Names::with(&[(declaration, "Flag"), (boolean, "Boolean")]);

    let emitted = RustLogos::new().emit(&logos, &names).expect("emit");

    assert_eq!(emitted, "pub struct Flag(bool);");
}
