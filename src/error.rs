//! Typed refusals at the Rust TextualForm boundary.

use raw_discovery::{BlockDiscoveryError, SourceBound};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{AuthoringError, DecodeError, EncodeError, TableError};

/// Why a textual encoded-ID token is not the canonical supported format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EncodedIdCodecRefusal {
    /// The token is empty and therefore carries no multibase discriminator.
    #[error("encoded-ID token has no multibase prefix")]
    MissingMultibasePrefix,
    /// The token selects another multibase or is ordinary text.
    #[error("encoded-ID token uses multibase prefix {found:?}, expected 'z'")]
    WrongMultibasePrefix { found: char },
    /// The Base58BTC body is empty or contains a character outside its alphabet.
    #[error("encoded-ID token has malformed Base58BTC data")]
    MalformedBase58,
    /// The Base58BTC body is not the sole canonical rendering of its bytes.
    #[error("encoded-ID token is not canonical Base58BTC")]
    NonCanonicalBase58,
    /// The packed payload selects a format version this build cannot interpret.
    #[error("encoded-ID format version {found} is unsupported")]
    UnsupportedFormatVersion { found: u8 },
    /// The packed payload contains no known production root tag.
    #[error("encoded-ID root tag {found} is unsupported")]
    UnsupportedRoot { found: u8 },
    /// The packed payload cannot carry both its version and root.
    #[error("encoded-ID payload length {found} cannot carry its header")]
    InvalidPayloadLength { found: usize },
    /// Bytes after the header cannot be divided into complete big-endian `u16`s.
    #[error("encoded-ID payload length {found} leaves a partial local ID")]
    InvalidPayloadParity { found: usize },
    /// The payload addresses a table rather than a durable entry.
    #[error("encoded-ID payload has an empty local-ID chain")]
    EmptyChain,
}

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
    /// A legacy Logos item carried an identifier absent from its sibling table.
    #[error("legacy Logos name projection failed: {0}")]
    LegacyName(#[from] legacy_name_table::NameTableError),

    /// A legacy Logos token structure did not form a Rust item or file.
    #[error("legacy Logos token structure did not parse as Rust: {0}")]
    Project(String),

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

    /// A caller-owned external type path was empty or not a canonical Rust
    /// path assembled from distinct segments.
    #[error("external Rust type path {path:?} is invalid")]
    InvalidExternalRustTypePath { path: String },

    /// An identity declared by this WholeLogos document was also configured as
    /// externally owned, which would make ownership ambiguous.
    #[error("declared identity {encoded_id:?} is also configured as an external Rust type")]
    ExternalRustTypeDeclaration { encoded_id: VocabularyEncodedId },

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

    /// The owning naming boundary does not currently allocate an emitted
    /// identity.
    #[error("{position} uses unallocated encoded identity {encoded_id:?}")]
    UnallocatedEncodedIdentity {
        /// The typed Logos position being validated.
        position: &'static str,
        /// The complete unallocated identity.
        encoded_id: VocabularyEncodedId,
    },

    /// No cue-terminated fixture item was present.
    #[error("Rust source contains no supported fixture item")]
    NoRustItems,

    /// Source outside one discovered newtype block was not solely its typed
    /// visibility position and trivia.
    #[error("unsupported Rust source outside the struct cue at {bound:?}")]
    UnsupportedItemPrefix {
        /// The refused source range.
        bound: SourceBound,
    },

    /// A discovered item did not fit its typed fixture record.
    #[error("Rust item at {bound:?} does not fit the bounded structural fixture")]
    UnsupportedItemShape {
        /// The refused item range.
        bound: SourceBound,
    },

    /// Rust enum tuple fields do not admit a visibility modifier.
    #[error("enumeration tuple fields must have private visibility")]
    UnsupportedVariantFieldVisibility,

    /// An enumeration tuple payload did not carry exactly one field.
    #[error("enumeration tuple payload requires exactly one field, found {found}")]
    UnsupportedVariantTupleArity {
        /// Refused positional-field count.
        found: usize,
    },

    /// Two configured Interface roles were assigned the same identity.
    #[error("Interface roles {first_role} and {second_role} share identity {identity:?}")]
    DuplicateInterfaceRoleIdentity {
        /// First configured role.
        first_role: &'static str,
        /// Second configured role.
        second_role: &'static str,
        /// Reused Universal identity.
        identity: VocabularyEncodedId,
    },

    /// A Refusal membership carried behavior not present in the marker trait.
    #[error("Refusal membership must not bind associated types, found {found}")]
    RefusalImplementationAssociatedTypes {
        /// Number of structurally unsupported bindings.
        found: usize,
    },

    /// A typed WholeLogos table could not produce its content-derived schema
    /// identity before Rust assembly.
    #[error("Sema table schema hash failed: {message}")]
    TableSchemaHash {
        /// Portable archive failure without record or key contents.
        message: String,
    },

    /// A sealed typed record returned a value under a different role or value
    /// kind than its record declares.
    #[error("shared evaluator did not return the declared {position} typed position")]
    TypedPositionMismatch {
        /// The typed position being reified.
        position: &'static str,
    },

    /// Source remained after the final discovered item.
    #[error("unsupported Rust source after the final fixture item at {bound:?}")]
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
