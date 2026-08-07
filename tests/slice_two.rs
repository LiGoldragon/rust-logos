use core_logos::{
    WholeLogos, WholeLogosItem, WholeLogosStruct, WholeLogosTypeApplication,
    WholeLogosTypeReference, WholeLogosVisibility,
};
use name_table::{
    EncodedName, FilePlacement, ModulePlacement, NameAssociation, NameView, TextualMetadata,
    TextualName,
};
use rust_logos::RustLogos;
use std::collections::BTreeMap;

struct Names(BTreeMap<EncodedName, TextualMetadata>);
impl NameView for Names {
    fn association(&self, _: &EncodedName) -> Option<&NameAssociation> {
        None
    }
    fn textual_metadata(&self, name: &EncodedName) -> Option<&TextualMetadata> {
        self.0.get(name)
    }
}
fn name(seed: u8) -> EncodedName {
    EncodedName::from_archive_bytes([seed; 16])
}

#[test]
fn vector_translation_covers_scalar_parameter_and_application_heads() {
    let declaration = name(3);
    let vector = name(4);
    let logos = WholeLogos::new(vec![WholeLogosItem::Struct(WholeLogosStruct::new(
        WholeLogosVisibility::Public,
        declaration,
        vec![
            WholeLogosTypeReference::Identity(vector),
            WholeLogosTypeReference::Parameter(vector),
            WholeLogosTypeReference::Application(
                WholeLogosTypeApplication::new(
                    vector,
                    vec![WholeLogosTypeReference::Identity(vector)],
                )
                .expect("nonempty application"),
            ),
        ],
    ))]);
    let names = Names(BTreeMap::from([
        (
            declaration,
            TextualMetadata::new(
                TextualName::new("Collection"),
                ModulePlacement::new(vec![]),
                FilePlacement::new(vec![]),
            ),
        ),
        (
            vector,
            TextualMetadata::new(
                TextualName::new("Vector"),
                ModulePlacement::new(vec![]),
                FilePlacement::new(vec![]),
            ),
        ),
    ]));
    assert_eq!(
        RustLogos::new().emit(&logos, &names).expect("emit"),
        "pub struct Collection(Vec, Vec, Vec<Vec>);"
    );
}
