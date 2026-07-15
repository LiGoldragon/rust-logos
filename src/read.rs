//! The reader: Rust text → CoreLogos, through `syn`.
//!
//! Decode never re-implements Rust's grammar — it parses with `syn` and maps the
//! in-subset AST to CoreLogos. The principled subset is exactly what CoreLogos
//! models: the four item kinds (newtype, named-field struct, enum, type alias) over
//! the witnessed attribute/visibility/generic/type vocabulary. Every out-of-subset
//! construct — a trait definition, an impl block, a free function, a `use`
//! re-export, a module, a macro, a union, a const generic, an unmodeled attribute,
//! a reference or tuple type — produces a **typed loud error naming the
//! construct**. The reader never guesses and never skips by default.
//!
//! The verb belongs to the noun being read: [`ReadRust`] is implemented on the
//! `syn` AST nodes, each producing its CoreLogos counterpart and interning names
//! through the threaded [`NameInterner`]. Interning through a transaction (see
//! [`crate::codec`]) is what makes a failed decode leave the NameTable untouched.

use core_logos::{
    Alias, Attribute, ConfigurationAttribute, ConfigurationPredicate, CoreItem, DeriveGroup,
    Enumeration, Field, GenericParameter, Generics, HelperDerive, LifetimeParameter, Newtype,
    PathNode, Struct, TypeApplication, TypeParameter, TypeReference, Variant, VariantPayload,
    Visibility,
};
use name_table::{Name, NameInterner};
use quote::ToTokens;
use syn::parse::Parse;
use syn::punctuated::Punctuated;

use crate::error::Error;

/// The single decode verb, implemented on each in-subset `syn` node, producing its
/// CoreLogos counterpart. Interning is threaded, never held.
pub trait ReadRust {
    /// The CoreLogos node this `syn` node reads into.
    type Logos;

    /// Read this node into CoreLogos, interning any names into the continuous
    /// identifier space, or fail loudly naming the out-of-subset construct.
    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Self::Logos, Error>;
}

/// Read an ordered attribute preamble — the verb on the attribute-vector noun.
trait ReadAttributePreamble {
    /// Read every outer attribute into the modeled vocabulary, in source order.
    fn read_preamble<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Vec<Attribute>, Error>;
}

impl ReadAttributePreamble for [syn::Attribute] {
    fn read_preamble<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Vec<Attribute>, Error> {
        self.iter()
            .map(|attribute| attribute.read(interner))
            .collect()
    }
}

impl ReadRust for syn::Item {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        match self {
            syn::Item::Struct(structure) => structure.read(interner),
            syn::Item::Enum(enumeration) => enumeration.read(interner),
            syn::Item::Type(alias) => alias.read(interner),
            syn::Item::Trait(_) => Err(Error::UnsupportedItem {
                construct: "a trait definition",
            }),
            syn::Item::TraitAlias(_) => Err(Error::UnsupportedItem {
                construct: "a trait alias",
            }),
            syn::Item::Impl(_) => Err(Error::UnsupportedItem {
                construct: "an impl block",
            }),
            syn::Item::Fn(_) => Err(Error::UnsupportedItem {
                construct: "a free function",
            }),
            syn::Item::Use(_) => Err(Error::UnsupportedItem {
                construct: "a use re-export",
            }),
            syn::Item::Mod(_) => Err(Error::UnsupportedItem {
                construct: "a module",
            }),
            syn::Item::Macro(_) => Err(Error::UnsupportedItem {
                construct: "a macro invocation",
            }),
            syn::Item::Union(_) => Err(Error::UnsupportedItem {
                construct: "a union",
            }),
            syn::Item::Const(_) => Err(Error::UnsupportedItem {
                construct: "a const item",
            }),
            syn::Item::Static(_) => Err(Error::UnsupportedItem {
                construct: "a static item",
            }),
            syn::Item::ExternCrate(_) => Err(Error::UnsupportedItem {
                construct: "an extern crate",
            }),
            syn::Item::ForeignMod(_) => Err(Error::UnsupportedItem {
                construct: "an extern block",
            }),
            _ => Err(Error::UnsupportedItem {
                construct: "an unrecognized item",
            }),
        }
    }
}

impl ReadRust for syn::ItemStruct {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        match &self.fields {
            syn::Fields::Named(named) => {
                let generics = self.generics.read(interner)?;
                let fields = named
                    .named
                    .iter()
                    .map(|field| field.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CoreItem::Struct(Struct {
                    visibility,
                    attributes,
                    name,
                    generics,
                    fields,
                }))
            }
            syn::Fields::Unnamed(unnamed) => {
                let count = unnamed.unnamed.len();
                if count != 1 {
                    return Err(Error::MultiFieldTupleStruct { field_count: count });
                }
                if !self.generics.params.is_empty() {
                    return Err(Error::UnsupportedItem {
                        construct: "a generic tuple newtype",
                    });
                }
                if self.generics.where_clause.is_some() {
                    return Err(Error::WhereClause);
                }
                let field = &unnamed.unnamed[0];
                field.reject_field_attributes()?;
                // The Core newtype models no tuple-field visibility (its `wrapped`
                // is a bare type). A `pub`-qualified field (`Name(pub Wrapped)`) is
                // therefore out of subset — dropping the `pub` would be a silent
                // guess, so it fails loudly instead.
                if !matches!(field.vis, syn::Visibility::Inherited) {
                    return Err(Error::UnsupportedItem {
                        construct: "a tuple newtype with a visibility-qualified field",
                    });
                }
                let wrapped = field.ty.read(interner)?;
                Ok(CoreItem::Newtype(Newtype {
                    visibility,
                    attributes,
                    name,
                    wrapped,
                }))
            }
            syn::Fields::Unit => Err(Error::UnitStruct),
        }
    }
}

impl ReadRust for syn::ItemEnum {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        let generics = self.generics.read(interner)?;
        let variants = self
            .variants
            .iter()
            .map(|variant| variant.read(interner))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CoreItem::Enumeration(Enumeration {
            visibility,
            attributes,
            name,
            generics,
            variants,
        }))
    }
}

impl ReadRust for syn::Variant {
    type Logos = Variant;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Variant, Error> {
        if !self.attrs.is_empty() {
            return Err(Error::UnsupportedAttribute {
                rendered: "an attribute on an enum variant".to_string(),
            });
        }
        if self.discriminant.is_some() {
            return Err(Error::UnsupportedItem {
                construct: "an enum variant with an explicit discriminant",
            });
        }
        let name = interner.intern(Name::new(self.ident.to_string()));
        let payload = match &self.fields {
            syn::Fields::Unit => VariantPayload::Unit,
            syn::Fields::Unnamed(unnamed) => {
                let types = unnamed
                    .unnamed
                    .iter()
                    .map(|field| {
                        field.reject_field_attributes()?;
                        field.ty.read(interner)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                VariantPayload::Tuple(types)
            }
            syn::Fields::Named(named) => {
                let fields = named
                    .named
                    .iter()
                    .map(|field| field.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                VariantPayload::Struct(fields)
            }
        };
        Ok(Variant { name, payload })
    }
}

impl ReadRust for syn::ItemType {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        let generics = self.generics.read(interner)?;
        let target = self.ty.read(interner)?;
        Ok(CoreItem::Alias(Alias {
            visibility,
            attributes,
            name,
            generics,
            target,
        }))
    }
}

impl ReadRust for syn::Field {
    type Logos = Field;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Field, Error> {
        self.reject_field_attributes()?;
        let visibility = self.vis.read(interner)?;
        let identifier = self.ident.as_ref().ok_or(Error::UnsupportedType {
            construct: "an unnamed field in a named-field position",
        })?;
        let name = interner.intern(Name::new(identifier.to_string()));
        let type_reference = self.ty.read(interner)?;
        Ok(Field {
            visibility,
            name,
            type_reference,
        })
    }
}

/// Field-attribute rejection — a verb on the `syn::Field` noun. CoreLogos fields
/// carry no attributes, so any field attribute is out of subset and loud.
trait RejectFieldAttributes {
    fn reject_field_attributes(&self) -> Result<(), Error>;
}

impl RejectFieldAttributes for syn::Field {
    fn reject_field_attributes(&self) -> Result<(), Error> {
        if self.attrs.is_empty() {
            return Ok(());
        }
        Err(Error::UnsupportedAttribute {
            rendered: "an attribute on a struct field".to_string(),
        })
    }
}

impl ReadRust for syn::Visibility {
    type Logos = Visibility;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Visibility, Error> {
        match self {
            syn::Visibility::Public(_) => Ok(Visibility::Public),
            syn::Visibility::Inherited => Ok(Visibility::Private),
            syn::Visibility::Restricted(restricted) => {
                let is_bare_crate =
                    restricted.in_token.is_none() && restricted.path.is_ident("crate");
                if is_bare_crate {
                    return Ok(Visibility::Crate);
                }
                if restricted.in_token.is_some() {
                    return Ok(Visibility::Module(restricted.path.read(interner)?));
                }
                Err(Error::UnsupportedVisibility {
                    rendered: restricted.to_token_stream().to_string(),
                })
            }
        }
    }
}

impl ReadRust for syn::Attribute {
    type Logos = Attribute;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Attribute, Error> {
        if !matches!(self.style, syn::AttrStyle::Outer) {
            return Err(Error::UnsupportedAttribute {
                rendered: "an inner attribute".to_string(),
            });
        }
        self.meta.read(interner)
    }
}

impl ReadRust for syn::Meta {
    type Logos = Attribute;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Attribute, Error> {
        match self {
            syn::Meta::Path(path) => Ok(Attribute::ToolPath(path.read(interner)?)),
            syn::Meta::List(list) => list.read(interner),
            syn::Meta::NameValue(name_value) => Err(Error::UnsupportedAttribute {
                rendered: name_value.to_token_stream().to_string(),
            }),
        }
    }
}

impl ReadRust for syn::MetaList {
    type Logos = Attribute;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Attribute, Error> {
        if self.path.is_ident("derive") {
            return Ok(Attribute::Derive(self.read_derive_group(interner)?));
        }
        if self.path.is_ident("cfg_attr") {
            return self.read_configuration(interner);
        }
        // A single-segment namespaced helper whose sole argument is a `derive(…)`:
        // `#[rkyv(derive(PartialOrd, Ord))]`.
        if let Some(helper) = self.read_helper_derive(interner)? {
            return Ok(Attribute::HelperDerive(helper));
        }
        Err(Error::UnsupportedAttribute {
            rendered: self.to_token_stream().to_string(),
        })
    }
}

/// The three list-attribute readers — verbs on the `syn::MetaList` noun.
trait ReadListAttribute {
    fn read_derive_group<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<DeriveGroup, Error>;

    fn read_configuration<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Attribute, Error>;

    fn read_helper_derive<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Option<HelperDerive>, Error>;
}

impl ReadListAttribute for syn::MetaList {
    fn read_derive_group<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<DeriveGroup, Error> {
        let paths = self
            .parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
            .map_err(|error| Error::Parse(error.to_string()))?;
        let paths = paths
            .iter()
            .map(|path| path.read(interner))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DeriveGroup { paths })
    }

    fn read_configuration<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Attribute, Error> {
        let arguments = self
            .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            .map_err(|error| Error::Parse(error.to_string()))?;
        if arguments.len() != 2 {
            return Err(Error::UnsupportedAttribute {
                rendered: self.to_token_stream().to_string(),
            });
        }
        let predicate = arguments[0].read_configuration_predicate(interner)?;
        let inner = Box::new(arguments[1].read(interner)?);
        Ok(Attribute::Configuration(ConfigurationAttribute {
            predicate,
            inner,
        }))
    }

    fn read_helper_derive<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Option<HelperDerive>, Error> {
        let Ok(argument) = self.parse_args_with(syn::Meta::parse) else {
            return Ok(None);
        };
        let syn::Meta::List(inner) = argument else {
            return Ok(None);
        };
        if !inner.path.is_ident("derive") {
            return Ok(None);
        }
        let derived = inner.read_derive_group(interner)?;
        Ok(Some(HelperDerive {
            path: self.path.read(interner)?,
            derived,
        }))
    }
}

/// Reading a `cfg_attr` predicate — a verb on the `syn::Meta` noun that carries it.
trait ReadConfigurationPredicate {
    fn read_configuration_predicate<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<ConfigurationPredicate, Error>;
}

impl ReadConfigurationPredicate for syn::Meta {
    fn read_configuration_predicate<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<ConfigurationPredicate, Error> {
        if let syn::Meta::NameValue(name_value) = self {
            if name_value.path.is_ident("feature") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(feature),
                    ..
                }) = &name_value.value
                {
                    let identifier = interner.intern(Name::new(feature.value()));
                    return Ok(ConfigurationPredicate::Feature(identifier));
                }
            }
        }
        Err(Error::UnsupportedConfigurationPredicate {
            rendered: self.to_token_stream().to_string(),
        })
    }
}

impl ReadRust for syn::Path {
    type Logos = PathNode;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<PathNode, Error> {
        let segments = self
            .segments
            .iter()
            .map(|segment| {
                if !matches!(segment.arguments, syn::PathArguments::None) {
                    return Err(Error::UnsupportedType {
                        construct: "a generic argument on a non-type path",
                    });
                }
                Ok(interner.intern(Name::new(segment.ident.to_string())))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PathNode { segments })
    }
}

impl ReadRust for syn::Type {
    type Logos = TypeReference;

    fn read<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<TypeReference, Error> {
        let syn::Type::Path(type_path) = self else {
            return Err(Error::UnsupportedType {
                construct: self.unsupported_type_kind(),
            });
        };
        if type_path.qself.is_some() {
            return Err(Error::UnsupportedType {
                construct: "a qualified-self type",
            });
        }
        type_path.path.read_type_reference(interner)
    }
}

/// Reading a `syn::Path` in *type position*, where the final segment may carry a
/// generic argument list — a verb on the path noun distinct from the bare-path
/// read used by attributes and bounds.
trait ReadTypeReference {
    fn read_type_reference<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<TypeReference, Error>;
}

impl ReadTypeReference for syn::Path {
    fn read_type_reference<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<TypeReference, Error> {
        let last = self.segments.len().saturating_sub(1);
        let mut head_segments = Vec::with_capacity(self.segments.len());
        let mut arguments: Option<Vec<TypeReference>> = None;
        for (index, segment) in self.segments.iter().enumerate() {
            head_segments.push(interner.intern(Name::new(segment.ident.to_string())));
            match &segment.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(bracketed) if index == last => {
                    let mut collected = Vec::with_capacity(bracketed.args.len());
                    for argument in &bracketed.args {
                        let syn::GenericArgument::Type(inner) = argument else {
                            return Err(Error::UnsupportedType {
                                construct: "a non-type generic argument",
                            });
                        };
                        collected.push(inner.read(interner)?);
                    }
                    arguments = Some(collected);
                }
                syn::PathArguments::AngleBracketed(_) => {
                    return Err(Error::UnsupportedType {
                        construct: "a generic argument on a non-final path segment",
                    });
                }
                syn::PathArguments::Parenthesized(_) => {
                    return Err(Error::UnsupportedType {
                        construct: "a parenthesized (Fn) generic argument list",
                    });
                }
            }
        }
        let head = PathNode {
            segments: head_segments,
        };
        Ok(match arguments {
            None => TypeReference::Path(head),
            Some(arguments) => TypeReference::Application(TypeApplication { head, arguments }),
        })
    }
}

/// Name the kind of an out-of-subset `syn::Type` for the loud error — a verb on
/// the type noun.
trait UnsupportedTypeKind {
    fn unsupported_type_kind(&self) -> &'static str;
}

impl UnsupportedTypeKind for syn::Type {
    fn unsupported_type_kind(&self) -> &'static str {
        match self {
            syn::Type::Reference(_) => "a reference type",
            syn::Type::Tuple(_) => "a tuple type",
            syn::Type::TraitObject(_) => "a trait-object type",
            syn::Type::ImplTrait(_) => "an impl-Trait type",
            syn::Type::Slice(_) => "a slice type",
            syn::Type::Array(_) => "an array type",
            syn::Type::Ptr(_) => "a raw-pointer type",
            syn::Type::BareFn(_) => "a bare-function type",
            syn::Type::Paren(_) => "a parenthesized type",
            syn::Type::Group(_) => "a grouped type",
            syn::Type::Never(_) => "the never type",
            syn::Type::Infer(_) => "an inferred type",
            syn::Type::Macro(_) => "a macro type",
            _ => "an unrecognized type",
        }
    }
}

impl ReadRust for syn::Generics {
    type Logos = Generics;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Generics, Error> {
        if self.where_clause.is_some() {
            return Err(Error::WhereClause);
        }
        let parameters = self
            .params
            .iter()
            .map(|parameter| parameter.read(interner))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Generics { parameters })
    }
}

impl ReadRust for syn::GenericParam {
    type Logos = GenericParameter;

    fn read<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<GenericParameter, Error> {
        match self {
            syn::GenericParam::Type(parameter) => {
                let name = interner.intern(Name::new(parameter.ident.to_string()));
                let mut bounds = Vec::with_capacity(parameter.bounds.len());
                for bound in &parameter.bounds {
                    let syn::TypeParamBound::Trait(trait_bound) = bound else {
                        return Err(Error::UnsupportedGenericBound {
                            construct: "a non-trait generic bound (lifetime or precise capture)",
                        });
                    };
                    if !matches!(trait_bound.modifier, syn::TraitBoundModifier::None) {
                        return Err(Error::UnsupportedGenericBound {
                            construct: "a `?Sized`-relaxed trait bound",
                        });
                    }
                    if trait_bound.lifetimes.is_some() {
                        return Err(Error::UnsupportedGenericBound {
                            construct: "a higher-ranked trait bound",
                        });
                    }
                    bounds.push(trait_bound.path.read(interner)?);
                }
                Ok(GenericParameter::Type(TypeParameter { name, bounds }))
            }
            syn::GenericParam::Lifetime(parameter) => {
                if !parameter.bounds.is_empty() {
                    return Err(Error::UnsupportedGenericBound {
                        construct: "a bounded lifetime parameter",
                    });
                }
                let name = interner.intern(Name::new(parameter.lifetime.ident.to_string()));
                Ok(GenericParameter::Lifetime(LifetimeParameter { name }))
            }
            syn::GenericParam::Const(_) => Err(Error::UnsupportedGenericParameter {
                construct: "a const generic parameter",
            }),
        }
    }
}
