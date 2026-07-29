use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use core_logos::{
    WholeLogos, WholeLogosEnumeration, WholeLogosItem, WholeLogosNewtype, WholeLogosTupleFields,
    WholeLogosTypeApplication, WholeLogosTypeReference, WholeLogosVariant,
    WholeLogosVariantPayload, WholeLogosVisibility,
};
use name_table::{LocalEncodedId, Name};
use raw_discovery::{BlockDiscoveryError, BoundaryDiscoveryError, SourceBound, TokenProfileError};
use rust_logos::{
    Error, FixtureRustEmittedIdentifier, FixtureRustNameProjectionTable, FixtureRustVocabulary,
    FixtureRustVocabularyIds, RustLogos,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, NameOccurrence,
    ResolvedReference,
};

const SOURCE: &str = "pub struct Id16 ( Vec < u64 > , ) ;\npub enum Id17 { Id171 , Id172 ( u64 , Vec < u64 > , ) , }\n";

fn encoded(root: VocabularyRoot, chain: &[u16]) -> VocabularyEncodedId {
    VocabularyEncodedId::new(
        root,
        chain.iter().copied().map(LocalEncodedId::new).collect(),
    )
    .expect("fixture encoded IDs are non-empty")
}

#[derive(Default)]
struct Names(BTreeMap<VocabularyEncodedId, Name>);

impl Names {
    fn add(&mut self, encoded_id: VocabularyEncodedId, spelling: &str) {
        self.0.insert(encoded_id, Name::new(spelling));
    }
}

impl EncodedNameResolver<VocabularyRoot> for Names {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.0.get(encoded_id)
    }
}

struct Fixture {
    codec: RustLogos,
    newtype: VocabularyEncodedId,
    enumeration: VocabularyEncodedId,
    unit: VocabularyEncodedId,
    payload: VocabularyEncodedId,
    integer: VocabularyEncodedId,
    vector: VocabularyEncodedId,
}

fn fixture() -> Fixture {
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

    let mut rust_names = Names::default();
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
        rust_names.add(identity, spelling);
    }
    let vocabulary = FixtureRustVocabulary::seal(
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
        &rust_names,
    )
    .expect("sealed fixture Rust vocabulary");
    Fixture {
        codec: RustLogos::new(vocabulary),
        newtype: encoded(VocabularyRoot::Universal, &[7, 16]),
        enumeration: encoded(VocabularyRoot::Universal, &[7, 17]),
        unit: encoded(VocabularyRoot::Universal, &[7, 17, 1]),
        payload: encoded(VocabularyRoot::Universal, &[7, 17, 2]),
        integer: encoded(VocabularyRoot::Universal, &[3]),
        vector: encoded(VocabularyRoot::Universal, &[4]),
    }
}

#[derive(Default)]
struct Bindings {
    declarations: BTreeMap<(usize, usize), (String, VocabularyEncodedId)>,
    references: BTreeMap<(usize, usize), (String, VocabularyEncodedId)>,
    projected: BTreeMap<VocabularyEncodedId, Name>,
    declaration_occurrences: RefCell<Vec<(String, SourceBound)>>,
    reference_occurrences: RefCell<Vec<(String, SourceBound)>>,
}

impl Bindings {
    fn declaration(
        &mut self,
        source: &str,
        token: &str,
        occurrence: usize,
        encoded_id: VocabularyEncodedId,
    ) {
        let bound = bound(source, token, occurrence);
        self.projected.insert(encoded_id.clone(), Name::new(token));
        self.declarations
            .insert((bound.start(), bound.end()), (token.to_owned(), encoded_id));
    }

    fn reference(
        &mut self,
        source: &str,
        token: &str,
        occurrence: usize,
        encoded_id: VocabularyEncodedId,
    ) {
        let bound = bound(source, token, occurrence);
        self.projected.insert(encoded_id.clone(), Name::new(token));
        self.references
            .insert((bound.start(), bound.end()), (token.to_owned(), encoded_id));
    }
}

impl EncodedNameResolver<VocabularyRoot> for Bindings {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.projected.get(encoded_id)
    }
}

impl DecodeNameBindings<VocabularyRoot> for Bindings {
    fn declaration_assignment(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        self.declaration_occurrences
            .borrow_mut()
            .push((occurrence.spelling().to_owned(), occurrence.bound()));
        self.declarations
            .get(&(occurrence.bound().start(), occurrence.bound().end()))
            .filter(|(token, _)| token == occurrence.spelling())
            .map(|(_, encoded_id)| DeclarationAssignment::new(encoded_id.clone()))
    }

    fn reference_resolution(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        self.reference_occurrences
            .borrow_mut()
            .push((occurrence.spelling().to_owned(), occurrence.bound()));
        self.references
            .get(&(occurrence.bound().start(), occurrence.bound().end()))
            .filter(|(token, _)| token == occurrence.spelling())
            .map(|(_, encoded_id)| ResolvedReference::new(encoded_id.clone()))
    }
}

fn bound(source: &str, needle: &str, occurrence: usize) -> SourceBound {
    let start = source
        .match_indices(needle)
        .nth(occurrence)
        .expect("fixture occurrence")
        .0;
    SourceBound::checked(source, start, start + needle.len()).expect("fixture bound")
}

fn bindings(source: &str, fixture: &Fixture) -> Bindings {
    let mut bindings = Bindings::default();
    for (token, identity) in [
        ("Id16", fixture.newtype.clone()),
        ("Id17", fixture.enumeration.clone()),
        ("Id171", fixture.unit.clone()),
        ("Id172", fixture.payload.clone()),
    ] {
        bindings.declaration(source, token, 0, identity);
    }
    for occurrence in 0..2 {
        bindings.reference(source, "Vec", occurrence, fixture.vector.clone());
    }
    for occurrence in 0..3 {
        bindings.reference(source, "u64", occurrence, fixture.integer.clone());
    }
    bindings
}

fn projections(fixture: &Fixture) -> FixtureRustNameProjectionTable {
    FixtureRustNameProjectionTable::try_from_entries([
        (
            fixture.newtype.clone(),
            FixtureRustEmittedIdentifier::try_new("Id16").expect("fixture token"),
        ),
        (
            fixture.enumeration.clone(),
            FixtureRustEmittedIdentifier::try_new("Id17").expect("fixture token"),
        ),
        (
            fixture.unit.clone(),
            FixtureRustEmittedIdentifier::try_new("Id171").expect("fixture token"),
        ),
        (
            fixture.payload.clone(),
            FixtureRustEmittedIdentifier::try_new("Id172").expect("fixture token"),
        ),
        (
            fixture.integer.clone(),
            FixtureRustEmittedIdentifier::try_new("u64").expect("fixture token"),
        ),
        (
            fixture.vector.clone(),
            FixtureRustEmittedIdentifier::try_new("Vec").expect("fixture token"),
        ),
    ])
    .expect("fixture projection table")
}

fn expected_logos(fixture: &Fixture) -> WholeLogos {
    WholeLogos::new(vec![
        WholeLogosItem::Newtype(WholeLogosNewtype::new(
            WholeLogosVisibility::Public,
            fixture.newtype.clone(),
            WholeLogosVisibility::Private,
            WholeLogosTypeReference::Application(WholeLogosTypeApplication::new(
                fixture.vector.clone(),
                WholeLogosTypeReference::Identity(fixture.integer.clone()),
            )),
        )),
        WholeLogosItem::Enumeration(WholeLogosEnumeration::new(
            WholeLogosVisibility::Public,
            fixture.enumeration.clone(),
            vec![
                WholeLogosVariant::new(fixture.unit.clone(), WholeLogosVariantPayload::Unit),
                WholeLogosVariant::new(
                    fixture.payload.clone(),
                    WholeLogosVariantPayload::Tuple(
                        WholeLogosTupleFields::new(vec![
                            WholeLogosTypeReference::Identity(fixture.integer.clone()),
                            WholeLogosTypeReference::Application(WholeLogosTypeApplication::new(
                                fixture.vector.clone(),
                                WholeLogosTypeReference::Identity(fixture.integer.clone()),
                            )),
                        ])
                        .expect("non-empty tuple"),
                    ),
                ),
            ],
        )),
    ])
}

#[test]
fn structural_decode_and_emission_round_trip_complete_fixture_shapes() {
    let fixture = fixture();
    let decoded = fixture
        .codec
        .decode_fixture(SOURCE, &bindings(SOURCE, &fixture))
        .expect("typed fixture decode");
    assert_eq!(decoded, expected_logos(&fixture));

    let emitted = fixture
        .codec
        .emit_fixture(&decoded, &projections(&fixture))
        .expect("complete item-record emission");
    let reparsed = fixture
        .codec
        .decode_fixture(&emitted, &bindings(&emitted, &fixture))
        .expect("emitted fixture reparses");
    assert_eq!(reparsed, decoded);
}

#[test]
fn declaration_and_reference_roles_are_distinct_and_lookup_only() {
    let fixture = fixture();
    let mut missing = bindings(SOURCE, &fixture);
    let missing_bound = bound(SOURCE, "u64", 2);
    missing
        .references
        .remove(&(missing_bound.start(), missing_bound.end()));
    let declarations_before = missing.declarations.clone();
    let references_before = missing.references.clone();

    assert!(matches!(
        fixture.codec.decode_fixture(SOURCE, &missing),
        Err(Error::Decode(DecodeError::UnresolvedReference { bound })) if bound == missing_bound
    ));
    assert_eq!(missing.declarations, declarations_before);
    assert_eq!(missing.references, references_before);
}

#[test]
fn unsupported_fields_attributes_and_unclosed_enum_body_refuse() {
    let fixture = fixture();
    for source in [
        "#[derive(Clone)] pub struct Id16 ( u64 , ) ;\n",
        "pub struct Id16 { u64 , } ;\n",
    ] {
        let mut source_bindings = Bindings::default();
        source_bindings.declaration(source, "Id16", 0, fixture.newtype.clone());
        source_bindings.reference(source, "u64", 0, fixture.integer.clone());
        assert!(
            fixture
                .codec
                .decode_fixture(source, &source_bindings)
                .is_err()
        );
    }

    let visible_variant_field = "pub enum Id17 { Id172 ( pub u64 , ) , }\n";
    let mut visible_bindings = Bindings::default();
    visible_bindings.declaration(
        visible_variant_field,
        "Id17",
        0,
        fixture.enumeration.clone(),
    );
    visible_bindings.declaration(visible_variant_field, "Id172", 0, fixture.payload.clone());
    visible_bindings.reference(visible_variant_field, "u64", 0, fixture.integer.clone());
    assert!(matches!(
        fixture
            .codec
            .decode_fixture(visible_variant_field, &visible_bindings),
        Err(Error::UnsupportedVariantFieldVisibility)
    ));

    assert!(matches!(
        fixture
            .codec
            .decode_fixture("pub enum Id17 { Id171 ,", &Bindings::default()),
        Err(Error::Discovery(BlockDiscoveryError::Boundary(
            BoundaryDiscoveryError::Profile(TokenProfileError::UnclosedBoundary { .. })
        )))
    ));
}

#[test]
fn fixture_projection_is_explicit_and_no_partial_source_is_returned() {
    let fixture = fixture();
    assert!(FixtureRustEmittedIdentifier::try_new("17.4").is_err());
    assert!(FixtureRustEmittedIdentifier::try_new("struct").is_err());
    let same = FixtureRustEmittedIdentifier::try_new("Opaque").expect("token");
    assert!(matches!(
        FixtureRustNameProjectionTable::try_from_entries([
            (fixture.newtype.clone(), same.clone()),
            (fixture.enumeration.clone(), same),
        ]),
        Err(Error::ProjectionTokenConflict { .. })
    ));

    let incomplete = FixtureRustNameProjectionTable::try_from_entries([(
        fixture.newtype.clone(),
        FixtureRustEmittedIdentifier::try_new("Id16").expect("token"),
    )])
    .expect("incomplete fixture table is constructible");
    assert!(matches!(
        fixture
            .codec
            .emit_fixture(&expected_logos(&fixture), &incomplete),
        Err(Error::MissingProjection { .. })
    ));
}

#[test]
fn structurally_emitted_fixture_compiles_and_runs_exhaustively_with_cargo() {
    let fixture = fixture();
    let emitted = fixture
        .codec
        .emit_fixture(&expected_logos(&fixture), &projections(&fixture))
        .expect("structural fixture emission");
    let temporary = std::env::temp_dir().join(format!("rust-logos-fixture-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).expect("clear prior process-local fixture");
    }
    fs::create_dir_all(temporary.join("src")).expect("create scratch Cargo crate");
    fs::write(
        temporary.join("Cargo.toml"),
        "[package]\nname = \"rust-logos-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write scratch manifest");
    fs::write(
        temporary.join("src/main.rs"),
        format!(
            "{emitted}\nfn score(value: Id17) -> usize {{ match value {{ Id17::Id171 => 1, Id17::Id172(number, values) => number as usize + values.len(), }} }}\nfn main() {{ let wrapped = Id16(vec![1, 2, 3]); assert_eq!(wrapped.0.len(), 3); assert_eq!(score(Id17::Id171), 1); assert_eq!(score(Id17::Id172(40, vec![1, 2])), 42); }}\n"
        ),
    )
    .expect("write scratch program");
    let execution = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .current_dir(&temporary)
        .output()
        .expect("run scratch Cargo crate");
    assert!(
        execution.status.success(),
        "cargo stderr:\n{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    fs::remove_dir_all(&temporary).expect("remove process-local fixture");
}

#[test]
fn production_codec_contains_no_source_concatenation_path() {
    let source = include_str!("../src/codec.rs");
    for forbidden in ["push_str", "format!", ".concat(", ".join("] {
        assert!(
            !source.contains(forbidden),
            "fixture codec contains forbidden source concatenation surface {forbidden}"
        );
    }
}
