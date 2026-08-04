//! Compiling projection witness for the second WholeLogos vocabulary slice.

use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use core_logos::{
    WholeLogos, WholeLogosAssociatedTypeBinding, WholeLogosEnumeration, WholeLogosItem,
    WholeLogosNewtype, WholeLogosStorageFingerprint, WholeLogosStruct, WholeLogosTable,
    WholeLogosTraitDef, WholeLogosTraitImpl, WholeLogosTraitMethod, WholeLogosTupleFields,
    WholeLogosTypeApplication, WholeLogosTypeAttributes, WholeLogosTypeParameter,
    WholeLogosTypeReference, WholeLogosVariant, WholeLogosVariantPayload, WholeLogosVisibility,
};
use name_table::{LocalEncodedId, Name};
use rust_logos::{
    Error, FixtureRustEmittedIdentifier, FixtureRustNameProjectionTable, FixtureRustVocabulary,
    FixtureRustVocabularyIds, InterfaceRustEmission, InterfaceRustRoleIds, RustEncodedIdCodec,
    RustLogos, RustTypePath, RustTypePathResolver,
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

#[derive(Default)]
struct TypePaths(BTreeMap<VocabularyEncodedId, RustTypePath>);

impl RustTypePathResolver for TypePaths {
    fn resolve_type_path(&self, encoded_id: &VocabularyEncodedId) -> Option<&RustTypePath> {
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
    let result = universal(33);
    let ordered = universal(34);
    let error = universal(35);
    let wire_result = universal(36);
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Newtype(
            WholeLogosNewtype::new(
                WholeLogosVisibility::Public,
                wire_result.clone(),
                WholeLogosVisibility::Private,
                WholeLogosTypeReference::Application(
                    WholeLogosTypeApplication::new(
                        result.clone(),
                        vec![
                            WholeLogosTypeReference::Application(
                                WholeLogosTypeApplication::new(
                                    vector.clone(),
                                    vec![WholeLogosTypeReference::Parameter(ordered.clone())],
                                )
                                .expect("nested Vector parameter application"),
                            ),
                            reference(&error),
                        ],
                    )
                    .expect("n-ary Result parameter application"),
                ),
            )
            .with_type_parameters(vec![WholeLogosTypeParameter::new(
                ordered.clone(),
                ordered.clone(),
            )]),
        ),
        WholeLogosItem::Struct(WholeLogosStruct::new(
            WholeLogosVisibility::Public,
            decision_context.clone(),
            vec![
                reference(&entry),
                WholeLogosTypeReference::Application(
                    WholeLogosTypeApplication::new(vector.clone(), vec![reference(&entry)])
                        .expect("non-empty Vector application"),
                ),
                WholeLogosTypeReference::Application(
                    WholeLogosTypeApplication::new(
                        result.clone(),
                        vec![
                            WholeLogosTypeReference::Application(
                                WholeLogosTypeApplication::new(
                                    vector.clone(),
                                    vec![reference(&ordered)],
                                )
                                .expect("nested Vector application"),
                            ),
                            reference(&error),
                        ],
                    )
                    .expect("n-ary Result application"),
                ),
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
        (vector, "Vector"),
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
        (result, "Result"),
        (ordered, "Ordered"),
        (error, "Error"),
        (wire_result, "WireResult"),
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
    assert!(
        emitted.contains("pub field_2: Result<Vec<Ordered>, Error>"),
        "{emitted}"
    );
    assert!(
        emitted.contains("pub struct WireResult<Ordered: Ord>(Result<Vec<Ordered>, Error>);"),
        "{emitted}"
    );
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
            "pub struct Entry;\npub struct AdmissionDecision;\npub type Unit = ();\npub struct Filter;\npub struct Observer;\npub struct ObserverReceipt;\n#[derive(Eq, PartialEq, Ord, PartialOrd)]\npub struct Ordered;\npub struct Error;\npub trait StreamOpen {{ type Stream; type Receipt; }}\n{emitted}\nfn main() {{ let _ = DecisionContext {{ field_0: Entry, field_1: Vec::new(), field_2: Ok(Vec::new()) }}; let _ = WireResult::<Ordered>(Ok(vec![Ordered])); }}\n"
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

#[test]
fn wire_policy_projects_the_existing_interface_attribute_preamble() {
    let wrapped = universal(40);
    let newtype_name = universal(41);
    let struct_name = universal(42);
    let enumeration_name = universal(43);
    let variant_name = universal(44);
    let wire = WholeLogosTypeAttributes::Wire;
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Newtype(
            WholeLogosNewtype::new(
                WholeLogosVisibility::Public,
                newtype_name.clone(),
                WholeLogosVisibility::Private,
                reference(&wrapped),
            )
            .with_attributes(wire),
        ),
        WholeLogosItem::Struct(
            WholeLogosStruct::new(
                WholeLogosVisibility::Public,
                struct_name.clone(),
                vec![reference(&wrapped)],
            )
            .with_attributes(wire),
        ),
        WholeLogosItem::Enumeration(
            WholeLogosEnumeration::new(
                WholeLogosVisibility::Public,
                enumeration_name.clone(),
                vec![WholeLogosVariant::new(
                    variant_name.clone(),
                    WholeLogosVariantPayload::Tuple(
                        WholeLogosTupleFields::new(vec![reference(&wrapped)])
                            .expect("single wire payload"),
                    ),
                )],
            )
            .with_attributes(wire),
        ),
    ]);
    let emitted = rust_logos()
        .emit_fixture(
            &logos,
            &projections(&[
                (wrapped, "Payload"),
                (newtype_name, "WireNewtype"),
                (struct_name, "WireStruct"),
                (enumeration_name, "WireEnumeration"),
                (variant_name, "Batch"),
            ]),
        )
        .expect("project typed wire policy");

    assert_eq!(emitted.matches("#[rustfmt::skip]").count(), 3, "{emitted}");
    assert_eq!(emitted.matches("rkyv::Archive").count(), 3, "{emitted}");
    assert_eq!(
        emitted.matches("nota::NotaDecodeTraced").count(),
        3,
        "{emitted}"
    );
    assert!(emitted.contains("Batch(Payload)"), "{emitted}");
}

#[test]
fn stored_policy_and_table_shape_project_to_the_sema_engine_trait() {
    let record = universal(50);
    let field = universal(51);
    let key = universal(52);
    let table = universal(53);
    let stored_newtype = universal(54);
    let stored_enumeration = universal(55);
    let stored_variant = universal(56);
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Newtype(
            WholeLogosNewtype::new(
                WholeLogosVisibility::Public,
                stored_newtype.clone(),
                WholeLogosVisibility::Private,
                reference(&field),
            )
            .with_attributes(WholeLogosTypeAttributes::Stored),
        ),
        WholeLogosItem::Enumeration(
            WholeLogosEnumeration::new(
                WholeLogosVisibility::Public,
                stored_enumeration.clone(),
                vec![WholeLogosVariant::new(
                    stored_variant.clone(),
                    WholeLogosVariantPayload::Tuple(
                        WholeLogosTupleFields::new(vec![reference(&field)])
                            .expect("single stored payload"),
                    ),
                )],
            )
            .with_attributes(WholeLogosTypeAttributes::Stored),
        ),
        WholeLogosItem::Struct(
            WholeLogosStruct::new(
                WholeLogosVisibility::Public,
                record.clone(),
                vec![reference(&field)],
            )
            .with_attributes(WholeLogosTypeAttributes::Stored),
        ),
        WholeLogosItem::Table(WholeLogosTable::new(
            table.clone(),
            reference(&record),
            reference(&key),
            WholeLogosStorageFingerprint::new([7; 32]),
            WholeLogosStorageFingerprint::new([8; 32]),
        )),
    ]);
    let emitted = rust_logos()
        .emit_fixture(
            &logos,
            &projections(&[
                (record, "StoredRecord"),
                (field, "Entry"),
                (key, "Domain"),
                (table.clone(), "Records"),
                (stored_newtype, "StoredNewtype"),
                (stored_enumeration, "StoredEnumeration"),
                (stored_variant, "StoredVariant"),
            ]),
        )
        .expect("project stored record and table");

    assert_eq!(emitted.matches("rkyv::Archive").count(), 3, "{emitted}");
    assert!(!emitted.contains("nota::NotaDecode"), "{emitted}");
    assert!(emitted.contains("pub struct StoredNewtype"), "{emitted}");
    assert!(emitted.contains("pub enum StoredEnumeration"), "{emitted}");
    assert!(emitted.contains("pub struct Records;"), "{emitted}");
    assert!(
        emitted.contains("impl sema_engine::TableSpecification for Records"),
        "{emitted}"
    );
    assert!(emitted.contains("type Record = StoredRecord;"), "{emitted}");
    assert!(emitted.contains("type Key = Domain;"), "{emitted}");
    assert!(
        emitted.contains("sema_engine::TableName::new(\"Records\")"),
        "{emitted}"
    );
    assert!(
        emitted.contains(&format!(
            "const FAMILY_NAME: &'static str = \"{}\";",
            RustEncodedIdCodec::encode(&table)
        )),
        "{emitted}"
    );
}

#[test]
fn production_references_use_validated_external_paths_without_redeclaring_the_type() {
    let record = universal(57);
    let domain = universal(58);
    let table = universal(59);
    let wrapper = universal(67);
    let choice = universal(68);
    let carrying = universal(69);
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Newtype(WholeLogosNewtype::new(
            WholeLogosVisibility::Public,
            wrapper.clone(),
            WholeLogosVisibility::Private,
            reference(&domain),
        )),
        WholeLogosItem::Enumeration(WholeLogosEnumeration::new(
            WholeLogosVisibility::Public,
            choice.clone(),
            vec![WholeLogosVariant::new(
                carrying.clone(),
                WholeLogosVariantPayload::Tuple(
                    WholeLogosTupleFields::new(vec![reference(&domain)])
                        .expect("one-field external payload"),
                ),
            )],
        )),
        WholeLogosItem::Struct(
            WholeLogosStruct::new(
                WholeLogosVisibility::Public,
                record.clone(),
                vec![reference(&domain)],
            )
            .with_attributes(WholeLogosTypeAttributes::Stored),
        ),
        WholeLogosItem::Table(WholeLogosTable::new(
            table.clone(),
            reference(&record),
            reference(&domain),
            WholeLogosStorageFingerprint::new([11; 32]),
            WholeLogosStorageFingerprint::new([12; 32]),
        )),
    ]);
    let mut names = Names::default();
    names.add(record.clone(), "StoredRecord");
    names.add(domain.clone(), "Domain");
    names.add(table, "records");
    names.add(wrapper.clone(), "DomainWrapper");
    names.add(choice, "DomainChoice");
    names.add(carrying.clone(), "Carrying");
    let mut paths = TypePaths::default();
    paths.0.insert(
        domain,
        RustTypePath::try_new(vec!["signal_domain".to_owned(), "Domain".to_owned()])
            .expect("external Domain path"),
    );

    let emitted = rust_logos()
        .emit_with_type_paths(&logos, &names, &paths)
        .expect("emit external storage type path");

    assert!(emitted.contains("signal_domain::Domain"), "{emitted}");
    assert!(
        emitted.contains(&format!(
            "pub struct {}(signal_domain::Domain);",
            RustEncodedIdCodec::encode(&wrapper)
        )),
        "{emitted}"
    );
    assert!(
        emitted.contains(&format!(
            "{}(signal_domain::Domain)",
            RustEncodedIdCodec::encode(&carrying)
        )),
        "{emitted}"
    );
    assert!(!emitted.contains("struct Domain"), "{emitted}");
    assert!(matches!(
        RustTypePath::try_new(vec!["signal_domain::Domain".to_owned()]),
        Err(Error::InvalidExternalRustTypePath { .. })
    ));
}

#[test]
fn interface_roles_compile_and_refusal_display_is_real_behavior() {
    let input_role = universal(60);
    let output_role = universal(61);
    let refusal_role = universal(62);
    let input = universal(63);
    let output = universal(64);
    let refusal = universal(65);
    let payload = universal(66);
    let roles = InterfaceRustRoleIds::new(
        input_role.clone(),
        output_role.clone(),
        refusal_role.clone(),
    )
    .expect("distinct Universal roles");
    let logos = WholeLogos::new(vec![
        WholeLogosItem::Newtype(WholeLogosNewtype::new(
            WholeLogosVisibility::Public,
            input.clone(),
            WholeLogosVisibility::Private,
            reference(&payload),
        )),
        WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
            reference(&input_role),
            reference(&input),
            Vec::new(),
        )),
        WholeLogosItem::Newtype(WholeLogosNewtype::new(
            WholeLogosVisibility::Public,
            output.clone(),
            WholeLogosVisibility::Private,
            reference(&payload),
        )),
        WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
            reference(&output_role),
            reference(&output),
            Vec::new(),
        )),
        WholeLogosItem::Struct(WholeLogosStruct::new(
            WholeLogosVisibility::Public,
            refusal.clone(),
            vec![reference(&payload)],
        )),
        WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
            reference(&refusal_role),
            reference(&refusal),
            Vec::new(),
        )),
    ]);
    let mut allocated = Names::default();
    for identity in [
        &input_role,
        &output_role,
        &refusal_role,
        &input,
        &output,
        &refusal,
        &payload,
    ] {
        allocated.add(identity.clone(), "allocated");
    }
    let emitted = rust_logos()
        .emit_interface(&logos, &allocated, &roles)
        .expect("emit complete Interface role behavior");

    let input_role = RustEncodedIdCodec::encode(&input_role);
    let output_role = RustEncodedIdCodec::encode(&output_role);
    let refusal_role = RustEncodedIdCodec::encode(&refusal_role);
    let input = RustEncodedIdCodec::encode(&input);
    let output = RustEncodedIdCodec::encode(&output);
    let refusal = RustEncodedIdCodec::encode(&refusal);
    let payload = RustEncodedIdCodec::encode(&payload);
    assert!(
        emitted.contains(&format!("impl {input_role} for {input}")),
        "{emitted}"
    );
    assert!(
        emitted.contains(&format!("impl {output_role} for {output}")),
        "{emitted}"
    );
    assert!(
        emitted.contains(&format!("impl std::error::Error for {refusal}")),
        "{emitted}"
    );
    assert!(!emitted.contains("impl From<"), "{emitted}");

    let temporary =
        std::env::temp_dir().join(format!("rust-logos-interface-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).expect("clear prior scratch crate");
    }
    fs::create_dir_all(temporary.join("src")).expect("scratch source directory");
    fs::write(
        temporary.join("Cargo.toml"),
        "[package]\nname = \"interface-role-projection\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("scratch manifest");
    fs::write(
        temporary.join("src/main.rs"),
        format!(
            "pub trait {input_role} {{}}\npub trait {output_role} {{}}\npub trait {refusal_role}: std::error::Error {{}}\npub type {payload} = u64;\n{emitted}\nimpl std::fmt::Debug for {refusal} {{ fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{ write!(formatter, \"Denied({{}})\", self.field_0) }} }}\nfn assert_input<T: {input_role}>() {{}}\nfn assert_output<T: {output_role}>() {{}}\nfn assert_refusal<T: {refusal_role}>() {{}}\nfn main() {{ assert_input::<{input}>(); assert_output::<{output}>(); assert_refusal::<{refusal}>(); let denied = {refusal} {{ field_0: 7 }}; let error: &dyn std::error::Error = &denied; assert_eq!(error.to_string(), \"Denied(7)\"); }}\n"
        ),
    )
    .expect("scratch generated source");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--offline", "--jobs", "2"])
        .current_dir(&temporary)
        .env("CARGO_TARGET_DIR", temporary.join("target"))
        .output()
        .expect("run scratch Interface witness");
    assert!(
        output.status.success(),
        "scratch Cargo stderr:\n{}\ngenerated:\n{emitted}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(&temporary).expect("remove scratch crate");
}

#[test]
fn refusal_membership_refuses_unruled_associated_type_behavior() {
    let input_role = universal(70);
    let output_role = universal(71);
    let refusal_role = universal(72);
    let refusal = universal(73);
    let associated_name = universal(74);
    let associated_value = universal(75);
    let roles = InterfaceRustRoleIds::new(
        input_role.clone(),
        output_role.clone(),
        refusal_role.clone(),
    )
    .expect("distinct Universal roles");
    let logos = WholeLogos::new(vec![WholeLogosItem::TraitImpl(WholeLogosTraitImpl::new(
        reference(&refusal_role),
        reference(&refusal),
        vec![WholeLogosAssociatedTypeBinding::new(
            associated_name.clone(),
            reference(&associated_value),
        )],
    ))]);
    let mut allocated = Names::default();
    for identity in [
        input_role,
        output_role,
        refusal_role,
        refusal,
        associated_name,
        associated_value,
    ] {
        allocated.add(identity, "allocated");
    }
    assert!(matches!(
        rust_logos().emit_interface(&logos, &allocated, &roles),
        Err(Error::RefusalImplementationAssociatedTypes { found: 1 })
    ));
}
