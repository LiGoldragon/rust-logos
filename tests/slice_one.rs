use std::collections::BTreeMap;

use core_logos::{
    WholeLogos, WholeLogosItem, WholeLogosNewtype, WholeLogosTypeReference, WholeLogosVisibility,
};
use name_table::{
    EncodedName, FilePlacement, ModulePlacement, NameAssociation, NameView, TextualMetadata,
    TextualName,
};
use rust_logos::RustLogos;

#[derive(Default)]
struct Names(BTreeMap<EncodedName, TextualMetadata>);

impl Names {
    fn add(&mut self, encoded_name: EncodedName, spelling: &str) {
        self.0.insert(
            encoded_name,
            TextualMetadata::new(
                TextualName::new(spelling),
                ModulePlacement::new(vec![]),
                FilePlacement::new(vec![]),
            ),
        );
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
fn emission_reads_only_the_authority_textual_view() {
    let declaration = name(1);
    let wrapped = name(2);
    let logos = WholeLogos::new(vec![WholeLogosItem::Newtype(WholeLogosNewtype::new(
        WholeLogosVisibility::Public,
        declaration,
        WholeLogosVisibility::Public,
        WholeLogosTypeReference::Identity(wrapped),
    ))]);
    let mut names = Names::default();
    names.add(declaration, "Message");
    names.add(wrapped, "String");

    assert_eq!(
        RustLogos::new().emit(&logos, &names).expect("emit"),
        "pub struct Message(pub String);"
    );
}
