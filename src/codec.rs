//! The single Rust-specific evaluator object permitted for the first MVP.
//!
//! It owns only orchestration that the shared evaluator does not yet express:
//! cue-to-termination item bounds, Rust adjacency, and fixed punctuation.
//! Every semantic token is decoded and encoded by `StructuralEvaluator`
//! against a real typed position record.

use core_logos::{WholeLogos, WholeLogosItem, WholeLogosNewtype, WholeLogosVisibility};
use name_table::Name;
use raw_discovery::{
    BlockDiscoveryError, BlockTree, CueTerminatedBlockCueEvidence, DiscoveredCueTerminatedBlock,
    DiscoveredCueTerminatedBlockTree, SourceBound,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, FieldRole,
    FieldValue, NameOccurrence, ResolvedReference, StructuralEvaluator, StructuralValue,
};

use crate::vocabulary::{
    DeclarationNamePosition, PARENTHESIS, PublicKeywordPosition, ReferencedTypePosition,
    RustNewtypeRule, RustNewtypeVocabulary, STRUCT_CUE, StructKeywordPosition, constructor_for,
};
use crate::{Error, RustNameProjectionTable};

/// Bidirectional structural Rust view for the attribute-free newtype slice.
pub struct RustLogos {
    vocabulary: RustNewtypeVocabulary,
}

impl RustLogos {
    /// Seat the already-sealed typed Rust vocabulary.
    pub fn new(vocabulary: RustNewtypeVocabulary) -> Self {
        Self { vocabulary }
    }

    /// The exact structural rule data used for all semantic tokens.
    pub fn vocabulary(&self) -> &RustNewtypeVocabulary {
        &self.vocabulary
    }

    /// Decode ordered Rust newtypes after cue-to-termination discovery.
    ///
    /// `bindings` supplies translator-issued declaration assignments and
    /// lookup-only references for the caller's opaque Rust token projection.
    /// The offset adapter below preserves absolute source bounds when each
    /// typed position is evaluated.
    pub fn decode<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
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

        let evaluator = StructuralEvaluator::<VocabularyRoot, RustNewtypeRule>::new(
            self.vocabulary.structuretree(),
        )?;
        let mut items = Vec::with_capacity(tree.root_blocks().len());
        let mut previous_end = 0;

        for block in tree.root_blocks() {
            let cue_start = block.cue().bound().start();
            let prefix = checked_bound(source, previous_end, cue_start)?;
            let visibility =
                self.decode_visibility(source, prefix, bindings, &evaluator, "item visibility")?;
            let newtype = self.decode_block(source, block, visibility, bindings, &evaluator)?;
            items.push(WholeLogosItem::Newtype(newtype));
            previous_end = block.source_bound().end();
        }

        let trailing = trim_bound(source, checked_bound(source, previous_end, source.len())?)?;
        if !trailing.is_empty() {
            return Err(Error::TrailingSource { bound: trailing });
        }
        Ok(WholeLogos::new(items))
    }

    /// Emit structural, attribute-free Rust in whole-Logos item order.
    ///
    /// The projection table contains caller-supplied rustc-safe tokens. This
    /// method neither derives nor guesses an encoded-ID textual encoding.
    pub fn emit(
        &self,
        logos: &WholeLogos,
        projections: &RustNameProjectionTable,
    ) -> Result<String, Error> {
        let evaluator = StructuralEvaluator::<VocabularyRoot, RustNewtypeRule>::new(
            self.vocabulary.structuretree(),
        )?;
        let struct_keyword = self.encode_literal::<StructKeywordPosition>(
            self.vocabulary.ids().struct_keyword_type(),
            self.vocabulary.ids().struct_keyword(),
            &evaluator,
        )?;
        let separator = self.vocabulary.item_separator();
        let (tuple_opening, tuple_closing) = self.vocabulary.tuple_delimiters();
        let termination = self.vocabulary.item_termination();
        let mut rendered = String::new();
        for item in logos.items() {
            match item {
                WholeLogosItem::Newtype(newtype) => {
                    let item_visibility =
                        self.encode_visibility(newtype.visibility(), &evaluator)?;
                    let name = self.encode_declaration(newtype.name(), projections, &evaluator)?;
                    let wrapped_visibility =
                        self.encode_visibility(newtype.wrapped_visibility(), &evaluator)?;
                    let wrapped =
                        self.encode_reference(newtype.wrapped(), projections, &evaluator)?;
                    rendered.push_str(&item_visibility);
                    rendered.push_str(&struct_keyword);
                    rendered.push_str(separator);
                    rendered.push_str(&name);
                    rendered.push_str(tuple_opening);
                    rendered.push_str(&wrapped_visibility);
                    rendered.push_str(&wrapped);
                    rendered.push_str(tuple_closing);
                    rendered.push_str(termination);
                    rendered.push('\n');
                }
            }
        }
        Ok(rendered)
    }

    fn decode_block<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        block: &DiscoveredCueTerminatedBlock,
        item_visibility: WholeLogosVisibility,
        bindings: &Bindings,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<WholeLogosNewtype, Error> {
        if block.cue().evidence() != CueTerminatedBlockCueEvidence::CueTermination(STRUCT_CUE) {
            return Err(Error::UnsupportedNewtypeShape {
                bound: block.source_bound(),
            });
        }
        self.decode_fixed_position(
            source,
            block.cue().bound(),
            self.vocabulary.ids().struct_keyword_type(),
            evaluator,
        )?;

        let [tuple] = block.children() else {
            return Err(Error::UnsupportedNewtypeShape {
                bound: block.source_bound(),
            });
        };
        if tuple.cue().evidence() != CueTerminatedBlockCueEvidence::Boundary(PARENTHESIS)
            || !tuple.children().is_empty()
        {
            return Err(Error::UnsupportedNewtypeShape {
                bound: block.source_bound(),
            });
        }

        let name_bound = trim_bound(
            source,
            checked_bound(
                source,
                block.content_bound().start(),
                tuple.source_bound().start(),
            )?,
        )?;
        let after_tuple = trim_bound(
            source,
            checked_bound(
                source,
                tuple.source_bound().end(),
                block.content_bound().end(),
            )?,
        )?;
        let field_bound = trim_bound(source, tuple.content_bound())?;
        if name_bound.is_empty() || !after_tuple.is_empty() || field_bound.is_empty() {
            return Err(Error::UnsupportedNewtypeShape {
                bound: block.source_bound(),
            });
        }

        let name = self.decode_declaration(source, name_bound, bindings, evaluator)?;
        let (wrapped_visibility, reference_bound) =
            self.field_visibility(source, field_bound, bindings, evaluator)?;
        let wrapped = self.decode_reference(source, reference_bound, bindings, evaluator)?;
        Ok(WholeLogosNewtype::new(
            item_visibility,
            name,
            wrapped_visibility,
            wrapped,
        ))
    }

    fn field_visibility<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        field: SourceBound,
        bindings: &Bindings,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<(WholeLogosVisibility, SourceBound), Error> {
        let public = self
            .vocabulary
            .resolve(self.vocabulary.ids().public_keyword())
            .expect("the fixed public word was validated at seal")
            .as_str();
        let text = &source[field.start()..field.end()];
        let public_end = field.start() + public.len();
        let has_public_prefix = text.starts_with(public)
            && text[public.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
        if !has_public_prefix {
            return Ok((WholeLogosVisibility::Private, field));
        }

        let keyword_bound = checked_bound(source, field.start(), public_end)?;
        self.decode_fixed_position(
            source,
            keyword_bound,
            self.vocabulary.ids().public_keyword_type(),
            evaluator,
        )?;
        let reference = trim_bound(source, checked_bound(source, public_end, field.end())?)?;
        if reference.is_empty() {
            return Err(Error::UnsupportedNewtypeShape { bound: field });
        }
        // The bindings remain lookup-only here; the public word was decoded
        // against the immutable Rust table, while the following token is
        // resolved through the caller's Universal projection.
        let _ = bindings;
        Ok((WholeLogosVisibility::Public, reference))
    }

    fn decode_visibility<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bound: SourceBound,
        _bindings: &Bindings,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
        _position: &'static str,
    ) -> Result<WholeLogosVisibility, Error> {
        let bound = trim_bound(source, bound)?;
        if bound.is_empty() {
            return Ok(WholeLogosVisibility::Private);
        }
        self.decode_fixed_position(
            source,
            bound,
            self.vocabulary.ids().public_keyword_type(),
            evaluator,
        )?;
        Ok(WholeLogosVisibility::Public)
    }

    fn decode_fixed_position(
        &self,
        source: &str,
        bound: SourceBound,
        expected: &structural_codec::EncodedTypeId<VocabularyRoot>,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<(), Error> {
        let slice = &source[bound.start()..bound.end()];
        let fixed = FixedBindings(&self.vocabulary);
        let decoded = evaluator
            .decode_text(expected, slice, &fixed)
            .map_err(|error| Error::Decode(offset_decode_error(error, source, bound.start())))?;
        if decoded.constructor() != &constructor_for(expected) {
            return Err(Error::TypedPositionMismatch {
                position: "fixed Rust vocabulary",
            });
        }
        Ok(())
    }

    fn decode_declaration<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bound: SourceBound,
        bindings: &Bindings,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<VocabularyEncodedId, Error> {
        let offset = OffsetBindings {
            inner: bindings,
            source,
            offset: bound.start(),
        };
        let value = evaluator
            .decode_text(
                self.vocabulary.ids().declaration_name_type(),
                &source[bound.start()..bound.end()],
                &offset,
            )
            .map_err(|error| Error::Decode(offset_decode_error(error, source, bound.start())))?;
        match value.field::<DeclarationNamePosition>() {
            Some(FieldValue::Declaration(assignment)) => {
                validate_universal("newtype name", assignment.encoded_id())?;
                Ok(assignment.encoded_id().clone())
            }
            _ => Err(Error::TypedPositionMismatch {
                position: "declaration name",
            }),
        }
    }

    fn decode_reference<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bound: SourceBound,
        bindings: &Bindings,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<VocabularyEncodedId, Error> {
        let offset = OffsetBindings {
            inner: bindings,
            source,
            offset: bound.start(),
        };
        let value = evaluator
            .decode_text(
                self.vocabulary.ids().referenced_type_type(),
                &source[bound.start()..bound.end()],
                &offset,
            )
            .map_err(|error| Error::Decode(offset_decode_error(error, source, bound.start())))?;
        match value.field::<ReferencedTypePosition>() {
            Some(FieldValue::Reference(reference)) => {
                validate_universal("wrapped type", reference.encoded_id())?;
                Ok(reference.encoded_id().clone())
            }
            _ => Err(Error::TypedPositionMismatch {
                position: "referenced type",
            }),
        }
    }

    fn encode_visibility(
        &self,
        visibility: &WholeLogosVisibility,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<String, Error> {
        match visibility {
            WholeLogosVisibility::Private => Ok(String::new()),
            WholeLogosVisibility::Public => {
                let word = self.encode_literal::<PublicKeywordPosition>(
                    self.vocabulary.ids().public_keyword_type(),
                    self.vocabulary.ids().public_keyword(),
                    evaluator,
                )?;
                Ok(format!("{word}{}", self.vocabulary.item_separator()))
            }
        }
    }

    fn encode_literal<Role: FieldRole>(
        &self,
        expected: &structural_codec::EncodedTypeId<VocabularyRoot>,
        encoded_id: &VocabularyEncodedId,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<String, Error> {
        let mut record = StructuralValue::record(constructor_for(expected));
        record.insert::<Role>(FieldValue::Literal(encoded_id.clone()))?;
        Ok(evaluator.encode_text(expected, &record.finish(), &self.vocabulary)?)
    }

    fn encode_declaration(
        &self,
        encoded_id: &VocabularyEncodedId,
        projections: &RustNameProjectionTable,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<String, Error> {
        validate_universal("newtype name", encoded_id)?;
        if projections.projected(encoded_id).is_none() {
            return Err(Error::MissingProjection {
                encoded_id: encoded_id.clone(),
            });
        }
        let expected = self.vocabulary.ids().declaration_name_type();
        let mut record = StructuralValue::record(constructor_for(expected));
        record.insert::<DeclarationNamePosition>(FieldValue::Declaration(
            DeclarationAssignment::new(encoded_id.clone()),
        ))?;
        Ok(evaluator.encode_text(expected, &record.finish(), projections)?)
    }

    fn encode_reference(
        &self,
        encoded_id: &VocabularyEncodedId,
        projections: &RustNameProjectionTable,
        evaluator: &StructuralEvaluator<'_, VocabularyRoot, RustNewtypeRule>,
    ) -> Result<String, Error> {
        validate_universal("wrapped type", encoded_id)?;
        if projections.projected(encoded_id).is_none() {
            return Err(Error::MissingProjection {
                encoded_id: encoded_id.clone(),
            });
        }
        let expected = self.vocabulary.ids().referenced_type_type();
        let mut record = StructuralValue::record(constructor_for(expected));
        record.insert::<ReferencedTypePosition>(FieldValue::Reference(ResolvedReference::new(
            encoded_id.clone(),
        )))?;
        Ok(evaluator.encode_text(expected, &record.finish(), projections)?)
    }
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
            .expect("a typed-position failure bound stays inside its original source")
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
        other => other,
    }
}

struct FixedBindings<'names>(&'names RustNewtypeVocabulary);

impl EncodedNameResolver<VocabularyRoot> for FixedBindings<'_> {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.0.resolve(encoded_id)
    }
}

impl DecodeNameBindings<VocabularyRoot> for FixedBindings<'_> {
    fn declaration_assignment(
        &self,
        _occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        None
    }

    fn reference_resolution(
        &self,
        _occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        None
    }
}

struct OffsetBindings<'bindings, Bindings: ?Sized> {
    inner: &'bindings Bindings,
    source: &'bindings str,
    offset: usize,
}

impl<Bindings: EncodedNameResolver<VocabularyRoot> + ?Sized> EncodedNameResolver<VocabularyRoot>
    for OffsetBindings<'_, Bindings>
{
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.inner.resolve(encoded_id)
    }
}

impl<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized> DecodeNameBindings<VocabularyRoot>
    for OffsetBindings<'_, Bindings>
{
    fn declaration_assignment(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        let bound = SourceBound::checked(
            self.source,
            self.offset + occurrence.bound().start(),
            self.offset + occurrence.bound().end(),
        )
        .expect("a relative bound inside the sliced source stays inside the original source");
        self.inner
            .declaration_assignment(NameOccurrence::new(occurrence.spelling(), bound))
    }

    fn reference_resolution(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        let bound = SourceBound::checked(
            self.source,
            self.offset + occurrence.bound().start(),
            self.offset + occurrence.bound().end(),
        )
        .expect("a relative bound inside the sliced source stays inside the original source");
        self.inner
            .reference_resolution(NameOccurrence::new(occurrence.spelling(), bound))
    }
}
