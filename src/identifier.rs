//! Opaque Rust emitted-name projections.
//!
//! No rule here maps an encoded-ID chain to text. The caller supplies a token
//! already associated with one complete Universal identity; this module only
//! validates the token and preserves that association.

use std::collections::{BTreeMap, BTreeSet};

use name_table::Name;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::EncodedNameResolver;

use crate::{Error, RustIdentifierRefusal};

/// A rustc-safe opaque token in the conservative ASCII identifier subset.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustEmittedIdentifier(Name);

impl RustEmittedIdentifier {
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

/// A checked association between complete Universal encoded-ID chains and
/// opaque Rust tokens.
#[derive(Clone, Debug, Default)]
pub struct RustNameProjectionTable {
    entries: BTreeMap<VocabularyEncodedId, RustEmittedIdentifier>,
}

impl RustNameProjectionTable {
    /// Build a projection table without deriving a token from any chain.
    pub fn try_from_entries(
        entries: impl IntoIterator<Item = (VocabularyEncodedId, RustEmittedIdentifier)>,
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
    pub fn projected(&self, encoded_id: &VocabularyEncodedId) -> Option<&RustEmittedIdentifier> {
        self.entries.get(encoded_id)
    }
}

impl EncodedNameResolver<VocabularyRoot> for RustNameProjectionTable {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.projected(encoded_id)
            .map(RustEmittedIdentifier::as_name)
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
