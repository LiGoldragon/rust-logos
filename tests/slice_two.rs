//! Compiling projection witness for the second WholeLogos vocabulary slice.

use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use core_logos::{
    WholeLogos, WholeLogosAssociatedTypeBinding, WholeLogosItem, WholeLogosStruct,
    WholeLogosTraitDef, WholeLogosTraitImpl, WholeLogosTraitMethod, WholeLogosTypeApplication,
    WholeLogosTypeReference, WholeLogosVisibility,
};
use name_table::{LocalEncodedId, Name};
use rust_logos::{
    FixtureRustEmittedIdentifier, FixtureRustNameProjectionTable, FixtureRustVocabulary,
    FixtureRustVocabularyIds, RustLogos,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::EncodedNameResolver;

fn encoded(root: VocabularyRoot, chain: &[u16]) -> VocabularyEncodedId {
    VocabularyEncodedId::new(
        root,
        chain.iter().copied().map(LocalEncodedId::new).collect(),
    )
    .expect("complete fixture identity")
}

fn universal(local: u16) -> VocabularyEncodedId {
    encoded(VocabularyRoot::Universal, &[local])
}

fn reference(identity: &VocabularyEncodedId) -> WholeLogosTypeReference {
    WholeLogosTypeReference::Identity(identity.clone())
}

#[derive(Default)]
struct Names(BTreeMap<VocabularyEncodedId, Name>);

impl Names {
    fn add(&mut self, identity: VocabularyEncodedId, spelling: &str) {
        self.0.insert(identity, Name::new(spelling));
    }
}

impl EncodedNameResolver<VocabularyRoot> for Names {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.0.get(encoded_id)
    }
}

fn rust_logos() -> RustLogos {
    let newtype_item = encoded(VocabularyRoot::Rust, &[10]);
    let enumeration_item = encoded(VocabularyRoot::Rust, &[11]);
    let variant = encoded(VocabularyRoot::Rust, &[12]);
    let tuple_field = encoded(VocabularyRoot::Rust, &[13]);
    let type_reference = encoded(VocabularyRoot::Rust, &[14]);
    let struct_keyword = encoded(VocabularyRoot::Rust, &[1]);
    let enum_keyword = encoded(VocabularyRoot::Rust, &[2]);
    let public_keyword = encoded(VocabularyRoot::Rust, &[3]);
    let comma = encoded(VocabularyRoot::Rust, &[4]);
    let semicolon = encoded(VocabularyRoot::Rust, &[5]);
    let mut names = Names::default();
    for (identity, spelling) in [
        (newtype_item.clone(), "NewtypeItemRecord"),
        (enumeration_item.clone(), "EnumerationItemRecord"),
        (variant.clone(), "VariantRecord"),
        (tuple_field.clone(), "TupleFieldRecord"),
        (type_reference.clone(), "TypeReferenceRecord"),
        (struct_keyword.clone(), "struct"),
        (enum_keyword.clone(), "enum"),
        (public_keyword.clone(), "pub"),
        (comma.clone(), ","),
        (semicolon.clone(), ";"),
    ] {
        names.add(identity, spelling);
    }
    RustLogos::new(
        FixtureRustVocabulary::seal(
            FixtureRustVocabularyIds::new(
                newtype_item,
                enumeration_item,
                variant,
                tuple_field,
                type_reference,
                struct_keyword,
                enum_keyword,
                public_keyword,
                comma,
                semicolon,
            ),
            &names,
        )
        .expect("sealed fixture Rust vocabulary"),
    )
}

fn projections(entries: &[(VocabularyEncodedId, &'static str)]) -> FixtureRustNameProjectionTable {
    FixtureRustNameProjectionTable::try_from_entries(entries.iter().map(|(identity, spelling)| {
        (
            identity.clone(),
            FixtureRustEmittedIdentifier::try_new(*spelling).expect("Rust fixture identifier"),
        )
    }))
    .expect("one-to-one fixture projections")
}

#[test]
fn struct_trait_and_associated_type_impl_project_to_compiling_rust() {
    let decision_context = universal(20);
    let entry = universal(21);
    let vector = universal(22);
    let signal_admission = universal(23);
    let record_decision = universal(24);
    let admission_decision = universal(25);
    let unit = universal(26);
    let stream_open = universal(27);
    let filter = universal(28);
    let stream = universal(29);
    let observer = universal(30);
    let receipt_name = universal(31);
    let receipt = universal(32);
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Struct(WholeLogosStruct::new(
            WholeLogosVisibility::Public,
            decision_context.clone(),
            vec![
                reference(&entry),
                WholeLogosTypeReference::Application(WholeLogosTypeApplication::new(
                    vector.clone(),
                    reference(&entry),
                )),
            ],
        )),
        WholeLogosItem::TraitDef(WholeLogosTraitDef::new(
            WholeLogosVisibility::Public,
            signal_admission.clone(),
            vec![WholeLogosTraitMethod::new(
                record_decision.clone(),
                vec![reference(&admission_decision)],
                reference(&unit),
            )],
        )),
        WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
            reference(&stream_open),
            reference(&filter),
            vec![
                WholeLogosAssociatedTypeBinding::new(stream.clone(), reference(&observer)),
                WholeLogosAssociatedTypeBinding::new(receipt_name.clone(), reference(&receipt)),
            ],
        )),
    ]);
    let projected = projections(&[
        (decision_context, "DecisionContext"),
        (entry, "Entry"),
        (vector, "Vec"),
        (signal_admission, "SignalAdmission"),
        (record_decision, "recordDecision"),
        (admission_decision, "AdmissionDecision"),
        (unit, "Unit"),
        (stream_open, "StreamOpen"),
        (filter, "Filter"),
        (stream, "Stream"),
        (observer, "Observer"),
        (receipt_name, "Receipt"),
        (receipt, "ObserverReceipt"),
    ]);
    let emitted = rust_logos()
        .emit_fixture(&logos, &projected)
        .expect("project slice-two Logos");

    assert!(
        emitted.contains("pub struct DecisionContext {"),
        "{emitted}"
    );
    assert!(emitted.contains("pub field_0: Entry"), "{emitted}");
    assert!(emitted.contains("pub field_1: Vec<Entry>"), "{emitted}");
    assert!(!emitted.contains("DecisionContext("), "{emitted}");
    assert!(
        emitted.contains("fn record_decision(&self, parameter_0: AdmissionDecision) -> Unit;"),
        "{emitted}"
    );
    assert!(emitted.contains("impl StreamOpen for Filter"), "{emitted}");
    assert!(emitted.contains("type Stream = Observer;"), "{emitted}");
    assert!(
        emitted.contains("type Receipt = ObserverReceipt;"),
        "{emitted}"
    );

    let temporary =
        std::env::temp_dir().join(format!("rust-logos-slice-two-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).expect("clear prior scratch crate");
    }
    fs::create_dir_all(temporary.join("src")).expect("scratch source directory");
    fs::write(
        temporary.join("Cargo.toml"),
        "[package]\nname = \"slice-two-projection\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("scratch manifest");
    fs::write(
        temporary.join("src/main.rs"),
        format!(
            "pub struct Entry;\npub struct AdmissionDecision;\npub type Unit = ();\npub struct Filter;\npub struct Observer;\npub struct ObserverReceipt;\npub trait StreamOpen {{ type Stream; type Receipt; }}\n{emitted}\nfn main() {{ let _ = DecisionContext {{ field_0: Entry, field_1: Vec::new() }}; }}\n"
        ),
    )
    .expect("scratch generated source");
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--jobs", "2"])
        .current_dir(&temporary)
        .env("CARGO_TARGET_DIR", temporary.join("target"))
        .output()
        .expect("run scratch Cargo check");
    assert!(
        output.status.success(),
        "scratch Cargo stderr:\n{}\ngenerated:\n{emitted}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(&temporary).expect("remove scratch crate");
}
