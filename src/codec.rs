//! Rust TextualForm orchestration over complete typed fixture records.

use std::collections::BTreeMap;

use core_logos::{
    WholeLogos, WholeLogosEnumeration, WholeLogosItem, WholeLogosNewtype, WholeLogosTupleFields,
    WholeLogosTypeApplication, WholeLogosTypeReference, WholeLogosVariant,
    WholeLogosVariantPayload, WholeLogosVisibility,
};
use name_table::Name;
use raw_discovery::{
    BlockDiscoveryError, BlockTree, CueTerminatedBlockCueEvidence,
    DiscoveredCueTerminatedBlockTree, SourceBound,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, FieldRole,
    FieldValue, NameOccurrence, ResolvedReference, StructuralEvaluator, StructuralValue,
};

use crate::fixture_vocabulary::{
    ApplicationBody, ApplicationHead, ApplicationPayload, ApplicationRoot, ENUM_CUE,
    FixtureRustRule, FixtureRustVocabulary, ItemBody, ItemElements, ItemKeyword, ItemName,
    ItemRoot, ItemTerminator, ItemVisibility, ReferencedTypePosition, STRUCT_CUE, TupleFieldRoot,
    TupleFieldTerminator, TupleFieldType, TupleFieldVisibility, VariantBody, VariantFields,
    VariantName, VariantRoot, VariantTerminator, constructor_for, ordered_sequence_value,
};
use crate::{Error, FixtureRustNameProjectionTable, RustEncodedIdCodec};

/// Bidirectional Rust view for the bounded fixture breadth.
pub struct RustLogos {
    vocabulary: FixtureRustVocabulary,
}

impl RustLogos {
    /// Seat the already-sealed typed Rust fixture vocabulary.
    pub fn new(vocabulary: FixtureRustVocabulary) -> Self {
        Self { vocabulary }
    }

    /// Exact typed structural data used for fixture evaluation.
    pub fn vocabulary(&self) -> &FixtureRustVocabulary {
        &self.vocabulary
    }

    /// Decode complete fixture items after cue-to-termination discovery.
    pub fn decode_fixture<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bindings: &Bindings,
    ) -> Result<WholeLogos, Error> {
        let tree = DiscoveredCueTerminatedBlockTree::discover(
            source,
            self.vocabulary.rust_profile(),
            self.vocabulary.rust_discovery(),
        )?;
        if tree.root_blocks().is_empty() {
            return Err(Error::NoRustItems);
        }
        let evaluator = StructuralEvaluator::<VocabularyRoot, FixtureRustRule>::new(
            self.vocabulary.structuretree(),
        )?;
        let mut items = Vec::with_capacity(tree.root_blocks().len());
        let mut previous_end = 0;
        for block in tree.root_blocks() {
            let start = trim_bound(
                source,
                checked_bound(source, previous_end, block.cue().bound().start())?,
            )?
            .start();
            let bound = checked_bound(source, start, block.source_bound().end())?;
            let expected = match block.cue().evidence() {
                CueTerminatedBlockCueEvidence::CueTermination(STRUCT_CUE) => {
                    self.vocabulary.ids().newtype_item()
                }
                CueTerminatedBlockCueEvidence::CueTermination(ENUM_CUE) => {
                    self.vocabulary.ids().enumeration_item()
                }
                CueTerminatedBlockCueEvidence::CueTermination(_) => {
                    return Err(Error::UnsupportedItemShape {
                        bound: block.source_bound(),
                    });
                }
                CueTerminatedBlockCueEvidence::Boundary(_) => {
                    return Err(Error::UnsupportedItemShape {
                        bound: block.source_bound(),
                    });
                }
            };
            let offset = OffsetBindings {
                vocabulary: &self.vocabulary,
                inner: bindings,
                source,
                offset: bound.start(),
            };
            let value = evaluator
                .decode_text(expected, &source[bound.start()..bound.end()], &offset)
                .map_err(|error| {
                    Error::Decode(offset_decode_error(error, source, bound.start()))
                })?;
            if expected == self.vocabulary.ids().newtype_item() {
                items.push(WholeLogosItem::Newtype(self.reify_newtype(&value)?));
            } else {
                items.push(WholeLogosItem::Enumeration(self.reify_enumeration(&value)?));
            }
            previous_end = block.source_bound().end();
        }
        let trailing = trim_bound(source, checked_bound(source, previous_end, source.len())?)?;
        if !trailing.is_empty() {
            return Err(Error::TrailingSource { bound: trailing });
        }
        Ok(WholeLogos::new(items))
    }

    /// Emit each fixture item through its complete typed structural record.
    ///
    /// Caller projections are test data only. No chain spelling is derived.
    pub fn emit_fixture(
        &self,
        logos: &WholeLogos,
        projections: &FixtureRustNameProjectionTable,
    ) -> Result<String, Error> {
        self.validate_projections(logos, projections)?;
        let resolver = FixtureResolver {
            vocabulary: &self.vocabulary,
            projections,
        };
        self.emit_with_resolver(logos, &resolver)
    }

    /// Emit production Rust names directly from complete encoded-ID chains.
    ///
    /// `allocated` is the owning naming boundary's verified current view.
    /// Universal identities are encoded without reading their spelling.
    /// Rust-root identities keep the immutable spelling resolved by that view.
    /// Every identity is validated and every generated name is prepared before
    /// structural emission begins.
    pub fn emit<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        logos: &WholeLogos,
        allocated: &Allocated,
    ) -> Result<String, Error> {
        let generated = generated_names(logos, allocated)?;
        let resolver = ProductionResolver {
            vocabulary: &self.vocabulary,
            allocated,
            generated,
        };
        self.emit_with_resolver(logos, &resolver)
    }

    fn emit_with_resolver<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        logos: &WholeLogos,
        resolver: &Resolver,
    ) -> Result<String, Error> {
        let evaluator = StructuralEvaluator::<VocabularyRoot, FixtureRustRule>::new(
            self.vocabulary.structuretree(),
        )?;
        let mut items = Vec::with_capacity(logos.items().len());
        for item in logos.items() {
            let (expected, value) = match item {
                WholeLogosItem::Newtype(newtype) => (
                    self.vocabulary.ids().newtype_item(),
                    self.reflect_newtype(newtype)?,
                ),
                WholeLogosItem::Enumeration(enumeration) => (
                    self.vocabulary.ids().enumeration_item(),
                    self.reflect_enumeration(enumeration)?,
                ),
            };
            items.push(evaluator.encode_text(expected, &value, resolver)?);
        }
        Ok(RenderedFixtureDocument(items).to_string())
    }

    fn validate_projections(
        &self,
        logos: &WholeLogos,
        projections: &FixtureRustNameProjectionTable,
    ) -> Result<(), Error> {
        for item in logos.items() {
            match item {
                WholeLogosItem::Newtype(newtype) => {
                    require_projection("newtype name", newtype.name(), projections)?;
                    self.validate_reference(newtype.wrapped(), projections)?;
                }
                WholeLogosItem::Enumeration(enumeration) => {
                    require_projection("enumeration name", enumeration.name(), projections)?;
                    for variant in enumeration.variants() {
                        require_projection("variant name", variant.name(), projections)?;
                        if let WholeLogosVariantPayload::Tuple(fields) = variant.payload() {
                            for field in fields.fields() {
                                self.validate_reference(field, projections)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_reference(
        &self,
        reference: &WholeLogosTypeReference,
        projections: &FixtureRustNameProjectionTable,
    ) -> Result<(), Error> {
        let _ = self;
        match reference {
            WholeLogosTypeReference::Identity(encoded_id) => {
                require_projection("type reference", encoded_id, projections)
            }
            WholeLogosTypeReference::Application(application) => {
                require_projection("application head", application.head(), projections)?;
                self.validate_reference(application.payload(), projections)
            }
        }
    }

    fn reify_newtype(
        &self,
        value: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeLogosNewtype, Error> {
        let fields = repeated::<ItemElements>(value, "newtype fields")?;
        let [FieldValue::Delegated(field)] = fields else {
            return Err(Error::TypedPositionMismatch {
                position: "one newtype field",
            });
        };
        let (wrapped_visibility, wrapped) = self.reify_tuple_field(field)?;
        Ok(WholeLogosNewtype::new(
            reify_visibility::<ItemVisibility>(value)?,
            declaration_id::<ItemName>(value, "newtype name")?,
            wrapped_visibility,
            wrapped,
        ))
    }

    fn reify_enumeration(
        &self,
        value: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeLogosEnumeration, Error> {
        let encoded_variants = repeated::<ItemElements>(value, "enumeration variants")?;
        let mut variants = Vec::with_capacity(encoded_variants.len());
        for variant in encoded_variants {
            let FieldValue::Delegated(variant) = variant else {
                return Err(Error::TypedPositionMismatch {
                    position: "enumeration variant",
                });
            };
            variants.push(self.reify_variant(variant)?);
        }
        Ok(WholeLogosEnumeration::new(
            reify_visibility::<ItemVisibility>(value)?,
            declaration_id::<ItemName>(value, "enumeration name")?,
            variants,
        ))
    }

    fn reify_variant(
        &self,
        value: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeLogosVariant, Error> {
        let name = declaration_id::<VariantName>(value, "variant name")?;
        if value.constructor() == &constructor_for(self.vocabulary.ids().variant(), 1) {
            return Ok(WholeLogosVariant::new(name, WholeLogosVariantPayload::Unit));
        }
        if value.constructor() == &constructor_for(self.vocabulary.ids().variant(), 2) {
            let encoded_fields = repeated::<VariantFields>(value, "variant fields")?;
            let mut fields = Vec::with_capacity(encoded_fields.len());
            for field in encoded_fields {
                let FieldValue::Delegated(field) = field else {
                    return Err(Error::TypedPositionMismatch {
                        position: "variant tuple field",
                    });
                };
                let (visibility, reference) = self.reify_tuple_field(field)?;
                if visibility != WholeLogosVisibility::Private {
                    return Err(Error::UnsupportedVariantFieldVisibility);
                }
                fields.push(reference);
            }
            let fields =
                WholeLogosTupleFields::new(fields).map_err(|_| Error::TypedPositionMismatch {
                    position: "non-empty variant fields",
                })?;
            return Ok(WholeLogosVariant::new(
                name,
                WholeLogosVariantPayload::Tuple(fields),
            ));
        }
        Err(Error::TypedPositionMismatch {
            position: "variant constructor",
        })
    }

    fn reify_tuple_field(
        &self,
        value: &StructuralValue<VocabularyRoot>,
    ) -> Result<(WholeLogosVisibility, WholeLogosTypeReference), Error> {
        let reference = delegated::<TupleFieldType>(value, "tuple field type")?;
        Ok((
            reify_visibility::<TupleFieldVisibility>(value)?,
            self.reify_reference(reference)?,
        ))
    }

    fn reify_reference(
        &self,
        value: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeLogosTypeReference, Error> {
        if value.constructor() == &constructor_for(self.vocabulary.ids().type_reference(), 1) {
            return Ok(WholeLogosTypeReference::Identity(reference_id::<
                ReferencedTypePosition,
            >(
                value,
                "type reference",
            )?));
        }
        if value.constructor() == &constructor_for(self.vocabulary.ids().type_reference(), 2) {
            let head = reference_id::<ApplicationHead>(value, "application head")?;
            let [FieldValue::Delegated(payload)] =
                repeated::<ApplicationPayload>(value, "application payload")?
            else {
                return Err(Error::TypedPositionMismatch {
                    position: "one application payload",
                });
            };
            return Ok(WholeLogosTypeReference::Application(
                WholeLogosTypeApplication::new(head, self.reify_reference(payload)?),
            ));
        }
        Err(Error::TypedPositionMismatch {
            position: "type-reference constructor",
        })
    }

    fn reflect_newtype(
        &self,
        newtype: &WholeLogosNewtype,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        let field = self.reflect_tuple_field(newtype.wrapped_visibility(), newtype.wrapped())?;
        self.reflect_item(
            self.vocabulary.ids().newtype_item(),
            newtype.visibility(),
            self.vocabulary.ids().struct_keyword(),
            newtype.name(),
            vec![field],
            true,
        )
    }

    fn reflect_enumeration(
        &self,
        enumeration: &WholeLogosEnumeration,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        let variants = enumeration
            .variants()
            .iter()
            .map(|variant| self.reflect_variant(variant))
            .collect::<Result<Vec<_>, _>>()?;
        self.reflect_item(
            self.vocabulary.ids().enumeration_item(),
            enumeration.visibility(),
            self.vocabulary.ids().enum_keyword(),
            enumeration.name(),
            variants,
            false,
        )
    }

    fn reflect_item(
        &self,
        expected: &structural_codec::EncodedTypeId<VocabularyRoot>,
        visibility: &WholeLogosVisibility,
        keyword: &VocabularyEncodedId,
        name: &VocabularyEncodedId,
        elements: Vec<StructuralValue<VocabularyRoot>>,
        terminated: bool,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        let elements = FieldValue::Repeated(
            elements
                .into_iter()
                .map(|value| FieldValue::Delegated(Box::new(value)))
                .collect(),
        );
        let mut record = StructuralValue::record(constructor_for(expected, 1));
        record.insert::<ItemRoot>(ordered_sequence_value())?;
        record.insert::<ItemVisibility>(reflect_visibility(
            visibility,
            self.vocabulary.ids().public_keyword(),
        ))?;
        record.insert::<ItemKeyword>(FieldValue::Literal(keyword.clone()))?;
        record.insert::<ItemName>(FieldValue::Declaration(DeclarationAssignment::new(
            name.clone(),
        )))?;
        record.insert::<ItemBody>(FieldValue::Delimited(Box::new(elements.clone())))?;
        record.insert::<ItemElements>(elements)?;
        if terminated {
            record.insert::<ItemTerminator>(FieldValue::Literal(
                self.vocabulary.ids().semicolon().clone(),
            ))?;
        } else {
            record.insert::<ItemTerminator>(FieldValue::Repeated(Vec::new()))?;
        }
        Ok(record.finish())
    }

    fn reflect_variant(
        &self,
        variant: &WholeLogosVariant,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        match variant.payload() {
            WholeLogosVariantPayload::Unit => {
                let mut record =
                    StructuralValue::record(constructor_for(self.vocabulary.ids().variant(), 1));
                record.insert::<VariantRoot>(ordered_sequence_value())?;
                record.insert::<VariantName>(FieldValue::Declaration(
                    DeclarationAssignment::new(variant.name().clone()),
                ))?;
                record.insert::<VariantTerminator>(FieldValue::Literal(
                    self.vocabulary.ids().comma().clone(),
                ))?;
                Ok(record.finish())
            }
            WholeLogosVariantPayload::Tuple(fields) => {
                let fields = FieldValue::Repeated(
                    fields
                        .fields()
                        .iter()
                        .map(|field| {
                            self.reflect_tuple_field(&WholeLogosVisibility::Private, field)
                                .map(|value| FieldValue::Delegated(Box::new(value)))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
                let mut record =
                    StructuralValue::record(constructor_for(self.vocabulary.ids().variant(), 2));
                record.insert::<VariantRoot>(ordered_sequence_value())?;
                record.insert::<VariantName>(FieldValue::Declaration(
                    DeclarationAssignment::new(variant.name().clone()),
                ))?;
                record.insert::<VariantBody>(FieldValue::Delimited(Box::new(fields.clone())))?;
                record.insert::<VariantFields>(fields)?;
                record.insert::<VariantTerminator>(FieldValue::Literal(
                    self.vocabulary.ids().comma().clone(),
                ))?;
                Ok(record.finish())
            }
        }
    }

    fn reflect_tuple_field(
        &self,
        visibility: &WholeLogosVisibility,
        reference: &WholeLogosTypeReference,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        let reference = FieldValue::Delegated(Box::new(self.reflect_reference(reference)?));
        let mut record =
            StructuralValue::record(constructor_for(self.vocabulary.ids().tuple_field(), 1));
        record.insert::<TupleFieldRoot>(ordered_sequence_value())?;
        record.insert::<TupleFieldVisibility>(reflect_visibility(
            visibility,
            self.vocabulary.ids().public_keyword(),
        ))?;
        record.insert::<TupleFieldType>(reference)?;
        record.insert::<TupleFieldTerminator>(FieldValue::Literal(
            self.vocabulary.ids().comma().clone(),
        ))?;
        Ok(record.finish())
    }

    fn reflect_reference(
        &self,
        reference: &WholeLogosTypeReference,
    ) -> Result<StructuralValue<VocabularyRoot>, Error> {
        match reference {
            WholeLogosTypeReference::Identity(encoded_id) => {
                let mut record = StructuralValue::record(constructor_for(
                    self.vocabulary.ids().type_reference(),
                    1,
                ));
                record.insert::<ReferencedTypePosition>(FieldValue::Reference(
                    ResolvedReference::new(encoded_id.clone()),
                ))?;
                Ok(record.finish())
            }
            WholeLogosTypeReference::Application(application) => {
                let payload = FieldValue::Repeated(vec![FieldValue::Delegated(Box::new(
                    self.reflect_reference(application.payload())?,
                ))]);
                let mut record = StructuralValue::record(constructor_for(
                    self.vocabulary.ids().type_reference(),
                    2,
                ));
                record.insert::<ApplicationRoot>(ordered_sequence_value())?;
                record.insert::<ApplicationHead>(FieldValue::Reference(ResolvedReference::new(
                    application.head().clone(),
                )))?;
                record
                    .insert::<ApplicationBody>(FieldValue::Delimited(Box::new(payload.clone())))?;
                record.insert::<ApplicationPayload>(payload)?;
                Ok(record.finish())
            }
        }
    }
}

fn reflect_visibility(
    visibility: &WholeLogosVisibility,
    public_keyword: &VocabularyEncodedId,
) -> FieldValue<VocabularyRoot> {
    match visibility {
        WholeLogosVisibility::Public => {
            FieldValue::Repeated(vec![FieldValue::Literal(public_keyword.clone())])
        }
        WholeLogosVisibility::Private => FieldValue::Repeated(Vec::new()),
    }
}

fn reify_visibility<Role: FieldRole>(
    value: &StructuralValue<VocabularyRoot>,
) -> Result<WholeLogosVisibility, Error> {
    match value.field::<Role>() {
        Some(FieldValue::Repeated(words)) if words.is_empty() => Ok(WholeLogosVisibility::Private),
        Some(FieldValue::Repeated(words))
            if words.len() == 1 && matches!(words[0], FieldValue::Literal(_)) =>
        {
            Ok(WholeLogosVisibility::Public)
        }
        _ => Err(Error::TypedPositionMismatch {
            position: "visibility",
        }),
    }
}

fn require_projection(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
    projections: &FixtureRustNameProjectionTable,
) -> Result<(), Error> {
    validate_universal(position, encoded_id)?;
    if projections.projected(encoded_id).is_none() {
        return Err(Error::MissingProjection {
            encoded_id: encoded_id.clone(),
        });
    }
    Ok(())
}

fn generated_names<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    logos: &WholeLogos,
    allocated: &Allocated,
) -> Result<BTreeMap<VocabularyEncodedId, Name>, Error> {
    let mut generated = BTreeMap::new();
    for item in logos.items() {
        match item {
            WholeLogosItem::Newtype(newtype) => {
                insert_generated("newtype name", newtype.name(), allocated, &mut generated)?;
                validate_production_reference(newtype.wrapped(), allocated, &mut generated)?;
            }
            WholeLogosItem::Enumeration(enumeration) => {
                insert_generated(
                    "enumeration name",
                    enumeration.name(),
                    allocated,
                    &mut generated,
                )?;
                for variant in enumeration.variants() {
                    insert_generated("variant name", variant.name(), allocated, &mut generated)?;
                    if let WholeLogosVariantPayload::Tuple(fields) = variant.payload() {
                        for field in fields.fields() {
                            validate_production_reference(field, allocated, &mut generated)?;
                        }
                    }
                }
            }
        }
    }
    Ok(generated)
}

fn validate_production_reference<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    reference: &WholeLogosTypeReference,
    allocated: &Allocated,
    generated: &mut BTreeMap<VocabularyEncodedId, Name>,
) -> Result<(), Error> {
    match reference {
        WholeLogosTypeReference::Identity(encoded_id) => {
            validate_allocated("type reference", encoded_id, allocated)?;
            if encoded_id.root_variant() == &VocabularyRoot::Universal {
                generated
                    .entry(encoded_id.clone())
                    .or_insert_with(|| RustEncodedIdCodec::encode_name(encoded_id));
            }
            Ok(())
        }
        WholeLogosTypeReference::Application(application) => {
            validate_allocated("application head", application.head(), allocated)?;
            if application.head().root_variant() == &VocabularyRoot::Universal {
                generated
                    .entry(application.head().clone())
                    .or_insert_with(|| RustEncodedIdCodec::encode_name(application.head()));
            }
            validate_production_reference(application.payload(), allocated, generated)
        }
    }
}

fn insert_generated<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
    allocated: &Allocated,
    generated: &mut BTreeMap<VocabularyEncodedId, Name>,
) -> Result<(), Error> {
    validate_universal(position, encoded_id)?;
    validate_allocated(position, encoded_id, allocated)?;
    generated
        .entry(encoded_id.clone())
        .or_insert_with(|| RustEncodedIdCodec::encode_name(encoded_id));
    Ok(())
}

fn validate_allocated<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
    allocated: &Allocated,
) -> Result<(), Error> {
    if allocated.resolve(encoded_id).is_none() {
        return Err(Error::UnallocatedEncodedIdentity {
            position,
            encoded_id: encoded_id.clone(),
        });
    }
    Ok(())
}

fn validate_universal(
    position: &'static str,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), Error> {
    if encoded_id.root_variant() != &VocabularyRoot::Universal {
        return Err(Error::NonUniversalIdentity {
            position,
            found: *encoded_id.root_variant(),
        });
    }
    Ok(())
}

fn repeated<'a, Role: FieldRole>(
    value: &'a StructuralValue<VocabularyRoot>,
    position: &'static str,
) -> Result<&'a [FieldValue<VocabularyRoot>], Error> {
    match value.field::<Role>() {
        Some(FieldValue::Repeated(values)) => Ok(values),
        _ => Err(Error::TypedPositionMismatch { position }),
    }
}

fn delegated<'a, Role: FieldRole>(
    value: &'a StructuralValue<VocabularyRoot>,
    position: &'static str,
) -> Result<&'a StructuralValue<VocabularyRoot>, Error> {
    match value.field::<Role>() {
        Some(FieldValue::Delegated(value)) => Ok(value),
        _ => Err(Error::TypedPositionMismatch { position }),
    }
}

fn declaration_id<Role: FieldRole>(
    value: &StructuralValue<VocabularyRoot>,
    position: &'static str,
) -> Result<VocabularyEncodedId, Error> {
    match value.field::<Role>() {
        Some(FieldValue::Declaration(assignment)) => {
            validate_universal(position, assignment.encoded_id())?;
            Ok(assignment.encoded_id().clone())
        }
        _ => Err(Error::TypedPositionMismatch { position }),
    }
}

fn reference_id<Role: FieldRole>(
    value: &StructuralValue<VocabularyRoot>,
    position: &'static str,
) -> Result<VocabularyEncodedId, Error> {
    match value.field::<Role>() {
        Some(FieldValue::Reference(reference)) => Ok(reference.encoded_id().clone()),
        _ => Err(Error::TypedPositionMismatch { position }),
    }
}

fn checked_bound(source: &str, start: usize, end: usize) -> Result<SourceBound, Error> {
    SourceBound::checked(source, start, end)
        .map_err(BlockDiscoveryError::from)
        .map_err(Error::from)
}

fn trim_bound(source: &str, bound: SourceBound) -> Result<SourceBound, Error> {
    let text = &source[bound.start()..bound.end()];
    let without_leading = text.trim_start();
    let leading = text.len() - without_leading.len();
    let trimmed = without_leading.trim_end();
    let start = bound.start() + leading;
    checked_bound(source, start, start + trimmed.len())
}

fn offset_decode_error(
    error: DecodeError<VocabularyRoot>,
    source: &str,
    offset: usize,
) -> DecodeError<VocabularyRoot> {
    let offset_bound = |bound: SourceBound| {
        SourceBound::checked(source, offset + bound.start(), offset + bound.end())
            .expect("relative evaluator bounds stay inside the complete source")
    };
    match error {
        DecodeError::MissingDeclarationAssignment { bound } => {
            DecodeError::MissingDeclarationAssignment {
                bound: offset_bound(bound),
            }
        }
        DecodeError::UnresolvedReference { bound } => DecodeError::UnresolvedReference {
            bound: offset_bound(bound),
        },
        DecodeError::NameBindingMismatch { bound } => DecodeError::NameBindingMismatch {
            bound: offset_bound(bound),
        },
        DecodeError::ProductPositionMismatch {
            position,
            role,
            bound,
        } => DecodeError::ProductPositionMismatch {
            position,
            role,
            bound: offset_bound(bound),
        },
        DecodeError::SequenceRepetitionBoundary { role, refusal } => {
            DecodeError::SequenceRepetitionBoundary {
                role,
                refusal: Box::new(offset_decode_error(*refusal, source, offset)),
            }
        }
        other => other,
    }
}

struct FixtureResolver<'a> {
    vocabulary: &'a FixtureRustVocabulary,
    projections: &'a FixtureRustNameProjectionTable,
}

impl EncodedNameResolver<VocabularyRoot> for FixtureResolver<'_> {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.vocabulary
            .resolve(encoded_id)
            .or_else(|| self.projections.resolve(encoded_id))
    }
}

struct ProductionResolver<'a, Allocated: ?Sized> {
    vocabulary: &'a FixtureRustVocabulary,
    allocated: &'a Allocated,
    generated: BTreeMap<VocabularyEncodedId, Name>,
}

impl<Allocated: EncodedNameResolver<VocabularyRoot> + ?Sized> EncodedNameResolver<VocabularyRoot>
    for ProductionResolver<'_, Allocated>
{
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.vocabulary
            .resolve(encoded_id)
            .or_else(|| match encoded_id.root_variant() {
                VocabularyRoot::Universal => self.generated.get(encoded_id),
                VocabularyRoot::Rust => self.allocated.resolve(encoded_id),
            })
    }
}

struct OffsetBindings<'a, Bindings: ?Sized> {
    vocabulary: &'a FixtureRustVocabulary,
    inner: &'a Bindings,
    source: &'a str,
    offset: usize,
}

impl<Bindings: EncodedNameResolver<VocabularyRoot> + ?Sized> EncodedNameResolver<VocabularyRoot>
    for OffsetBindings<'_, Bindings>
{
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.vocabulary
            .resolve(encoded_id)
            .or_else(|| self.inner.resolve(encoded_id))
    }
}

impl<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized> DecodeNameBindings<VocabularyRoot>
    for OffsetBindings<'_, Bindings>
{
    fn declaration_assignment(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        self.inner
            .declaration_assignment(self.absolute_occurrence(occurrence))
    }

    fn reference_resolution(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        self.inner
            .reference_resolution(self.absolute_occurrence(occurrence))
    }
}

impl<Bindings: ?Sized> OffsetBindings<'_, Bindings> {
    fn absolute_occurrence<'a>(&self, occurrence: NameOccurrence<'a>) -> NameOccurrence<'a> {
        let bound = SourceBound::checked(
            self.source,
            self.offset + occurrence.bound().start(),
            self.offset + occurrence.bound().end(),
        )
        .expect("relative evaluator bounds stay inside the complete source");
        NameOccurrence::new(occurrence.spelling(), bound)
    }
}

struct RenderedFixtureDocument(Vec<String>);

impl std::fmt::Display for RenderedFixtureDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in &self.0 {
            writeln!(formatter, "{item}")?;
        }
        Ok(())
    }
}
