use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use name_table::{LocalEncodedId, Name};
use raw_discovery::SourceBound;
use rust_logos::{
    Error, RustEmittedIdentifier, RustLogos, RustNameProjectionTable, RustNewtypeVocabulary,
    RustNewtypeVocabularyIds,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, NameOccurrence,
    ResolvedReference,
};

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
    declaration_type: VocabularyEncodedId,
    reference_type: VocabularyEncodedId,
    first: VocabularyEncodedId,
    second: VocabularyEncodedId,
    integer: VocabularyEncodedId,
}

fn fixture() -> Fixture {
    let struct_keyword_type = encoded(VocabularyRoot::Rust, &[10]);
    let public_keyword_type = encoded(VocabularyRoot::Rust, &[11]);
    let declaration_type = encoded(VocabularyRoot::Rust, &[12]);
    let reference_type = encoded(VocabularyRoot::Rust, &[13]);
    let struct_keyword = encoded(VocabularyRoot::Rust, &[1]);
    let public_keyword = encoded(VocabularyRoot::Rust, &[2]);

    let mut rust_names = Names::default();
    rust_names.add(struct_keyword_type.clone(), "StructKeywordToken");
    rust_names.add(public_keyword_type.clone(), "PublicKeywordToken");
    rust_names.add(declaration_type.clone(), "DeclarationToken");
    rust_names.add(reference_type.clone(), "ReferenceToken");
    rust_names.add(struct_keyword.clone(), "struct");
    rust_names.add(public_keyword.clone(), "pub");

    let ids = RustNewtypeVocabularyIds::new(
        struct_keyword_type,
        public_keyword_type,
        declaration_type.clone(),
        reference_type.clone(),
        struct_keyword,
        public_keyword,
    );
    let vocabulary = RustNewtypeVocabulary::seal(ids, &rust_names).expect("sealed Rust vocabulary");

    Fixture {
        codec: RustLogos::new(vocabulary),
        declaration_type,
        reference_type,
        first: encoded(VocabularyRoot::Universal, &[7, 16]),
        second: encoded(VocabularyRoot::Universal, &[7, 17]),
        integer: encoded(VocabularyRoot::Universal, &[3]),
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
    fn declaration(&mut self, bound: SourceBound, token: &str, encoded_id: VocabularyEncodedId) {
        self.projected.insert(encoded_id.clone(), Name::new(token));
        self.declarations
            .insert((bound.start(), bound.end()), (token.to_owned(), encoded_id));
    }

    fn reference(&mut self, bound: SourceBound, token: &str, encoded_id: VocabularyEncodedId) {
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

fn projections(fixture: &Fixture) -> RustNameProjectionTable {
    RustNameProjectionTable::try_from_entries([
        (
            fixture.first.clone(),
            RustEmittedIdentifier::try_new("Id16").expect("opaque token"),
        ),
        (
            fixture.second.clone(),
            RustEmittedIdentifier::try_new("Id17").expect("opaque token"),
        ),
        (
            fixture.integer.clone(),
            RustEmittedIdentifier::try_new("u64").expect("opaque token"),
        ),
    ])
    .expect("projection table")
}

#[test]
fn two_pass_decode_preserves_absolute_bounds_and_token_longest_match() {
    let fixture = fixture();
    let source = "pub struct Id16(u64);\nstruct Id17(pub u64);\n";
    let first_name = bound(source, "Id16", 0);
    let second_name = bound(source, "Id17", 0);
    let first_integer = bound(source, "u64", 0);
    let second_integer = bound(source, "u64", 1);
    let mut bindings = Bindings::default();
    bindings.declaration(first_name, "Id16", fixture.first.clone());
    bindings.declaration(second_name, "Id17", fixture.second.clone());
    bindings.reference(first_integer, "u64", fixture.integer.clone());
    bindings.reference(second_integer, "u64", fixture.integer.clone());

    let logos = fixture
        .codec
        .decode(source, &bindings)
        .expect("typed Rust decode");
    assert_eq!(logos.items().len(), 2);
    assert_eq!(
        bindings.declaration_occurrences.borrow().as_slice(),
        [
            ("Id16".to_owned(), first_name),
            ("Id17".to_owned(), second_name),
        ]
    );
    assert_eq!(
        bindings.reference_occurrences.borrow().as_slice(),
        [
            ("u64".to_owned(), first_integer),
            ("u64".to_owned(), second_integer),
        ]
    );

    let emitted = fixture
        .codec
        .emit(&logos, &projections(&fixture))
        .expect("structural Rust emission");
    assert_eq!(emitted, source);

    // `Id16` is one longest token. No `Id1` occurrence was queried, so a
    // source-order fallback cannot have supplied the declaration.
    assert!(
        bindings
            .declaration_occurrences
            .borrow()
            .iter()
            .all(|(token, _)| token != "Id1")
    );
}

#[test]
fn declaration_and_reference_roles_refuse_missing_lookup_without_allocation() {
    let fixture = fixture();
    let source = "pub struct Id16(Missing);";
    let mut bindings = Bindings::default();
    bindings.declaration(bound(source, "Id16", 0), "Id16", fixture.first.clone());

    let error = fixture
        .codec
        .decode(source, &bindings)
        .expect_err("missing lookup must refuse");
    assert!(
        matches!(
            error,
            Error::Decode(DecodeError::UnresolvedReference { bound: found })
                if found == bound(source, "Missing", 0)
        ),
        "{error:?}"
    );
    assert_eq!(bindings.references.len(), 0);
}

#[test]
fn attributes_named_fields_and_extra_tuple_fields_are_refused() {
    let fixture = fixture();
    for source in [
        "#[derive(Clone)] pub struct Id16(u64);",
        "pub struct Id16 { value: u64 };",
        "pub struct Id16(u64, u64);",
    ] {
        let bindings = Bindings::default();
        assert!(fixture.codec.decode(source, &bindings).is_err(), "{source}");
    }
}

#[test]
fn opaque_projection_validation_does_not_define_a_chain_encoding() {
    let fixture = fixture();
    assert!(RustEmittedIdentifier::try_new("17.4").is_err());
    assert!(RustEmittedIdentifier::try_new("struct").is_err());

    let same_token = RustEmittedIdentifier::try_new("Opaque").expect("token");
    assert!(matches!(
        RustNameProjectionTable::try_from_entries([
            (fixture.first.clone(), same_token.clone()),
            (fixture.second.clone(), same_token),
        ]),
        Err(Error::ProjectionTokenConflict { .. })
    ));
    assert!(matches!(
        RustNameProjectionTable::try_from_entries([(
            encoded(VocabularyRoot::Rust, &[99]),
            RustEmittedIdentifier::try_new("RustOwned").expect("token"),
        )]),
        Err(Error::NonUniversalIdentity { .. })
    ));
}

#[test]
fn structurally_emitted_rust_compiles_and_runs() {
    let fixture = fixture();
    let logos = core_logos::WholeLogos::new(vec![core_logos::WholeLogosItem::Newtype(
        core_logos::WholeLogosNewtype::new(
            core_logos::WholeLogosVisibility::Public,
            fixture.first.clone(),
            core_logos::WholeLogosVisibility::Private,
            fixture.integer.clone(),
        ),
    )]);
    let emitted = fixture
        .codec
        .emit(&logos, &projections(&fixture))
        .expect("emit Rust");
    assert_eq!(emitted, "pub struct Id16(u64);\n");

    let temporary =
        std::env::temp_dir().join(format!("rust-logos-slice-one-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).expect("clear prior process-local fixture");
    }
    fs::create_dir(&temporary).expect("create process-local fixture");
    let source = temporary.join("main.rs");
    let binary = temporary.join("witness");
    fs::write(
        &source,
        format!("{emitted}\nfn main() {{ let value = Id16(41); assert_eq!(value.0 + 1, 42); }}\n"),
    )
    .expect("write generated witness");

    let compilation = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("run rustc");
    assert!(
        compilation.status.success(),
        "rustc stderr:\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(&binary)
        .output()
        .expect("run generated binary");
    assert!(
        execution.status.success(),
        "generated binary stderr:\n{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    fs::remove_dir_all(&temporary).expect("remove process-local fixture");
}

#[test]
fn fixture_types_are_distinct_rust_root_addresses() {
    let fixture = fixture();
    assert_eq!(
        fixture.declaration_type.root_variant(),
        &VocabularyRoot::Rust
    );
    assert_eq!(fixture.reference_type.root_variant(), &VocabularyRoot::Rust);
    assert_ne!(fixture.declaration_type, fixture.reference_type);
}
