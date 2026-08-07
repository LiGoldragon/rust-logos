//! # rust-logos
//!
//! Rust emission for lowered Logos through a sealed read-only name view.

mod codec;
mod error;

pub use codec::{
    InterfaceRustEmission, InterfaceRustRoleIds, RustLogos, RustTypePath, RustTypePathResolver,
};
pub use error::Error;
