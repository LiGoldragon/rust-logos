//! Fully typed Slice One Rust position records.
//!
//! The records contain no parser or emitter algorithm. They are archived data
//! consumed by `structural-codec`'s one evaluator and conservative
//! disjointness prover.

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
    AcceptedDecodeForm, AddressedStructuralTable, AtomDescriptor, ConstructorCodec,
    ContextualTextualPolicy, DecodeFormId, EncodedConstructorId, EncodedNameResolver,
    EncodedTypeId, FieldEnd, FieldLink, FieldRole, Position, RuleCoproduct, SharedDescriptor,
    StableRoleId, StructuralEntry, StructuralVocabularyIdentity, StructureRecord,
    TableIdentityPayload, TargetLayoutIdentity, TextualRenderingPolicy,
};

use crate::Error;

pub(crate) const PARENTHESIS: TriggerIdentifier = TriggerIdentifier::new(1);
const SQUARE: TriggerIdentifier = TriggerIdentifier::new(2);
const BRACE: TriggerIdentifier = TriggerIdentifier::new(3);
const STRING: TriggerIdentifier = TriggerIdentifier::new(4);
const LINE_COMMENT: TriggerIdentifier = TriggerIdentifier::new(5);
const BLOCK_COMMENT: TriggerIdentifier = TriggerIdentifier::new(6);
const WHITESPACE: TriggerIdentifier = TriggerIdentifier::new(7);
pub(crate) const STRUCT_CUE: CueTerminationRuleIdentifier = CueTerminationRuleIdentifier::new(1);
const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier = BoundaryDiscoveryContextIdentifier::new(1);
const CONSTRUCTOR_LOCAL: u16 = 1;
const FORM: DecodeFormId = DecodeFormId::new(1);

macro_rules! rust_role {
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

rust_role!(StructKeywordPosition, 20_001);
rust_role!(PublicKeywordPosition, 20_002);
rust_role!(DeclarationNamePosition, 20_003);
rust_role!(ReferencedTypePosition, 20_004);

macro_rules! one_position_rule {
    ($record:ident, $role:ident, $field:ident) => {
        #[derive(
            rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq,
        )]
        pub struct $record {
            $field: Position<$role, VocabularyRoot>,
        }

        impl $record {
            /// Author the typed position with shared descriptor data.
            pub fn try_new(descriptor: SharedDescriptor<VocabularyRoot>) -> Result<Self, Error> {
                Ok(Self {
                    $field: Position::try_new(descriptor)?,
                })
            }

            /// The actual typed position carried by this record.
            pub fn position(&self) -> &Position<$role, VocabularyRoot> {
                &self.$field
            }
        }

        impl StructureRecord<VocabularyRoot> for $record {
            type View<'record> = FieldLink<'record, $role, VocabularyRoot, FieldEnd>;

            fn root_role(&self) -> StableRoleId {
                self.$field.role()
            }

            fn fields(&self) -> Self::View<'_> {
                FieldLink::new(&self.$field, FieldEnd)
            }
        }
    };
}

one_position_rule!(StructKeywordRule, StructKeywordPosition, struct_keyword);
one_position_rule!(PublicKeywordRule, PublicKeywordPosition, public_keyword);
one_position_rule!(
    DeclarationNameRule,
    DeclarationNamePosition,
    declaration_name
);
one_position_rule!(ReferencedTypeRule, ReferencedTypePosition, referenced_type);

/// The closed typed rule vocabulary for the attribute-free newtype slice.
pub type RustNewtypeRule = RuleCoproduct<
    StructKeywordRule,
    RuleCoproduct<PublicKeywordRule, RuleCoproduct<DeclarationNameRule, ReferencedTypeRule>>,
>;

/// Translator-issued Rust-root identities required to seat the typed rules.
///
/// This carrier does not allocate them. A trusted Rust-vocabulary release must
/// already have made every identity resolvable.
#[derive(Clone, Debug)]
pub struct RustNewtypeVocabularyIds {
    struct_keyword_type: EncodedTypeId<VocabularyRoot>,
    public_keyword_type: EncodedTypeId<VocabularyRoot>,
    declaration_name_type: EncodedTypeId<VocabularyRoot>,
    referenced_type_type: EncodedTypeId<VocabularyRoot>,
    struct_keyword: VocabularyEncodedId,
    public_keyword: VocabularyEncodedId,
}

impl RustNewtypeVocabularyIds {
    /// Carry the complete Rust-root chains needed by the structural table.
    pub fn new(
        struct_keyword_type: VocabularyEncodedId,
        public_keyword_type: VocabularyEncodedId,
        declaration_name_type: VocabularyEncodedId,
        referenced_type_type: VocabularyEncodedId,
        struct_keyword: VocabularyEncodedId,
        public_keyword: VocabularyEncodedId,
    ) -> Self {
        Self {
            struct_keyword_type: EncodedTypeId::new(struct_keyword_type),
            public_keyword_type: EncodedTypeId::new(public_keyword_type),
            declaration_name_type: EncodedTypeId::new(declaration_name_type),
            referenced_type_type: EncodedTypeId::new(referenced_type_type),
            struct_keyword,
            public_keyword,
        }
    }

    /// The expected type for the fixed `struct` token.
    pub fn struct_keyword_type(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.struct_keyword_type
    }

    /// The expected type for the fixed `pub` token.
    pub fn public_keyword_type(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.public_keyword_type
    }

    /// The expected type for a declaration token.
    pub fn declaration_name_type(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.declaration_name_type
    }

    /// The expected type for a reference token.
    pub fn referenced_type_type(&self) -> &EncodedTypeId<VocabularyRoot> {
        &self.referenced_type_type
    }

    /// The immutable Rust-root identity of `struct`.
    pub fn struct_keyword(&self) -> &VocabularyEncodedId {
        &self.struct_keyword
    }

    /// The immutable Rust-root identity of `pub`.
    pub fn public_keyword(&self) -> &VocabularyEncodedId {
        &self.public_keyword
    }
}

/// Sealed typed rule data and pass-one Rust boundary data.
pub struct RustNewtypeVocabulary {
    ids: RustNewtypeVocabularyIds,
    table: AddressedStructuralTable<VocabularyRoot, RustNewtypeRule>,
    rust_profile: SealedTokenProfile,
    rust_discovery: SealedCueTerminatedBlockDiscoveryConfiguration,
    fixed_names: BTreeMap<VocabularyEncodedId, Name>,
}

impl RustNewtypeVocabulary {
    /// Validate the immutable Rust vocabulary, seal the shared-evaluator table,
    /// and seal the cue-to-termination pass-one rules.
    pub fn seal<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
        ids: RustNewtypeVocabularyIds,
        rust_names: &Resolver,
    ) -> Result<Self, Error> {
        validate_rust_id(
            "struct keyword type",
            ids.struct_keyword_type().encoded_id(),
            rust_names,
        )?;
        validate_rust_id(
            "public keyword type",
            ids.public_keyword_type().encoded_id(),
            rust_names,
        )?;
        validate_rust_id(
            "declaration-name type",
            ids.declaration_name_type().encoded_id(),
            rust_names,
        )?;
        validate_rust_id(
            "referenced-type type",
            ids.referenced_type_type().encoded_id(),
            rust_names,
        )?;
        let struct_word =
            validate_fixed_word("struct keyword", ids.struct_keyword(), "struct", rust_names)?;
        validate_fixed_word("public keyword", ids.public_keyword(), "pub", rust_names)?;

        let token_profile = token_profile()?;
        let table = seal_table(&ids, &token_profile)?;
        let (rust_profile, rust_discovery) = rust_discovery(&struct_word)?;
        let fixed_names = [
            (
                ids.struct_keyword().clone(),
                rust_names
                    .resolve(ids.struct_keyword())
                    .expect("fixed struct word was validated")
                    .clone(),
            ),
            (
                ids.public_keyword().clone(),
                rust_names
                    .resolve(ids.public_keyword())
                    .expect("fixed public word was validated")
                    .clone(),
            ),
        ]
        .into_iter()
        .collect();

        Ok(Self {
            ids,
            table,
            rust_profile,
            rust_discovery,
            fixed_names,
        })
    }

    /// The shared-evaluator structural table.
    pub fn structuretree(&self) -> &AddressedStructuralTable<VocabularyRoot, RustNewtypeRule> {
        &self.table
    }

    /// The caller-issued typed identities used by the table.
    pub fn ids(&self) -> &RustNewtypeVocabularyIds {
        &self.ids
    }

    pub(crate) fn rust_profile(&self) -> &SealedTokenProfile {
        &self.rust_profile
    }

    pub(crate) fn rust_discovery(&self) -> &SealedCueTerminatedBlockDiscoveryConfiguration {
        &self.rust_discovery
    }

    pub(crate) fn item_separator(&self) -> &str {
        let Trigger::Whitespace { canonical_spelling } = &self
            .table
            .token_profile()
            .definition(WHITESPACE)
            .expect("the sealed table retains its validated whitespace position")
            .trigger
        else {
            unreachable!("the table seal validated the textual separator as whitespace")
        };
        canonical_spelling
    }

    pub(crate) fn tuple_delimiters(&self) -> (&str, &str) {
        let Trigger::Boundary { opening, closing } = &self
            .rust_profile
            .definition(PARENTHESIS)
            .expect("the sealed Rust profile retains its tuple boundary")
            .trigger
        else {
            unreachable!("the cue discovery seal validated the tuple boundary")
        };
        (opening, closing)
    }

    pub(crate) fn item_termination(&self) -> &str {
        self.rust_discovery
            .rules()
            .iter()
            .find(|rule| rule.identifier() == STRUCT_CUE)
            .expect("the sealed Rust discovery retains its struct rule")
            .termination()
    }
}

impl EncodedNameResolver<VocabularyRoot> for RustNewtypeVocabulary {
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
        .expect("validate_rust_id established a resolved word")
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
    ids: &RustNewtypeVocabularyIds,
    profile: &SealedTokenProfile,
) -> Result<AddressedStructuralTable<VocabularyRoot, RustNewtypeRule>, Error> {
    let struct_rule = RustNewtypeRule::Left(StructKeywordRule::try_new(
        SharedDescriptor::Literal(ids.struct_keyword().clone()),
    )?);
    let public_rule = RustNewtypeRule::Right(RuleCoproduct::Left(PublicKeywordRule::try_new(
        SharedDescriptor::Literal(ids.public_keyword().clone()),
    )?));
    let declaration_rule = RustNewtypeRule::Right(RuleCoproduct::Right(RuleCoproduct::Left(
        DeclarationNameRule::try_new(SharedDescriptor::Declaration(AtomDescriptor::any_case()))?,
    )));
    let reference_rule = RustNewtypeRule::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        ReferencedTypeRule::try_new(SharedDescriptor::Reference(AtomDescriptor::any_case()))?,
    )));

    let entries = vec![
        entry(ids.struct_keyword_type(), struct_rule),
        entry(ids.public_keyword_type(), public_rule),
        entry(ids.declaration_name_type(), declaration_rule),
        entry(ids.referenced_type_type(), reference_rule),
    ];
    let block_discovery = BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![BoundaryDiscoveryContext::new(
                ROOT_CONTEXT,
                TriggerSet::new(vec![WHITESPACE]),
            )],
            vec![],
        ),
        vec![],
    );
    let payload = TableIdentityPayload::new(
        TargetLayoutIdentity::derive(b"rust-logos attribute-free newtype positions v1"),
        profile.identity(),
        StructuralVocabularyIdentity::language(
            b"rust-logos fully typed attribute-free newtype vocabulary v1",
        ),
        block_discovery,
        TextualRenderingPolicy::new(vec![ContextualTextualPolicy::new(
            ROOT_CONTEXT,
            Some(WHITESPACE),
            None,
        )]),
        entries,
    );
    Ok(AddressedStructuralTable::seal(payload, profile)?)
}

fn entry(
    encoded_type: &EncodedTypeId<VocabularyRoot>,
    rule: RustNewtypeRule,
) -> StructuralEntry<VocabularyRoot, RustNewtypeRule> {
    let constructor = EncodedConstructorId::under(encoded_type, CONSTRUCTOR_LOCAL);
    StructuralEntry::new(
        encoded_type.clone(),
        vec![ConstructorCodec::new(
            constructor,
            vec![AcceptedDecodeForm::new(FORM, rule.clone())],
            rule,
        )],
    )
}

fn token_profile() -> Result<SealedTokenProfile, Error> {
    Ok(TokenProfileData::new(
        ProfileRevision::new(1),
        vec![TriggerDefinition {
            identifier: WHITESPACE,
            trigger: Trigger::Whitespace {
                canonical_spelling: " ".to_owned(),
            },
        }],
        TriggerSet::new(vec![WHITESPACE]),
        CharacterSet::from_text("(){}[];,"),
    )
    .seal()
    .map_err(raw_discovery::BlockDiscoveryError::from)?)
}

fn rust_discovery(
    struct_word: &str,
) -> Result<
    (
        SealedTokenProfile,
        SealedCueTerminatedBlockDiscoveryConfiguration,
    ),
    Error,
> {
    let profile = TokenProfileData::new(
        ProfileRevision::new(2),
        vec![
            TriggerDefinition {
                identifier: PARENTHESIS,
                trigger: Trigger::Boundary {
                    opening: "(".to_owned(),
                    closing: ")".to_owned(),
                },
            },
            TriggerDefinition {
                identifier: SQUARE,
                trigger: Trigger::Boundary {
                    opening: "[".to_owned(),
                    closing: "]".to_owned(),
                },
            },
            TriggerDefinition {
                identifier: BRACE,
                trigger: Trigger::Boundary {
                    opening: "{".to_owned(),
                    closing: "}".to_owned(),
                },
            },
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
            SQUARE,
            BRACE,
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
                SQUARE,
                BRACE,
                STRING,
                LINE_COMMENT,
                BLOCK_COMMENT,
                WHITESPACE,
            ]),
        )],
        vec![
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, PARENTHESIS, ROOT_CONTEXT),
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, SQUARE, ROOT_CONTEXT),
            BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE, ROOT_CONTEXT),
        ],
    );
    let configuration = CueTerminatedBlockDiscoveryConfiguration::new(
        boundaries,
        vec![CueTerminationRule::new(
            STRUCT_CUE,
            struct_word,
            ";",
            rust_word_characters(),
        )],
    )
    .seal(&profile)?;
    Ok((profile, configuration))
}

fn rust_word_characters() -> CharacterClass {
    CharacterClass::Characters(CharacterSet::new(
        ('a'..='z').chain('A'..='Z').chain('0'..='9').chain(['_']),
    ))
}

pub(crate) fn constructor_for(
    encoded_type: &EncodedTypeId<VocabularyRoot>,
) -> EncodedConstructorId<VocabularyRoot> {
    EncodedConstructorId::under(encoded_type, CONSTRUCTOR_LOCAL)
}

#[cfg(test)]
mod tests {
    use name_table::LocalEncodedId;
    use structural_codec::{DisjointnessError, TableError};

    use super::*;

    fn encoded(chain: &[u16]) -> VocabularyEncodedId {
        VocabularyEncodedId::new(
            VocabularyRoot::Rust,
            chain.iter().copied().map(LocalEncodedId::new).collect(),
        )
        .expect("fixture chain")
    }

    #[test]
    fn declaration_and_reference_alternatives_are_refused_as_overlapping() {
        let profile = token_profile().expect("token profile");
        let encoded_type = EncodedTypeId::new(encoded(&[90]));
        let declaration = RustNewtypeRule::Right(RuleCoproduct::Right(RuleCoproduct::Left(
            DeclarationNameRule::try_new(SharedDescriptor::Declaration(AtomDescriptor::any_case()))
                .expect("declaration record"),
        )));
        let reference = RustNewtypeRule::Right(RuleCoproduct::Right(RuleCoproduct::Right(
            ReferencedTypeRule::try_new(SharedDescriptor::Reference(AtomDescriptor::any_case()))
                .expect("reference record"),
        )));
        let entry = StructuralEntry::new(
            encoded_type.clone(),
            vec![
                ConstructorCodec::new(
                    EncodedConstructorId::under(&encoded_type, 1),
                    vec![AcceptedDecodeForm::new(
                        DecodeFormId::new(1),
                        declaration.clone(),
                    )],
                    declaration,
                ),
                ConstructorCodec::new(
                    EncodedConstructorId::under(&encoded_type, 2),
                    vec![AcceptedDecodeForm::new(
                        DecodeFormId::new(1),
                        reference.clone(),
                    )],
                    reference,
                ),
            ],
        );
        let block_discovery = BlockTreeDiscoveryConfiguration::new(
            BoundaryDiscoveryConfiguration::new(
                ROOT_CONTEXT,
                vec![BoundaryDiscoveryContext::new(
                    ROOT_CONTEXT,
                    TriggerSet::new(vec![WHITESPACE]),
                )],
                vec![],
            ),
            vec![],
        );
        let payload = TableIdentityPayload::new(
            TargetLayoutIdentity::derive(b"overlap refusal witness"),
            profile.identity(),
            StructuralVocabularyIdentity::language(b"overlap refusal witness"),
            block_discovery,
            TextualRenderingPolicy::new(vec![ContextualTextualPolicy::new(
                ROOT_CONTEXT,
                Some(WHITESPACE),
                None,
            )]),
            vec![entry],
        );

        assert!(matches!(
            AddressedStructuralTable::seal(payload, &profile),
            Err(TableError::Disjointness(
                DisjointnessError::NotProvablyDisjoint { .. }
            ))
        ));
    }
}
