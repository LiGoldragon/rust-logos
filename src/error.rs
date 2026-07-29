//! Typed refusals at the Rust TextualForm boundary.

use raw_discovery::{BlockDiscoveryError, SourceBound};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{AuthoringError, DecodeError, EncodeError, TableError};

/// Why an opaque emitted-name token is not a Rust identifier in the supported
/// structural subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustIdentifierRefusal {
    /// The token has no characters.
    Empty,
    /// The first character is outside the accepted identifier class.
    InvalidLeadingCharacter,
    /// A later character is outside the accepted identifier class.
    InvalidContinuationCharacter,
    /// `_` alone is not an item identifier.
    UnderscoreOnly,
    /// The token is a Rust keyword or reserved word.
    ReservedWord,
}

/// A typed failure while sealing or evaluating the Slice One Rust vocabulary.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A caller supplied a Rust-owned vocabulary position under another root.
    #[error("{position} must use the Rust vocabulary root, found {found:?}")]
    NonRustVocabulary {
        /// The position being validated.
        position: &'static str,
        /// The supplied root.
        found: VocabularyRoot,
    },

    /// A whole-Logos name position was not a Universal identity.
    #[error("{position} must use the Universal vocabulary root, found {found:?}")]
    NonUniversalIdentity {
        /// The position being validated.
        position: &'static str,
        /// The supplied root.
        found: VocabularyRoot,
    },

    /// A required translator-issued vocabulary identity did not resolve.
    #[error("{position} has no spelling in the supplied immutable vocabulary")]
    MissingVocabularyName {
        /// The position being validated.
        position: &'static str,
        /// The unresolved identity.
        encoded_id: VocabularyEncodedId,
    },

    /// A fixed Rust vocabulary position resolved to the wrong exact word.
    #[error("{position} resolved to {found:?}; expected {expected:?}")]
    VocabularySpellingMismatch {
        /// The position being validated.
        position: &'static str,
        /// The expected Rust spelling.
        expected: &'static str,
        /// The spelling in the supplied vocabulary.
        found: String,
    },

    /// An emitted name did not satisfy the conservative Rust identifier subset.
    #[error("opaque emitted-name token {token:?} is invalid: {reason:?}")]
    InvalidRustIdentifier {
        /// The supplied token.
        token: String,
        /// The structural refusal.
        reason: RustIdentifierRefusal,
    },

    /// One encoded identity was projected twice.
    #[error("encoded identity {encoded_id:?} has more than one emitted-name projection")]
    DuplicateProjectionIdentity {
        /// The repeated identity.
        encoded_id: VocabularyEncodedId,
    },

    /// Two different identities were assigned the same emitted token.
    #[error("opaque emitted-name token {token:?} is associated with different identities")]
    ProjectionTokenConflict {
        /// The colliding token.
        token: String,
    },

    /// An emitted item or reference has no opaque token at the boundary.
    #[error("encoded identity {encoded_id:?} has no opaque Rust emitted-name projection")]
    MissingProjection {
        /// The unresolved identity.
        encoded_id: VocabularyEncodedId,
    },

    /// No cue-terminated Rust struct item was present.
    #[error("Rust source contains no struct item")]
    NoRustItems,

    /// Source outside one discovered newtype block was not solely its typed
    /// visibility position and trivia.
    #[error("unsupported Rust source outside the struct cue at {bound:?}")]
    UnsupportedItemPrefix {
        /// The refused source range.
        bound: SourceBound,
    },

    /// A discovered struct did not have the one tuple field shape.
    #[error("struct block at {bound:?} is not an attribute-free one-field tuple newtype")]
    UnsupportedNewtypeShape {
        /// The refused item range.
        bound: SourceBound,
    },

    /// A sealed typed record returned a value under a different role or value
    /// kind than its record declares.
    #[error("shared evaluator did not return the declared {position} typed position")]
    TypedPositionMismatch {
        /// The typed position being reified.
        position: &'static str,
    },

    /// Source remained after the final discovered item.
    #[error("unsupported Rust source after the final newtype at {bound:?}")]
    TrailingSource {
        /// The refused source range.
        bound: SourceBound,
    },

    /// The raw pass-one rules refused the source.
    #[error(transparent)]
    Discovery(#[from] BlockDiscoveryError),

    /// Authoring a typed position record failed.
    #[error(transparent)]
    Authoring(#[from] AuthoringError),

    /// The shared evaluator's table seal refused the vocabulary.
    #[error(transparent)]
    Table(Box<TableError<VocabularyRoot>>),

    /// The shared evaluator refused a typed decode position.
    #[error(transparent)]
    Decode(#[from] DecodeError<VocabularyRoot>),

    /// The shared evaluator refused a typed emission position.
    #[error(transparent)]
    Encode(#[from] EncodeError<VocabularyRoot>),
}

impl From<TableError<VocabularyRoot>> for Error {
    fn from(error: TableError<VocabularyRoot>) -> Self {
        Self::Table(Box::new(error))
    }
}
