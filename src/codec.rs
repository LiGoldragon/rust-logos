//! Rust emission from opaque authority-issued Logos names.

use core_logos::{
    WholeLogos, WholeLogosAssociatedTypeBinding, WholeLogosEnumeration, WholeLogosItem,
    WholeLogosNewtype, WholeLogosStruct, WholeLogosTable, WholeLogosTraitDef, WholeLogosTraitImpl,
    WholeLogosTraitMethod, WholeLogosTypeAttributes, WholeLogosTypeReference,
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
///
/// When `refusal_display_trait` is set, the emitter identifies trait
/// implementations matching that encoded identity and emits `Display` and
/// `std::error::Error` implementations for the implementing types.
pub struct RustLogos {
    refusal_display_trait: Option<EncodedName>,
}

impl RustLogos {
    /// Construct the Rust emitter with no role-specific behavior.
    pub const fn new() -> Self {
        Self {
            refusal_display_trait: None,
        }
    }

    /// Configure the emitter to generate `Display` and `std::error::Error`
    /// implementations for types that implement the given trait identity.
    ///
    /// The Refusal role trait requires `std::error::Error`, which requires
    /// `Debug + Display`. The emitter derives `Debug` through the Wire
    /// attribute preamble; this method adds the remaining `Display` and
    /// `Error` implementations.
    pub const fn with_refusal_display_trait(mut self, trait_identity: EncodedName) -> Self {
        self.refusal_display_trait = Some(trait_identity);
        self
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
        let mut sections: Vec<String> = logos
            .items()
            .iter()
            .map(|item| self.item(item, names, paths))
            .collect::<Result<Vec<_>, _>>()?;

        // Emit Display and std::error::Error for types implementing the
        // Refusal role trait, identified by the configured trait identity.
        if let Some(refusal_trait) = &self.refusal_display_trait {
            for item in logos.items() {
                if let WholeLogosItem::TraitImpl(trait_impl) = item {
                    if let WholeLogosTypeReference::Identity(implemented) =
                        trait_impl.implemented_trait()
                    {
                        if implemented == refusal_trait {
                            if let WholeLogosTypeReference::Identity(implementing) =
                                trait_impl.implementing_type()
                            {
                                let type_name = self.name(implementing, names)?;
                                sections.push(format!(
                                    "impl std::fmt::Display for {type_name} {{\n    \
                                     fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        \
                                     std::fmt::Debug::fmt(self, formatter)\n    \
                                     }}\n\
                                     }}"
                                ));
                                sections.push(format!(
                                    "impl std::error::Error for {type_name} {{}}"
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(sections.join("\n\n"))
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
            "{}{}struct {}({}{});",
            derive_preamble(value.attributes()),
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
            "{}{}struct {}({});",
            derive_preamble(value.attributes()),
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
            "{}{}enum {} {{\n{}\n}}",
            derive_preamble(value.attributes()),
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

/// The canonical derive preamble for each attribute policy.
///
/// `Plain` emits nothing (Nexus types, role-free vocabulary).
/// `Wire` and `Stored` emit the rkyv serialization surface plus `Clone`,
/// `Debug`, `PartialEq`, and `Eq`. The rkyv bounds annotations are
/// unconditional: they provide the explicit bytecheck and serializer
/// bounds that types containing `Vec<T>` need for rkyv 0.8 with the
/// `bytecheck` feature, and are no-ops on types without allocations.
fn derive_preamble(attributes: WholeLogosTypeAttributes) -> &'static str {
    match attributes {
        WholeLogosTypeAttributes::Plain => "",
        WholeLogosTypeAttributes::Wire | WholeLogosTypeAttributes::Stored => {
            "#[derive(Clone, Debug, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]\n\
             #[rkyv(\n    \
             serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),\n    \
             deserialize_bounds(__D::Error: rkyv::rancor::Source),\n    \
             bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),\n\
             )]\n"
        }
    }
}

fn type_translation(spelling: String) -> String {
    match spelling.as_str() {
        "Vector" => "Vec".to_owned(),
        "Unit" => "()".to_owned(),
        "Boolean" => "bool".to_owned(),
        // [assumption] Integer -> i64 pending psyche ruling on the canonical
        // Rust mapping for the Ethos Integer builtin. The old pipeline used
        // u64 in some consumers and i64 in others; i64 is the signed default.
        "Integer" => "i64".to_owned(),
        _ => spelling,
    }
}
