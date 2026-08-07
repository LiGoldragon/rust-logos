//! Refusals at the Rust textual projection boundary.

use name_table::EncodedName;

/// A typed refusal while projecting Logos through an authority name view.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The authority view has no current textual metadata for a referenced name.
    #[error("encoded name {encoded_name:?} has no textual metadata")]
    MissingName { encoded_name: EncodedName },
    /// A textual projection is not a single Rust identifier.
    #[error("textual name {token:?} is not a Rust identifier")]
    InvalidRustIdentifier { token: String },
    /// An explicit imported Rust path is malformed.
    #[error("external Rust type path {path:?} is invalid")]
    InvalidExternalRustTypePath { path: String },
    /// Interface role identities must be distinct.
    #[error("Interface role identities must be distinct")]
    DuplicateInterfaceRoleIdentity,
}
