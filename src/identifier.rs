//! Rust emitted-name encoding and fixture projections.
//!
//! Production names are computed from complete encoded-ID chains. Fixture
//! projections remain isolated here so the earlier slice witness stays useful
//! without becoming a production naming authority.

use std::collections::{BTreeMap, BTreeSet};

use name_table::{LocalEncodedId, Name};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::EncodedNameResolver;

use crate::{EncodedIdCodecRefusal, Error, RustIdentifierRefusal};

/// Version byte at the front of the packed encoded-ID payload.
pub const ENCODED_ID_FORMAT_VERSION: u8 = 1;

/// Multibase discriminator for Base58BTC.
pub const BASE58BTC_MULTIBASE_PREFIX: char = 'z';

/// Canonical production codec for root-fronted encoded-ID chains.
///
/// The payload is the format version, the explicit production root tag, then
/// every table-local `u16` in big-endian order. Base58BTC preserves the exact
/// bytes while using only Rust identifier continuation characters. Its
/// multibase `z` prefix also guarantees a valid non-keyword leading character.
pub struct RustEncodedIdCodec;

impl RustEncodedIdCodec {
    /// Compute the Rust identifier for one complete durable identity.
    pub fn encode(encoded_id: &VocabularyEncodedId) -> String {
        let mut payload = Vec::with_capacity(2 + encoded_id.chain().len() * 2);
        payload.push(ENCODED_ID_FORMAT_VERSION);
        payload.push(encoded_id.root_variant().tag());
        for local in encoded_id.chain() {
            payload.extend_from_slice(&local.value().to_be_bytes());
        }
        let mut token = String::from(BASE58BTC_MULTIBASE_PREFIX);
        token.push_str(&bs58::encode(payload).into_string());
        token
    }

    /// Recover the complete durable identity from one canonical Rust token.
    pub fn decode(token: &str) -> Result<VocabularyEncodedId, EncodedIdCodecRefusal> {
        let mut characters = token.chars();
        let Some(prefix) = characters.next() else {
            return Err(EncodedIdCodecRefusal::MissingMultibasePrefix);
        };
        if prefix != BASE58BTC_MULTIBASE_PREFIX {
            return Err(EncodedIdCodecRefusal::WrongMultibasePrefix { found: prefix });
        }
        let body = characters.as_str();
        if body.is_empty() {
            return Err(EncodedIdCodecRefusal::MalformedBase58);
        }
        let payload = bs58::decode(body)
            .into_vec()
            .map_err(|_| EncodedIdCodecRefusal::MalformedBase58)?;
        if body.starts_with('1') || bs58::encode(&payload).into_string() != body {
            return Err(EncodedIdCodecRefusal::NonCanonicalBase58);
        }
        let Some(&version) = payload.first() else {
            return Err(EncodedIdCodecRefusal::MalformedBase58);
        };
        if version != ENCODED_ID_FORMAT_VERSION {
            return Err(EncodedIdCodecRefusal::UnsupportedFormatVersion { found: version });
        }
        let Some(&root_tag) = payload.get(1) else {
            return Err(EncodedIdCodecRefusal::InvalidPayloadLength {
                found: payload.len(),
            });
        };
        let root = VocabularyRoot::try_from(root_tag)
            .map_err(|_| EncodedIdCodecRefusal::UnsupportedRoot { found: root_tag })?;
        let chain_bytes = &payload[2..];
        if chain_bytes.is_empty() {
            return Err(EncodedIdCodecRefusal::EmptyChain);
        }
        if chain_bytes.len() % 2 != 0 {
            return Err(EncodedIdCodecRefusal::InvalidPayloadParity {
                found: payload.len(),
            });
        }
        let chain = chain_bytes
            .chunks_exact(2)
            .map(|bytes| LocalEncodedId::new(u16::from_be_bytes([bytes[0], bytes[1]])))
            .collect();
        VocabularyEncodedId::new(root, chain).map_err(|_| EncodedIdCodecRefusal::EmptyChain)
    }

    pub(crate) fn encode_name(encoded_id: &VocabularyEncodedId) -> Name {
        Name::new(Self::encode(encoded_id))
    }
}

/// A fixture-only rustc-safe opaque token in the conservative ASCII subset.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FixtureRustEmittedIdentifier(Name);

impl FixtureRustEmittedIdentifier {
    /// Validate one caller-supplied token.
    pub fn try_new(token: impl Into<String>) -> Result<Self, Error> {
        let token = token.into();
        validate_identifier(&token)?;
        Ok(Self(Name::new(token)))
    }

    /// The exact token passed to rustc.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn as_name(&self) -> &Name {
        &self.0
    }
}

/// A fixture-only association between complete Universal encoded-ID chains and
/// opaque Rust tokens.
#[derive(Clone, Debug, Default)]
pub struct FixtureRustNameProjectionTable {
    entries: BTreeMap<VocabularyEncodedId, FixtureRustEmittedIdentifier>,
}

impl FixtureRustNameProjectionTable {
    /// Build a projection table without deriving a token from any chain.
    pub fn try_from_entries(
        entries: impl IntoIterator<Item = (VocabularyEncodedId, FixtureRustEmittedIdentifier)>,
    ) -> Result<Self, Error> {
        let mut projected = BTreeMap::new();
        let mut tokens = BTreeSet::new();
        for (encoded_id, token) in entries {
            if encoded_id.root_variant() != &VocabularyRoot::Universal {
                return Err(Error::NonUniversalIdentity {
                    position: "emitted name",
                    found: *encoded_id.root_variant(),
                });
            }
            if projected.contains_key(&encoded_id) {
                return Err(Error::DuplicateProjectionIdentity { encoded_id });
            }
            if !tokens.insert(token.clone()) {
                return Err(Error::ProjectionTokenConflict {
                    token: token.as_str().to_owned(),
                });
            }
            projected.insert(encoded_id, token);
        }
        Ok(Self { entries: projected })
    }

    /// Read the opaque token for one complete encoded-ID chain.
    pub fn projected(
        &self,
        encoded_id: &VocabularyEncodedId,
    ) -> Option<&FixtureRustEmittedIdentifier> {
        self.entries.get(encoded_id)
    }
}

impl EncodedNameResolver<VocabularyRoot> for FixtureRustNameProjectionTable {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.projected(encoded_id)
            .map(FixtureRustEmittedIdentifier::as_name)
    }
}

fn validate_identifier(token: &str) -> Result<(), Error> {
    let mut characters = token.chars();
    let Some(first) = characters.next() else {
        return invalid(token, RustIdentifierRefusal::Empty);
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return invalid(token, RustIdentifierRefusal::InvalidLeadingCharacter);
    }
    if !characters.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return invalid(token, RustIdentifierRefusal::InvalidContinuationCharacter);
    }
    if token == "_" {
        return invalid(token, RustIdentifierRefusal::UnderscoreOnly);
    }
    if is_reserved(token) {
        return invalid(token, RustIdentifierRefusal::ReservedWord);
    }
    Ok(())
}

fn invalid(token: &str, reason: RustIdentifierRefusal) -> Result<(), Error> {
    Err(Error::InvalidRustIdentifier {
        token: token.to_owned(),
        reason,
    })
}

fn is_reserved(token: &str) -> bool {
    matches!(
        token,
        "Self"
            | "abstract"
            | "as"
            | "async"
            | "await"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "override"
            | "priv"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "union"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}
