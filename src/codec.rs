//! Rust emission from opaque authority-issued Logos names.

use core_logos::{
    WholeLogos, WholeLogosAssociatedTypeBinding, WholeLogosEnumeration, WholeLogosItem,
    WholeLogosNewtype, WholeLogosStruct, WholeLogosTable,
    WholeLogosTraitDef, WholeLogosTraitImpl, WholeLogosTraitMethod, WholeLogosTypeReference,
    WholeLogosVariantPayload, WholeLogosVisibility,
};
use name_table::{EncodedName, NameView};

use crate::Error;

/// Lookup of caller-owned Rust type paths for imported references.
pub trait RustTypePathResolver {
    /// Resolve one opaque encoded name to its canonical external Rust path.
    fn resolve_type_path(&self, encoded_name: &EncodedName) -> Option<&RustTypePath>;
}

/// One validated canonical external Rust path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTypePath {
    segments: Vec<String>,
}

impl RustTypePath {
    /// Validate a non-empty sequence of Rust path segments.
    pub fn try_new(segments: Vec<String>) -> Result<Self, Error> {
        if segments.is_empty()
            || segments.iter().any(|segment| {
                segment.is_empty()
                    || segment.contains("::")
                    || syn::parse_str::<syn::PathSegment>(segment).is_err()
            })
        {
            return Err(Error::InvalidExternalRustTypePath {
                path: segments.join("::"),
            });
        }
        Ok(Self { segments })
    }

    /// Canonical path segments after validation.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    fn render(&self) -> String {
        self.segments.join("::")
    }
}

struct NoRustTypePaths;

impl RustTypePathResolver for NoRustTypePaths {
    fn resolve_type_path(&self, _: &EncodedName) -> Option<&RustTypePath> {
        None
    }
}

/// The three named roles emitted for an Interface declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterfaceRustRoleIds {
    input: EncodedName,
    output: EncodedName,
    refusal: EncodedName,
}

impl InterfaceRustRoleIds {
    /// Construct distinct authority-issued role names.
    pub fn new(
        input: EncodedName,
        output: EncodedName,
        refusal: EncodedName,
    ) -> Result<Self, Error> {
        if input == output || input == refusal || output == refusal {
            return Err(Error::DuplicateInterfaceRoleIdentity);
        }
        Ok(Self {
            input,
            output,
            refusal,
        })
    }

    /// Input role name.
    pub const fn input(&self) -> &EncodedName {
        &self.input
    }
    /// Output role name.
    pub const fn output(&self) -> &EncodedName {
        &self.output
    }
    /// Refusal role name.
    pub const fn refusal(&self) -> &EncodedName {
        &self.refusal
    }
}

/// Interface-specific Rust assembly over already lowered Whole Logos.
pub trait InterfaceRustEmission {
    /// Emit role memberships using a read-only authority name view.
    fn emit_interface<View: NameView + ?Sized>(
        &self,
        logos: &WholeLogos,
        names: &View,
        roles: &InterfaceRustRoleIds,
    ) -> Result<String, Error>;
}

/// Rust textual projection for a lowered Whole Logos carrier.
#[derive(Default)]
pub struct RustLogos;

impl RustLogos {
    /// Construct the stateless Rust emitter.
    pub const fn new() -> Self {
        Self
    }

    /// Emit Rust solely through the supplied read-only authority name view.
    pub fn emit<View: NameView + ?Sized>(
        &self,
        logos: &WholeLogos,
        names: &View,
    ) -> Result<String, Error> {
        self.emit_with_type_paths(logos, names, &NoRustTypePaths)
    }

    /// Emit Rust while resolving explicitly supplied external type paths.
    pub fn emit_with_type_paths<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        logos: &WholeLogos,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        logos
            .items()
            .iter()
            .map(|item| self.item(item, names, paths))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.join("\n\n"))
    }

    fn item<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        item: &WholeLogosItem,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        match item {
            WholeLogosItem::Newtype(value) => self.newtype(value, names, paths),
            WholeLogosItem::Struct(value) => self.structure(value, names, paths),
            WholeLogosItem::Enumeration(value) => self.enumeration(value, names, paths),
            WholeLogosItem::TraitDef(value) => self.trait_definition(value, names, paths),
            WholeLogosItem::TraitImpl(value) => self.trait_implementation(value, names, paths),
            WholeLogosItem::Table(value) => self.table(value, names, paths),
        }
    }

    fn newtype<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosNewtype,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        Ok(format!(
            "{}struct {}({}{});",
            visibility(value.visibility()),
            self.name(value.name(), names)?,
            visibility(value.wrapped_visibility()),
            self.reference(value.wrapped(), names, paths)?
        ))
    }

    fn structure<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosStruct,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        Ok(format!(
            "{}struct {}({});",
            visibility(value.visibility()),
            self.name(value.name(), names)?,
            self.references(value.fields(), names, paths)?
        ))
    }

    fn enumeration<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosEnumeration,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        let variants = value
            .variants()
            .iter()
            .map(|variant| {
                let name = self.name(variant.name(), names)?;
                let payload = match variant.payload() {
                    WholeLogosVariantPayload::Unit => String::new(),
                    WholeLogosVariantPayload::Tuple(fields) => {
                        format!("({})", self.references(fields.fields(), names, paths)?)
                    }
                };
                Ok(format!("    {name}{payload},"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(format!(
            "{}enum {} {{\n{}\n}}",
            visibility(value.visibility()),
            self.name(value.name(), names)?,
            variants.join("\n")
        ))
    }

    fn trait_definition<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosTraitDef,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        let methods = value
            .methods()
            .iter()
            .map(|method| self.method(method, names, paths))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "{}trait {} {{\n{}\n}}",
            visibility(value.visibility()),
            self.name(value.name(), names)?,
            methods.join("\n")
        ))
    }

    fn method<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosTraitMethod,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        let parameters = value
            .parameters()
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                Ok(format!(
                    "argument_{index}: {}",
                    self.reference(parameter, names, paths)?
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(format!(
            "    fn {}(&self, {}) -> {};",
            self.name(value.name(), names)?,
            parameters.join(", "),
            self.reference(value.return_type(), names, paths)?
        ))
    }

    fn trait_implementation<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosTraitImpl,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        let bindings = value
            .associated_type_bindings()
            .iter()
            .map(|binding| self.binding(binding, names, paths))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "impl {} for {} {{\n{}\n}}",
            self.reference(value.implemented_trait(), names, paths)?,
            self.reference(value.implementing_type(), names, paths)?,
            bindings.join("\n")
        ))
    }

    fn binding<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosAssociatedTypeBinding,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        Ok(format!(
            "    type {} = {};",
            self.name(value.name(), names)?,
            self.reference(value.value(), names, paths)?
        ))
    }

    fn table<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosTable,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        Ok(format!(
            "type {} = ({}, {});",
            self.name(value.name(), names)?,
            self.reference(value.record(), names, paths)?,
            self.reference(value.key(), names, paths)?
        ))
    }

    fn references<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        values: &[WholeLogosTypeReference],
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        values
            .iter()
            .map(|value| self.reference(value, names, paths))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.join(", "))
    }

    fn reference<View: NameView + ?Sized, Paths: RustTypePathResolver + ?Sized>(
        &self,
        value: &WholeLogosTypeReference,
        names: &View,
        paths: &Paths,
    ) -> Result<String, Error> {
        match value {
            WholeLogosTypeReference::Identity(name) | WholeLogosTypeReference::Parameter(name) => {
                paths
                    .resolve_type_path(name)
                    .map(RustTypePath::render)
                    .map(Ok)
                    .unwrap_or_else(|| self.name(name, names).map(type_translation))
            }
            WholeLogosTypeReference::Application(application) => {
                let head = match paths.resolve_type_path(application.head()) {
                    Some(resolved) => resolved.render(),
                    None => type_translation(self.name(application.head(), names)?),
                };
                Ok(format!(
                    "{}<{}>",
                    head,
                    self.references(application.arguments(), names, paths)?
                ))
            }
        }
    }

    fn name<View: NameView + ?Sized>(
        &self,
        encoded_name: &EncodedName,
        names: &View,
    ) -> Result<String, Error> {
        let spelling = names
            .textual_metadata(encoded_name)
            .ok_or(Error::MissingName {
                encoded_name: *encoded_name,
            })?
            .textual_name()
            .as_str();
        if syn::parse_str::<syn::Ident>(spelling).is_err() {
            return Err(Error::InvalidRustIdentifier {
                token: spelling.to_owned(),
            });
        }
        Ok(spelling.to_owned())
    }
}

impl InterfaceRustEmission for RustLogos {
    fn emit_interface<View: NameView + ?Sized>(
        &self,
        logos: &WholeLogos,
        names: &View,
        roles: &InterfaceRustRoleIds,
    ) -> Result<String, Error> {
        let base = self.emit(logos, names)?;
        Ok(format!(
            "{base}\n\ntrait {} {{}}\ntrait {} {{}}\ntrait {} {{}}",
            self.name(roles.input(), names)?,
            self.name(roles.output(), names)?,
            self.name(roles.refusal(), names)?
        ))
    }
}

fn visibility(value: &WholeLogosVisibility) -> &'static str {
    match value {
        WholeLogosVisibility::Public => "pub ",
        WholeLogosVisibility::Private => "",
    }
}
fn type_translation(spelling: String) -> String {
    if spelling == "Vector" {
        "Vec".to_owned()
    } else {
        spelling
    }
}
