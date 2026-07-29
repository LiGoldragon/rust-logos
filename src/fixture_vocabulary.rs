//! Typed structural records for the bounded Rust fixture surface.

use std::collections::BTreeMap;

use name_table::Name;
use raw_discovery::{
    BlockTreeDiscoveryConfiguration, BoundaryDiscoveryConfiguration, BoundaryDiscoveryContext,
    BoundaryDiscoveryContextIdentifier, BoundaryDiscoveryTransition, CharacterClass, CharacterSet,
    CueTerminatedBlockDiscoveryConfiguration, CueTerminationRule, CueTerminationRuleIdentifier,
    ProfileRevision, SealedCueTerminatedBlockDiscoveryConfiguration, SealedTokenProfile,
    TokenProfileData, Trigger, TriggerDefinition, TriggerIdentifier, TriggerSet,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, AtomDescriptor, BorrowedFieldView,
    ConstructorCodec, ContextualTextualPolicy, DecodeFormId, EncodedConstructorId,
    EncodedNameResolver, EncodedTypeId, FieldRole, FieldValue, FieldVisitor, OrderedSequence,
    Position, RuleCoproduct, SharedDescriptor, StableRoleId, StructuralEntry,
    StructuralVocabularyIdentity, StructureRecord, TableIdentityPayload, TargetLayoutIdentity,
    TextualRenderingPolicy,
};

use crate::Error;

pub(crate) const PARENTHESIS: TriggerIdentifier = TriggerIdentifier::new(1);
pub(crate) const BRACE: TriggerIdentifier = TriggerIdentifier::new(2);
const ANGLE: TriggerIdentifier = TriggerIdentifier::new(3);
const COMMA: TriggerIdentifier = TriggerIdentifier::new(4);
const SEMICOLON: TriggerIdentifier = TriggerIdentifier::new(5);
const WHITESPACE: TriggerIdentifier = TriggerIdentifier::new(6);
const SQUARE: TriggerIdentifier = TriggerIdentifier::new(7);
const STRING: TriggerIdentifier = TriggerIdentifier::new(8);
const LINE_COMMENT: TriggerIdentifier = TriggerIdentifier::new(9);
const BLOCK_COMMENT: TriggerIdentifier = TriggerIdentifier::new(10);
pub(crate) const STRUCT_CUE: CueTerminationRuleIdentifier = CueTerminationRuleIdentifier::new(1);
pub(crate) const ENUM_CUE: CueTerminationRuleIdentifier = CueTerminationRuleIdentifier::new(2);
const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier = BoundaryDiscoveryContextIdentifier::new(1);
const CONSTRUCTOR_IDENTITY: u16 = 1;
const CONSTRUCTOR_APPLICATION: u16 = 2;
const CONSTRUCTOR_UNIT: u16 = 1;
const CONSTRUCTOR_TUPLE: u16 = 2;
const FORM: DecodeFormId = DecodeFormId::new(1);

macro_rules! role {
    ($name:ident, $id:expr) => {
        #[derive(
            rkyv::Archive,
            rkyv::Serialize,
            rkyv::Deserialize,
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
        )]
        #[rkyv(derive(PartialEq, Eq, PartialOrd, Ord))]
        pub struct $name;

        impl FieldRole for $name {
            const STABLE_ID: u16 = $id;
        }
    };
}

role!(ItemRoot, 21_001);
role!(ItemVisibility, 21_002);
role!(ItemKeyword, 21_003);
role!(ItemName, 21_004);
role!(ItemBody, 21_005);
role!(ItemElements, 21_006);
role!(ItemTerminator, 21_007);
role!(VariantRoot, 21_010);
role!(VariantName, 21_011);
role!(VariantBody, 21_012);
role!(VariantFields, 21_013);
role!(VariantTerminator, 21_014);
role!(TupleFieldRoot, 21_020);
role!(TupleFieldVisibility, 21_021);
role!(TupleFieldType, 21_022);
role!(TupleFieldTerminator, 21_023);
role!(ReferencedTypePosition, 21_030);
role!(ApplicationRoot, 21_031);
role!(ApplicationHead, 21_032);
role!(ApplicationBody, 21_033);
role!(ApplicationPayload, 21_034);

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RustItemRecord {
    root: Position<ItemRoot, VocabularyRoot>,
    visibility: Position<ItemVisibility, VocabularyRoot>,
    keyword: Position<ItemKeyword, VocabularyRoot>,
    name: Position<ItemName, VocabularyRoot>,
    body: Position<ItemBody, VocabularyRoot>,
    elements: Position<ItemElements, VocabularyRoot>,
    terminator: Position<ItemTerminator, VocabularyRoot>,
}

impl RustItemRecord {
    fn new(
        boundary: TriggerIdentifier,
        element: EncodedTypeId<VocabularyRoot>,
        element_maximum: Option<u64>,
        keyword: VocabularyEncodedId,
        public_keyword: VocabularyEncodedId,
        semicolon: VocabularyEncodedId,
        has_terminator: bool,
    ) -> Result<Self, Error> {
        let sequence = OrderedSequence::try_new::<ItemVisibility>()?
            .then::<ItemKeyword>()?
            .then::<ItemName>()?
            .then::<ItemBody>()?
            .then::<ItemTerminator>()?;
        let elements = Position::try_new(SharedDescriptor::Repeated {
            minimum: 1,
            maximum: element_maximum,
            element: Box::new(SharedDescriptor::Delegate {
                target: element,
                payload: None,
            }),
        })?;
        let terminator = if has_terminator {
            SharedDescriptor::Literal(semicolon)
        } else {
            SharedDescriptor::Repeated {
                minimum: 0,
                maximum: Some(0),
                element: Box::new(SharedDescriptor::Literal(semicolon)),
            }
        };
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(sequence))?,
            visibility: Position::try_new(SharedDescriptor::Repeated {
                minimum: 0,
                maximum: Some(1),
                element: Box::new(SharedDescriptor::Literal(public_keyword)),
            })?,
            keyword: Position::try_new(SharedDescriptor::Literal(keyword))?,
            name: Position::try_new(SharedDescriptor::Declaration(AtomDescriptor::any_case()))?,
            body: Position::try_new(SharedDescriptor::Delimited {
                boundary,
                content: elements.role(),
            })?,
            elements,
            terminator: Position::try_new(terminator)?,
        })
    }
}

pub struct RustItemView<'a>(&'a RustItemRecord);

impl BorrowedFieldView<VocabularyRoot> for RustItemView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.visibility);
        visitor.field(&self.0.keyword);
        visitor.field(&self.0.name);
        visitor.field(&self.0.body);
        visitor.field(&self.0.elements);
        visitor.field(&self.0.terminator);
    }
}

impl StructureRecord<VocabularyRoot> for RustItemRecord {
    type View<'a> = RustItemView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        RustItemView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct UnitVariantRecord {
    root: Position<VariantRoot, VocabularyRoot>,
    name: Position<VariantName, VocabularyRoot>,
    terminator: Position<VariantTerminator, VocabularyRoot>,
}

impl UnitVariantRecord {
    fn new(comma: VocabularyEncodedId) -> Result<Self, Error> {
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(
                OrderedSequence::try_new::<VariantName>()?.then::<VariantTerminator>()?,
            ))?,
            name: Position::try_new(SharedDescriptor::Declaration(AtomDescriptor::any_case()))?,
            terminator: Position::try_new(SharedDescriptor::Literal(comma))?,
        })
    }
}

pub struct UnitVariantView<'a>(&'a UnitVariantRecord);

impl BorrowedFieldView<VocabularyRoot> for UnitVariantView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.name);
        visitor.field(&self.0.terminator);
    }
}

impl StructureRecord<VocabularyRoot> for UnitVariantRecord {
    type View<'a> = UnitVariantView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        UnitVariantView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TupleVariantRecord {
    root: Position<VariantRoot, VocabularyRoot>,
    name: Position<VariantName, VocabularyRoot>,
    body: Position<VariantBody, VocabularyRoot>,
    fields: Position<VariantFields, VocabularyRoot>,
    terminator: Position<VariantTerminator, VocabularyRoot>,
}

impl TupleVariantRecord {
    fn new(
        field_type: EncodedTypeId<VocabularyRoot>,
        comma: VocabularyEncodedId,
    ) -> Result<Self, Error> {
        let fields = Position::try_new(SharedDescriptor::Repeated {
            minimum: 1,
            maximum: None,
            element: Box::new(SharedDescriptor::Delegate {
                target: field_type,
                payload: None,
            }),
        })?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(
                OrderedSequence::try_new::<VariantName>()?
                    .then::<VariantBody>()?
                    .then::<VariantTerminator>()?,
            ))?,
            name: Position::try_new(SharedDescriptor::Declaration(AtomDescriptor::any_case()))?,
            body: Position::try_new(SharedDescriptor::Delimited {
                boundary: PARENTHESIS,
                content: fields.role(),
            })?,
            fields,
            terminator: Position::try_new(SharedDescriptor::Literal(comma))?,
        })
    }
}

pub struct TupleVariantView<'a>(&'a TupleVariantRecord);

impl BorrowedFieldView<VocabularyRoot> for TupleVariantView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.name);
        visitor.field(&self.0.body);
        visitor.field(&self.0.fields);
        visitor.field(&self.0.terminator);
    }
}

impl StructureRecord<VocabularyRoot> for TupleVariantRecord {
    type View<'a> = TupleVariantView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        TupleVariantView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TupleFieldRecord {
    root: Position<TupleFieldRoot, VocabularyRoot>,
    visibility: Position<TupleFieldVisibility, VocabularyRoot>,
    reference: Position<TupleFieldType, VocabularyRoot>,
    terminator: Position<TupleFieldTerminator, VocabularyRoot>,
}

impl TupleFieldRecord {
    fn new(
        reference_type: EncodedTypeId<VocabularyRoot>,
        public_keyword: VocabularyEncodedId,
        comma: VocabularyEncodedId,
    ) -> Result<Self, Error> {
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(
                OrderedSequence::try_new::<TupleFieldVisibility>()?
                    .then::<TupleFieldType>()?
                    .then::<TupleFieldTerminator>()?,
            ))?,
            visibility: Position::try_new(SharedDescriptor::Repeated {
                minimum: 0,
                maximum: Some(1),
                element: Box::new(SharedDescriptor::Literal(public_keyword)),
            })?,
            reference: Position::try_new(SharedDescriptor::Delegate {
                target: reference_type,
                payload: None,
            })?,
            terminator: Position::try_new(SharedDescriptor::Literal(comma))?,
        })
    }
}

pub struct TupleFieldView<'a>(&'a TupleFieldRecord);

impl BorrowedFieldView<VocabularyRoot> for TupleFieldView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.visibility);
        visitor.field(&self.0.reference);
        visitor.field(&self.0.terminator);
    }
}

impl StructureRecord<VocabularyRoot> for TupleFieldRecord {
    type View<'a> = TupleFieldView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        TupleFieldView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ReferencedTypeRecord {
    reference: Position<ReferencedTypePosition, VocabularyRoot>,
}

impl ReferencedTypeRecord {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            reference: Position::try_new(SharedDescriptor::Reference(AtomDescriptor::any_case()))?,
        })
    }
}

pub struct ReferencedTypeView<'a>(&'a ReferencedTypeRecord);

impl BorrowedFieldView<VocabularyRoot> for ReferencedTypeView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.reference);
    }
}

impl StructureRecord<VocabularyRoot> for ReferencedTypeRecord {
    type View<'a> = ReferencedTypeView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.reference.role()
    }

    fn fields(&self) -> Self::View<'_> {
        ReferencedTypeView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TypeApplicationRecord {
    root: Position<ApplicationRoot, VocabularyRoot>,
    head: Position<ApplicationHead, VocabularyRoot>,
    body: Position<ApplicationBody, VocabularyRoot>,
    payload: Position<ApplicationPayload, VocabularyRoot>,
}

impl TypeApplicationRecord {
    fn new(reference_type: EncodedTypeId<VocabularyRoot>) -> Result<Self, Error> {
        let payload = Position::try_new(SharedDescriptor::Repeated {
            minimum: 1,
            maximum: Some(1),
            element: Box::new(SharedDescriptor::Delegate {
                target: reference_type,
                payload: None,
            }),
        })?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(
                OrderedSequence::try_new::<ApplicationHead>()?.then::<ApplicationBody>()?,
            ))?,
            head: Position::try_new(SharedDescriptor::Reference(AtomDescriptor::any_case()))?,
            body: Position::try_new(SharedDescriptor::Delimited {
                boundary: ANGLE,
                content: payload.role(),
            })?,
            payload,
        })
    }
}

pub struct TypeApplicationView<'a>(&'a TypeApplicationRecord);

impl BorrowedFieldView<VocabularyRoot> for TypeApplicationView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.head);
        visitor.field(&self.0.body);
        visitor.field(&self.0.payload);
    }
}

impl StructureRecord<VocabularyRoot> for TypeApplicationRecord {
    type View<'a> = TypeApplicationView<'a>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        TypeApplicationView(self)
    }
}

pub type FixtureRustRule = RuleCoproduct<
    RustItemRecord,
    RuleCoproduct<
        UnitVariantRecord,
        RuleCoproduct<
            TupleVariantRecord,
            RuleCoproduct<
                TupleFieldRecord,
                RuleCoproduct<ReferencedTypeRecord, TypeApplicationRecord>,
            >,
        >,
    >,
>;

#[derive(Clone, Debug)]
pub struct FixtureRustVocabularyIds {
    newtype_item: EncodedTypeId<VocabularyRoot>,
    enumeration_item: EncodedTypeId<VocabularyRoot>,
    variant: EncodedTypeId<VocabularyRoot>,
    tuple_field: EncodedTypeId<VocabularyRoot>,
    type_reference: EncodedTypeId<VocabularyRoot>,
    struct_keyword: VocabularyEncodedId,
    enum_keyword: VocabularyEncodedId,
    public_keyword: VocabularyEncodedId,
    comma: VocabularyEncodedId,
    semicolon: VocabularyEncodedId,
}

impl FixtureRustVocabularyIds {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        newtype_item: VocabularyEncodedId,
        enumeration_item: VocabularyEncodedId,
        variant: VocabularyEncodedId,
        tuple_field: VocabularyEncodedId,
        type_reference: VocabularyEncodedId,
        struct_keyword: VocabularyEncodedId,
        enum_keyword: VocabularyEncodedId,
        public_keyword: VocabularyEncodedId,
        comma: VocabularyEncodedId,
        semicolon: VocabularyEncodedId,
    ) -> Self {
        Self {
            newtype_item: EncodedTypeId::new(newtype_item),
            enumeration_item: EncodedTypeId::new(enumeration_item),
            variant: EncodedTypeId::new(variant),
            tuple_field: EncodedTypeId::new(tuple_field),
            type_reference: EncodedTypeId::new(type_reference),
            struct_keyword,
            enum_keyword,
            public_keyword,
            comma,
            semicolon,
        }
    }

    pub(crate) fn newtype_item(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.newtype_item
    }

    pub(crate) fn enumeration_item(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.enumeration_item
    }

    pub(crate) fn variant(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.variant
    }

    pub(crate) fn tuple_field(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.tuple_field
    }

    pub(crate) fn type_reference(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.type_reference
    }

    pub(crate) fn struct_keyword(&self) -> &VocabularyEncodedId {
        &self.struct_keyword
    }

    pub(crate) fn enum_keyword(&self) -> &VocabularyEncodedId {
        &self.enum_keyword
    }

    pub(crate) fn public_keyword(&self) -> &VocabularyEncodedId {
        &self.public_keyword
    }

    pub(crate) fn comma(&self) -> &VocabularyEncodedId {
        &self.comma
    }

    pub(crate) fn semicolon(&self) -> &VocabularyEncodedId {
        &self.semicolon
    }
}

pub struct FixtureRustVocabulary {
    ids: FixtureRustVocabularyIds,
    table: AddressedStructuralTable<VocabularyRoot, FixtureRustRule>,
    rust_profile: SealedTokenProfile,
    rust_discovery: SealedCueTerminatedBlockDiscoveryConfiguration,
    fixed_names: BTreeMap<VocabularyEncodedId, Name>,
}

impl FixtureRustVocabulary {
    pub fn seal<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
        ids: FixtureRustVocabularyIds,
        rust_names: &Resolver,
    ) -> Result<Self, Error> {
        for (position, encoded_type) in [
            ("newtype item type", ids.newtype_item()),
            ("enumeration item type", ids.enumeration_item()),
            ("variant type", ids.variant()),
            ("tuple field type", ids.tuple_field()),
            ("type reference type", ids.type_reference()),
        ] {
            validate_rust_id(position, encoded_type.encoded_id(), rust_names)?;
        }
        let struct_word =
            validate_fixed_word("struct keyword", ids.struct_keyword(), "struct", rust_names)?;
        let enum_word =
            validate_fixed_word("enum keyword", ids.enum_keyword(), "enum", rust_names)?;
        validate_fixed_word("public keyword", ids.public_keyword(), "pub", rust_names)?;
        validate_fixed_word("comma", ids.comma(), ",", rust_names)?;
        validate_fixed_word("semicolon", ids.semicolon(), ";", rust_names)?;

        let profile = token_profile()?;
        let table = seal_table(&ids, &profile)?;
        let (rust_profile, rust_discovery) = rust_discovery(&struct_word, &enum_word)?;
        let fixed_names = [
            ids.struct_keyword(),
            ids.enum_keyword(),
            ids.public_keyword(),
            ids.comma(),
            ids.semicolon(),
        ]
        .into_iter()
        .map(|encoded_id| {
            (
                encoded_id.clone(),
                rust_names
                    .resolve(encoded_id)
                    .expect("fixed Rust word was validated")
                    .clone(),
            )
        })
        .collect();
        Ok(Self {
            ids,
            table,
            rust_profile,
            rust_discovery,
            fixed_names,
        })
    }

    pub fn structuretree(&self) -> &AddressedStructuralTable<VocabularyRoot, FixtureRustRule> {
        &self.table
    }

    pub fn ids(&self) -> &FixtureRustVocabularyIds {
        &self.ids
    }

    pub(crate) fn rust_profile(&self) -> &SealedTokenProfile {
        &self.rust_profile
    }

    pub(crate) fn rust_discovery(&self) -> &SealedCueTerminatedBlockDiscoveryConfiguration {
        &self.rust_discovery
    }
}

impl EncodedNameResolver<VocabularyRoot> for FixtureRustVocabulary {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.fixed_names.get(encoded_id)
    }
}

fn validate_rust_id<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
    resolver: &Resolver,
) -> Result<(), Error> {
    if encoded_id.root_variant() != &VocabularyRoot::Rust {
        return Err(Error::NonRustVocabulary {
            position,
            found: *encoded_id.root_variant(),
        });
    }
    if resolver.resolve(encoded_id).is_none() {
        return Err(Error::MissingVocabularyName {
            position,
            encoded_id: encoded_id.clone(),
        });
    }
    Ok(())
}

fn validate_fixed_word<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
    expected: &'static str,
    resolver: &Resolver,
) -> Result<String, Error> {
    validate_rust_id(position, encoded_id, resolver)?;
    let found = resolver
        .resolve(encoded_id)
        .expect("Rust vocabulary identity was validated")
        .as_str();
    if found != expected {
        return Err(Error::VocabularySpellingMismatch {
            position,
            expected,
            found: found.to_owned(),
        });
    }
    Ok(found.to_owned())
}

fn seal_table(
    ids: &FixtureRustVocabularyIds,
    profile: &SealedTokenProfile,
) -> Result<AddressedStructuralTable<VocabularyRoot, FixtureRustRule>, Error> {
    let newtype = FixtureRustRule::Left(RustItemRecord::new(
        PARENTHESIS,
        ids.tuple_field().clone(),
        Some(1),
        ids.struct_keyword().clone(),
        ids.public_keyword().clone(),
        ids.semicolon().clone(),
        true,
    )?);
    let enumeration = FixtureRustRule::Left(RustItemRecord::new(
        BRACE,
        ids.variant().clone(),
        None,
        ids.enum_keyword().clone(),
        ids.public_keyword().clone(),
        ids.semicolon().clone(),
        false,
    )?);
    let unit_variant = FixtureRustRule::Right(RuleCoproduct::Left(UnitVariantRecord::new(
        ids.comma().clone(),
    )?));
    let tuple_variant = FixtureRustRule::Right(RuleCoproduct::Right(RuleCoproduct::Left(
        TupleVariantRecord::new(ids.tuple_field().clone(), ids.comma().clone())?,
    )));
    let tuple_field = FixtureRustRule::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Left(TupleFieldRecord::new(
            ids.type_reference().clone(),
            ids.public_keyword().clone(),
            ids.comma().clone(),
        )?),
    )));
    let identity_reference = FixtureRustRule::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Right(RuleCoproduct::Left(ReferencedTypeRecord::new()?)),
    )));
    let application_reference = FixtureRustRule::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Right(RuleCoproduct::Right(TypeApplicationRecord::new(
            ids.type_reference().clone(),
        )?)),
    )));

    let entries = vec![
        one_constructor(ids.newtype_item(), CONSTRUCTOR_IDENTITY, newtype),
        one_constructor(ids.enumeration_item(), CONSTRUCTOR_IDENTITY, enumeration),
        constructors(
            ids.variant(),
            vec![
                (CONSTRUCTOR_UNIT, unit_variant),
                (CONSTRUCTOR_TUPLE, tuple_variant),
            ],
        ),
        one_constructor(ids.tuple_field(), CONSTRUCTOR_IDENTITY, tuple_field),
        constructors(
            ids.type_reference(),
            vec![
                (CONSTRUCTOR_IDENTITY, identity_reference),
                (CONSTRUCTOR_APPLICATION, application_reference),
            ],
        ),
    ];
    Ok(AddressedStructuralTable::seal(
        TableIdentityPayload::new(
            TargetLayoutIdentity::derive(b"rust-logos fixture enum application records v2"),
            profile.identity(),
            StructuralVocabularyIdentity::language(
                b"rust-logos fixture enum application typed vocabulary v2",
            ),
            table_discovery(),
            TextualRenderingPolicy::new(vec![ContextualTextualPolicy::new(
                ROOT_CONTEXT,
                Some(WHITESPACE),
                None,
            )]),
            entries,
        ),
        profile,
    )?)
}

fn one_constructor(
    encoded_type: &EncodedTypeId<VocabularyRoot>,
    local: u16,
    rule: FixtureRustRule,
) -> StructuralEntry<VocabularyRoot, FixtureRustRule> {
    constructors(encoded_type, vec![(local, rule)])
}

fn constructors(
    encoded_type: &EncodedTypeId<VocabularyRoot>,
    rules: Vec<(u16, FixtureRustRule)>,
) -> StructuralEntry<VocabularyRoot, FixtureRustRule> {
    StructuralEntry::new(
        encoded_type.clone(),
        rules
            .into_iter()
            .map(|(local, rule)| {
                ConstructorCodec::new(
                    EncodedConstructorId::under(encoded_type, local),
                    vec![AcceptedDecodeForm::new(FORM, rule.clone())],
                    rule,
                )
            })
            .collect(),
    )
}

fn table_discovery() -> BlockTreeDiscoveryConfiguration {
    BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![BoundaryDiscoveryContext::new(
                ROOT_CONTEXT,
                TriggerSet::new(vec![PARENTHESIS, BRACE, ANGLE, WHITESPACE]),
            )],
            vec![
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, PARENTHESIS, ROOT_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE, ROOT_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, ANGLE, ROOT_CONTEXT),
            ],
        ),
        vec![],
    )
}

fn token_profile() -> Result<SealedTokenProfile, Error> {
    Ok(TokenProfileData::new(
        ProfileRevision::new(3),
        vec![
            boundary(PARENTHESIS, "(", ")"),
            boundary(BRACE, "{", "}"),
            boundary(ANGLE, "<", ">"),
            punctuation(COMMA, ","),
            punctuation(SEMICOLON, ";"),
            TriggerDefinition {
                identifier: WHITESPACE,
                trigger: Trigger::Whitespace {
                    canonical_spelling: " ".to_owned(),
                },
            },
        ],
        TriggerSet::new(vec![
            PARENTHESIS,
            BRACE,
            ANGLE,
            COMMA,
            SEMICOLON,
            WHITESPACE,
        ]),
        CharacterSet::from_text(""),
    )
    .seal()
    .map_err(raw_discovery::BlockDiscoveryError::from)?)
}

fn rust_discovery(
    struct_word: &str,
    enum_word: &str,
) -> Result<
    (
        SealedTokenProfile,
        SealedCueTerminatedBlockDiscoveryConfiguration,
    ),
    Error,
> {
    let profile = TokenProfileData::new(
        ProfileRevision::new(4),
        vec![
            boundary(PARENTHESIS, "(", ")"),
            boundary(BRACE, "{", "}"),
            boundary(ANGLE, "<", ">"),
            boundary(SQUARE, "[", "]"),
            punctuation(COMMA, ","),
            punctuation(SEMICOLON, ";"),
            TriggerDefinition {
                identifier: STRING,
                trigger: Trigger::Carrier {
                    opening: "\"".to_owned(),
                    closing: "\"".to_owned(),
                    escape: Some("\\".to_owned()),
                },
            },
            TriggerDefinition {
                identifier: LINE_COMMENT,
                trigger: Trigger::LineComment {
                    opening: "//".to_owned(),
                },
            },
            TriggerDefinition {
                identifier: BLOCK_COMMENT,
                trigger: Trigger::Carrier {
                    opening: "/*".to_owned(),
                    closing: "*/".to_owned(),
                    escape: None,
                },
            },
            TriggerDefinition {
                identifier: WHITESPACE,
                trigger: Trigger::Whitespace {
                    canonical_spelling: " ".to_owned(),
                },
            },
        ],
        TriggerSet::new(vec![
            PARENTHESIS,
            BRACE,
            ANGLE,
            SQUARE,
            COMMA,
            SEMICOLON,
            STRING,
            LINE_COMMENT,
            BLOCK_COMMENT,
            WHITESPACE,
        ]),
        CharacterSet::from_text(""),
    )
    .seal()
    .map_err(raw_discovery::BlockDiscoveryError::from)?;
    let boundaries = BoundaryDiscoveryConfiguration::new(
        ROOT_CONTEXT,
        vec![BoundaryDiscoveryContext::new(
            ROOT_CONTEXT,
            TriggerSet::new(vec![
                PARENTHESIS,
                BRACE,
                ANGLE,
                SQUARE,
                STRING,
                LINE_COMMENT,
                BLOCK_COMMENT,
                WHITESPACE,
            ]),
        )],
        vec![
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, PARENTHESIS, ROOT_CONTEXT),
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE, ROOT_CONTEXT),
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, ANGLE, ROOT_CONTEXT),
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, SQUARE, ROOT_CONTEXT),
        ],
    );
    let configuration = CueTerminatedBlockDiscoveryConfiguration::new(
        boundaries,
        vec![
            CueTerminationRule::new(STRUCT_CUE, struct_word, ";", rust_word_characters()),
            CueTerminationRule::through_boundary(
                ENUM_CUE,
                enum_word,
                BRACE,
                rust_word_characters(),
            ),
        ],
    )
    .seal(&profile)?;
    Ok((profile, configuration))
}

fn boundary(identifier: TriggerIdentifier, opening: &str, closing: &str) -> TriggerDefinition {
    TriggerDefinition {
        identifier,
        trigger: Trigger::Boundary {
            opening: opening.to_owned(),
            closing: closing.to_owned(),
        },
    }
}

fn punctuation(identifier: TriggerIdentifier, glyph: &str) -> TriggerDefinition {
    TriggerDefinition {
        identifier,
        trigger: Trigger::Punctuation {
            glyph: glyph.to_owned(),
        },
    }
}

fn rust_word_characters() -> CharacterClass {
    CharacterClass::Characters(CharacterSet::new(
        ('a'..='z').chain('A'..='Z').chain('0'..='9').chain(['_']),
    ))
}

pub(crate) fn constructor_for(
    encoded_type: &EncodedTypeId<VocabularyRoot>,
    local: u16,
) -> EncodedConstructorId<VocabularyRoot> {
    EncodedConstructorId::under(encoded_type, local)
}

pub(crate) fn ordered_sequence_value() -> FieldValue<VocabularyRoot> {
    FieldValue::OrderedProduct
}
