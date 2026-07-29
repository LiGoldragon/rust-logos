//! Compatibility projection for the fixed generated-module prelude.
//!
//! Production declarations use [`crate::RustLogos`]. The fixed prelude still
//! consists of legacy `EncodedItem::Alias` and `EncodedItem::Use` values, so
//! this deliberately narrow projector keeps that foreign-text seam in the Rust
//! textual-form crate while the prelude is migrated.

use core_logos::{
    Alias, Attribute, ConfigurationPredicate, EncodedItem, PathNode, TypeReference, Use, Visibility,
};
use legacy_name_table::{Identifier, NameResolver};
use proc_macro2::TokenStream;
use quote::quote;

use crate::Error;

/// Formatted Rust produced from the fixed legacy prelude subset.
pub struct RustSource(String);

impl RustSource {
    pub fn project_items<Resolver: NameResolver + ?Sized>(
        items: &[EncodedItem],
        names: &Resolver,
    ) -> Result<Self, Error> {
        let projected = items
            .iter()
            .map(|item| project_item(item, names))
            .collect::<Result<Vec<_>, _>>()?;
        let tokens = quote! { #(#projected)* };
        let file =
            syn::parse2(tokens).map_err(|error| Error::LegacyProjection(error.to_string()))?;
        Ok(Self(prettyplease::unparse(&file)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn project_item<Resolver: NameResolver + ?Sized>(
    item: &EncodedItem,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    match item {
        EncodedItem::Alias(alias) => project_alias(alias, names),
        EncodedItem::Use(use_import) => project_use(use_import, names),
        EncodedItem::Newtype(_)
        | EncodedItem::Struct(_)
        | EncodedItem::Enumeration(_)
        | EncodedItem::ImplBlock(_)
        | EncodedItem::Function(_)
        | EncodedItem::Const(_)
        | EncodedItem::Module(_) => Err(Error::LegacyProjectionUnsupported {
            construct: "an item outside Alias/Use",
        }),
    }
}

fn project_alias<Resolver: NameResolver + ?Sized>(
    alias: &Alias,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    if !alias.generics.parameters.is_empty() {
        return Err(Error::LegacyProjectionUnsupported {
            construct: "a generic fixed-prelude alias",
        });
    }
    let attributes = project_attributes(&alias.attributes, names)?;
    let visibility = project_visibility(&alias.visibility, names)?;
    let name = project_identifier(alias.name, names)?;
    let target = project_type(&alias.target, names)?;
    Ok(quote! {
        #attributes
        #visibility type #name = #target;
    })
}

fn project_use<Resolver: NameResolver + ?Sized>(
    use_import: &Use,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    let attributes = project_attributes(&use_import.attributes, names)?;
    let visibility = project_visibility(&use_import.visibility, names)?;
    let base = project_path(&use_import.base, names)?;
    let group = use_import
        .group
        .iter()
        .copied()
        .map(|identifier| project_identifier(identifier, names))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! {
        #attributes
        #visibility use #base::{#(#group),*};
    })
}

fn project_attributes<Resolver: NameResolver + ?Sized>(
    attributes: &[Attribute],
    names: &Resolver,
) -> Result<TokenStream, Error> {
    let projected = attributes
        .iter()
        .map(|attribute| project_attribute(attribute, names))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { #(#projected)* })
}

fn project_attribute<Resolver: NameResolver + ?Sized>(
    attribute: &Attribute,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    match attribute {
        Attribute::Cfg(ConfigurationPredicate::Feature(feature)) => {
            let feature = names.resolve(*feature)?;
            let feature = feature.as_str();
            Ok(quote! { #[cfg(feature = #feature)] })
        }
        Attribute::ToolPath(path) => {
            let path = project_path(path, names)?;
            Ok(quote! { #[#path] })
        }
        Attribute::Derive(_) | Attribute::Configuration(_) | Attribute::HelperDerive(_) => {
            Err(Error::LegacyProjectionUnsupported {
                construct: "a non-prelude legacy attribute",
            })
        }
    }
}

fn project_visibility<Resolver: NameResolver + ?Sized>(
    visibility: &Visibility,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    match visibility {
        Visibility::Public => Ok(quote! { pub }),
        Visibility::Crate => Ok(quote! { pub(crate) }),
        Visibility::Module(path) => {
            let path = project_path(path, names)?;
            Ok(quote! { pub(in #path) })
        }
        Visibility::Private => Ok(TokenStream::new()),
    }
}

fn project_type<Resolver: NameResolver + ?Sized>(
    type_reference: &TypeReference,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    match type_reference {
        TypeReference::Path(path) => project_path(path, names),
        TypeReference::Application(application) => {
            let head = project_path(&application.head, names)?;
            let arguments = application
                .arguments
                .iter()
                .map(|argument| project_type(argument, names))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quote! { #head<#(#arguments),*> })
        }
        TypeReference::Reference(_)
        | TypeReference::ImplTrait(_)
        | TypeReference::Slice(_)
        | TypeReference::Tuple(_)
        | TypeReference::Lifetime(_) => Err(Error::LegacyProjectionUnsupported {
            construct: "a non-prelude legacy type reference",
        }),
    }
}

fn project_path<Resolver: NameResolver + ?Sized>(
    path: &PathNode,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    let segments = path
        .segments
        .iter()
        .copied()
        .map(|identifier| project_identifier(identifier, names))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { #(#segments)::* })
}

fn project_identifier<Resolver: NameResolver + ?Sized>(
    identifier: Identifier,
    names: &Resolver,
) -> Result<TokenStream, Error> {
    names
        .resolve(identifier)?
        .as_str()
        .parse()
        .map_err(|error: proc_macro2::LexError| Error::LegacyProjection(error.to_string()))
}
