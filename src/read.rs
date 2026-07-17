//! The reader: Rust text → CoreLogos, through `syn`.
//!
//! Decode never re-implements Rust's grammar — it parses with `syn` and maps the
//! in-subset AST to CoreLogos. The principled subset is exactly what CoreLogos
//! models: the data item kinds (newtype, named-field struct, enum, type alias) plus
//! impl blocks and functions whose bodies are the closed Tier-1 expression
//! vocabulary, associated types and consts in impls, `const` items and const-carrying
//! inline modules, over the witnessed attribute/visibility/generic/type vocabulary,
//! plus the brace-group `use` import that heads the module prelude. Every
//! out-of-subset construct — a trait definition, a non-brace-group `use` shape, a
//! non-const module, a macro, a union, a const generic, an unmodeled attribute, a
//! wire-data reference or tuple type, or a Tier-2 body statement/expression/pattern —
//! produces a **typed loud error naming the construct**. The reader never guesses
//! and never skips by default.
//!
//! The verb belongs to the noun being read: [`ReadRust`] is implemented on the
//! `syn` AST nodes, each producing its CoreLogos counterpart and interning names
//! through the threaded [`NameInterner`]. Interning through a transaction (see
//! [`crate::codec`]) is what makes a failed decode leave the NameTable untouched.

use core_logos::{
    Alias, ArrayExpression, AssociatedType, Attribute, Block, Call, Callee, ConfigurationAttribute,
    ConfigurationPredicate, Const, CoreItem, DeriveGroup, Enumeration, Expression, Field, Function,
    GenericParameter, Generics, HelperDerive, ImplBlock, ImplItem, ImplTraitType, IntegerLiteral,
    IntegerRepresentation, LifetimeParameter, Match, MatchArm, MethodCall, Module, Newtype,
    Parameter, PathNode, Pattern, PatternElement, QualifiedPath, Receiver, ReferenceExpression,
    ReferenceMutability, ReferenceType, SliceType, Struct, TupleFieldAccess, TupleVariantPattern,
    TypeApplication, TypeParameter, TypeReference, Use, Variant, VariantPayload, Visibility,
};
use name_table::{Identifier, Name, NameInterner};
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
            syn::Item::Impl(impl_block) => impl_block.read(interner),
            syn::Item::Fn(function) => function.read(interner),
            syn::Item::Trait(_) => Err(Error::UnsupportedItem {
                construct: "a trait definition",
            }),
            syn::Item::TraitAlias(_) => Err(Error::UnsupportedItem {
                construct: "a trait alias",
            }),
            syn::Item::Use(use_import) => use_import.read(interner),
            syn::Item::Mod(module) => module.read(interner),
            syn::Item::Macro(_) => Err(Error::UnsupportedItem {
                construct: "a macro invocation",
            }),
            syn::Item::Union(_) => Err(Error::UnsupportedItem {
                construct: "a union",
            }),
            syn::Item::Const(const_item) => const_item.read(interner),
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
                // The tuple field carries its own visibility as stored data
                // (layout 4): `Name(pub Wrapped)` reads `Public`, the bare
                // `Name(Wrapped)` reads `Private` (which projects to nothing), so
                // the `pub`-field form round-trips instead of failing out of subset.
                let wrapped_visibility = field.vis.read(interner)?;
                let wrapped = field.ty.read(interner)?;
                Ok(CoreItem::Newtype(Newtype {
                    visibility,
                    attributes,
                    name,
                    wrapped_visibility,
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

impl ReadRust for syn::ItemUse {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        if self.leading_colon.is_some() {
            return Err(Error::UnsupportedUseTree {
                construct: "a leading-colon absolute import",
            });
        }
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        // Walk the `<base>::…::{<group>}` chain: the leading path segments are the
        // base, and the terminal brace group is the imported leaf names. A bare
        // name, a glob, or a rename is out of subset.
        let mut base = Vec::new();
        let group = self.tree.read_use_group(interner, &mut base)?;
        Ok(CoreItem::Use(Use {
            visibility,
            attributes,
            base: PathNode { segments: base },
            group,
        }))
    }
}

impl ReadRust for syn::ItemMod {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        if self.unsafety.is_some() {
            return Err(Error::UnsupportedItem {
                construct: "an unsafe module",
            });
        }
        // Only an inline module (`mod name { … }`) is modeled; a file module
        // (`mod name;`) has no items to carry.
        let Some((_brace, items)) = &self.content else {
            return Err(Error::UnsupportedItem {
                construct: "a module without an inline body",
            });
        };
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        // The witnessed module is the `short_header` const module — its members are
        // consts. A module carrying an enum, impl, or other item is a broader growth
        // point (it re-exposes the derive-group trailing-comma layout that a
        // context-free `DeriveGroup` does not yet carry faithfully), so it stays out
        // of subset and fails loudly rather than round-tripping wrong.
        let items = items
            .iter()
            .map(|item| match item {
                syn::Item::Const(const_item) => const_item.read(interner),
                _ => Err(Error::UnsupportedItem {
                    construct: "a module with a non-const item",
                }),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CoreItem::Module(Module {
            visibility,
            attributes,
            name,
            items,
        }))
    }
}

impl ReadRust for syn::ItemConst {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        Ok(CoreItem::Const(self.read_const(interner)?))
    }
}

/// Reading a `const` declaration into a [`Const`] — one verb shared by the top-level
/// const item, a module const, and an impl-block associated const, because they are
/// the same node. A const generic parameter or where clause is out of the modeled
/// shape.
trait ReadConst {
    fn read_const<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Const, Error>;
}

impl ReadConst for syn::ItemConst {
    fn read_const<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Const, Error> {
        if !self.generics.params.is_empty() || self.generics.where_clause.is_some() {
            return Err(Error::UnsupportedItem {
                construct: "a generic const",
            });
        }
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        let type_reference = self.ty.read_signature_type(interner)?;
        let value = self.expr.read(interner)?;
        Ok(Const {
            visibility,
            attributes,
            name,
            type_reference,
            value,
        })
    }
}

impl ReadConst for syn::ImplItemConst {
    fn read_const<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Const, Error> {
        if self.defaultness.is_some() {
            return Err(Error::UnsupportedImplItem {
                construct: "a default associated const",
            });
        }
        if !self.generics.params.is_empty() || self.generics.where_clause.is_some() {
            return Err(Error::UnsupportedImplItem {
                construct: "a generic associated const",
            });
        }
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let name = interner.intern(Name::new(self.ident.to_string()));
        let type_reference = self.ty.read_signature_type(interner)?;
        let value = self.expr.read(interner)?;
        Ok(Const {
            visibility,
            attributes,
            name,
            type_reference,
            value,
        })
    }
}

/// Reading the `<base>::{<group>}` shape of a use tree — a verb on the
/// `syn::UseTree` noun. The leading path segments accumulate into `base`; the
/// terminal brace group of plain names is returned. Any other shape (a bare name, a
/// glob, a rename, a nested group) is out of subset and loud.
trait ReadUseGroup {
    fn read_use_group<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
        base: &mut Vec<Identifier>,
    ) -> Result<Vec<Identifier>, Error>;
}

impl ReadUseGroup for syn::UseTree {
    fn read_use_group<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
        base: &mut Vec<Identifier>,
    ) -> Result<Vec<Identifier>, Error> {
        match self {
            syn::UseTree::Path(path) => {
                base.push(interner.intern(Name::new(path.ident.to_string())));
                path.tree.read_use_group(interner, base)
            }
            syn::UseTree::Group(group) => {
                let mut names = Vec::with_capacity(group.items.len());
                for item in &group.items {
                    let syn::UseTree::Name(name) = item else {
                        return Err(Error::UnsupportedUseTree {
                            construct: "a use group element that is not a plain name",
                        });
                    };
                    names.push(interner.intern(Name::new(name.ident.to_string())));
                }
                Ok(names)
            }
            syn::UseTree::Name(_) => Err(Error::UnsupportedUseTree {
                construct: "a bare `use path::Name;` import (only the brace-group form is modeled)",
            }),
            syn::UseTree::Rename(_) => Err(Error::UnsupportedUseTree {
                construct: "a renamed `as` import",
            }),
            syn::UseTree::Glob(_) => Err(Error::UnsupportedUseTree {
                construct: "a glob `*` import",
            }),
        }
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
        if self.path.is_ident("cfg") {
            return self.read_cfg(interner);
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

    fn read_cfg<Interner: NameInterner>(&self, interner: &mut Interner)
    -> Result<Attribute, Error>;

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

    fn read_cfg<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Attribute, Error> {
        let arguments = self
            .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            .map_err(|error| Error::Parse(error.to_string()))?;
        if arguments.len() != 1 {
            return Err(Error::UnsupportedAttribute {
                rendered: self.to_token_stream().to_string(),
            });
        }
        let predicate = arguments[0].read_configuration_predicate(interner)?;
        Ok(Attribute::Cfg(predicate))
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
                        match argument {
                            syn::GenericArgument::Type(inner) => {
                                collected.push(inner.read(interner)?);
                            }
                            // A lifetime argument (`Formatter<'_>`, an explicit
                            // `'static`): stored as the interned lifetime name.
                            syn::GenericArgument::Lifetime(lifetime) => {
                                let name = interner.intern(Name::new(lifetime.ident.to_string()));
                                collected.push(TypeReference::Lifetime(name));
                            }
                            _ => {
                                return Err(Error::UnsupportedType {
                                    construct: "a non-type, non-lifetime generic argument",
                                });
                            }
                        }
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

// ---------------------------------------------------------------------------
// Impl blocks, functions, and the Tier-1 body vocabulary.
//
// The verb still belongs to the syn node being read. An impl block reads its
// attributes, generics, optional implemented trait, self type, and member
// functions; a function reads its signature parts and single-tail-expression body;
// and the body reads into the closed Tier-1 expression/pattern vocabulary. Every
// out-of-vocabulary construct is a typed loud error that names it, and the whole
// item fails atomically so a class-B body leaks no name.
// ---------------------------------------------------------------------------

impl ReadRust for syn::ItemImpl {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        if self.unsafety.is_some() {
            return Err(Error::UnsupportedItem {
                construct: "an unsafe impl",
            });
        }
        if self.defaultness.is_some() {
            return Err(Error::UnsupportedItem {
                construct: "a default impl",
            });
        }
        let attributes = self.attrs.read_preamble(interner)?;
        let generics = self.generics.read(interner)?;
        let implemented_trait = match &self.trait_ {
            None => None,
            Some((bang, path, _for)) => {
                if bang.is_some() {
                    return Err(Error::UnsupportedItem {
                        construct: "a negative impl",
                    });
                }
                Some(path.read_type_reference(interner)?)
            }
        };
        let self_type = self.self_ty.read(interner)?;
        let items = self
            .items
            .iter()
            .map(|item| item.read(interner))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CoreItem::ImplBlock(ImplBlock {
            attributes,
            generics,
            implemented_trait,
            self_type,
            items,
        }))
    }
}

impl ReadRust for syn::ImplItem {
    type Logos = ImplItem;

    /// One associated item of an impl block, dispatched by kind. A method, an
    /// associated type, and an associated const are modeled; a macro or an
    /// unrecognized member is out of subset.
    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<ImplItem, Error> {
        match self {
            syn::ImplItem::Fn(function) => Ok(ImplItem::Method(function.read(interner)?)),
            syn::ImplItem::Type(associated_type) => {
                Ok(ImplItem::AssociatedType(associated_type.read(interner)?))
            }
            syn::ImplItem::Const(associated_const) => Ok(ImplItem::AssociatedConst(
                associated_const.read_const(interner)?,
            )),
            syn::ImplItem::Macro(_) => Err(Error::UnsupportedImplItem {
                construct: "a macro invocation",
            }),
            _ => Err(Error::UnsupportedImplItem {
                construct: "an unrecognized associated item",
            }),
        }
    }
}

impl ReadRust for syn::ImplItemType {
    type Logos = AssociatedType;

    /// An associated type binding `type <name> = <value>;`. Bounds, generics, and a
    /// where clause are out of the modeled equality-binding shape.
    fn read<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<AssociatedType, Error> {
        if self.defaultness.is_some() {
            return Err(Error::UnsupportedImplItem {
                construct: "a default associated type",
            });
        }
        if !self.generics.params.is_empty() || self.generics.where_clause.is_some() {
            return Err(Error::UnsupportedImplItem {
                construct: "a generic associated type",
            });
        }
        if !self.attrs.is_empty() {
            return Err(Error::UnsupportedAttribute {
                rendered: "an attribute on an associated type".to_string(),
            });
        }
        let name = interner.intern(Name::new(self.ident.to_string()));
        let value = self.ty.read_signature_type(interner)?;
        Ok(AssociatedType { name, value })
    }
}

impl ReadRust for syn::ImplItemFn {
    type Logos = Function;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Function, Error> {
        if self.defaultness.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a default method",
            });
        }
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let parts = self.sig.read_function_signature(interner)?;
        let body = self.block.read(interner)?;
        Ok(parts.into_function(attributes, visibility, body))
    }
}

impl ReadRust for syn::ItemFn {
    type Logos = CoreItem;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<CoreItem, Error> {
        let attributes = self.attrs.read_preamble(interner)?;
        let visibility = self.vis.read(interner)?;
        let parts = self.sig.read_function_signature(interner)?;
        let body = self.block.read(interner)?;
        Ok(CoreItem::Function(
            parts.into_function(attributes, visibility, body),
        ))
    }
}

/// The Core parts of a read function signature, assembled into a [`Function`] with
/// the attributes, visibility, and body the enclosing item supplies.
struct FunctionSignatureParts {
    name: Identifier,
    generics: Generics,
    receiver: Option<Receiver>,
    parameters: Vec<Parameter>,
    return_type: Option<TypeReference>,
}

impl FunctionSignatureParts {
    fn into_function(
        self,
        attributes: Vec<Attribute>,
        visibility: Visibility,
        body: Block,
    ) -> Function {
        Function {
            attributes,
            visibility,
            name: self.name,
            generics: self.generics,
            receiver: self.receiver,
            parameters: self.parameters,
            return_type: self.return_type,
            body,
        }
    }
}

/// Reading a function signature into its Core parts — a verb on the `syn::Signature`
/// noun. A Tier-1 signature is a plain function: no `const`/`async`/`unsafe`, no
/// explicit ABI, no variadic, a `self`/`&self` or absent receiver, identifier
/// parameters, and a signature-position return type.
trait ReadFunctionSignature {
    fn read_function_signature<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<FunctionSignatureParts, Error>;
}

impl ReadFunctionSignature for syn::Signature {
    fn read_function_signature<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<FunctionSignatureParts, Error> {
        if self.constness.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a const function",
            });
        }
        if self.asyncness.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "an async function",
            });
        }
        if self.unsafety.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "an unsafe function",
            });
        }
        if self.abi.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "an explicit-ABI function",
            });
        }
        if self.variadic.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a variadic function",
            });
        }
        let name = interner.intern(Name::new(self.ident.to_string()));
        let generics = self.generics.read(interner)?;
        let mut receiver = None;
        let mut parameters = Vec::new();
        for input in &self.inputs {
            match input {
                syn::FnArg::Receiver(syn_receiver) => {
                    receiver = Some(syn_receiver.read_receiver()?);
                }
                syn::FnArg::Typed(pat_type) => {
                    parameters.push(pat_type.read_parameter(interner)?);
                }
            }
        }
        let return_type = match &self.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(ty.read_signature_type(interner)?),
        };
        Ok(FunctionSignatureParts {
            name,
            generics,
            receiver,
            parameters,
            return_type,
        })
    }
}

/// Reading a method receiver — a verb on the `syn::Receiver` noun. Only `self` by
/// value and `&self` by shared reference are modeled.
trait ReadReceiver {
    fn read_receiver(&self) -> Result<Receiver, Error>;
}

impl ReadReceiver for syn::Receiver {
    fn read_receiver(&self) -> Result<Receiver, Error> {
        if self.colon_token.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a typed self receiver",
            });
        }
        if self.mutability.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a `mut` self receiver",
            });
        }
        match &self.reference {
            None => Ok(Receiver::Value),
            Some((_, lifetime)) => {
                if lifetime.is_some() {
                    return Err(Error::UnsupportedFunctionSignature {
                        construct: "a lifetime-annotated self receiver",
                    });
                }
                Ok(Receiver::Reference)
            }
        }
    }
}

/// Reading a typed parameter — a verb on the `syn::PatType` noun. The pattern must
/// be a plain identifier; the type is a signature-position type.
trait ReadParameter {
    fn read_parameter<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Parameter, Error>;
}

impl ReadParameter for syn::PatType {
    fn read_parameter<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Parameter, Error> {
        let syn::Pat::Ident(ident) = &*self.pat else {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a non-identifier parameter pattern",
            });
        };
        if ident.by_ref.is_some() || ident.mutability.is_some() || ident.subpat.is_some() {
            return Err(Error::UnsupportedFunctionSignature {
                construct: "a complex parameter binding",
            });
        }
        let name = interner.intern(Name::new(ident.ident.to_string()));
        let type_reference = self.ty.read_signature_type(interner)?;
        Ok(Parameter {
            name,
            type_reference,
        })
    }
}

/// Reading a type in *function-signature position*, where a shared reference and an
/// `impl Trait` bound list are in subset (they never are in wire-data field
/// position) — a verb on the type noun distinct from the wire-data [`ReadRust`]
/// read. Path and application types delegate to the wire-data read; `&mut` and every
/// other type kind fail loudly there.
trait ReadSignatureType {
    fn read_signature_type<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<TypeReference, Error>;
}

impl ReadSignatureType for syn::Type {
    fn read_signature_type<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<TypeReference, Error> {
        match self {
            syn::Type::Reference(reference) => {
                let mutability = match reference.mutability {
                    None => ReferenceMutability::Shared,
                    Some(_) => ReferenceMutability::Mutable,
                };
                let lifetime = reference
                    .lifetime
                    .as_ref()
                    .map(|lifetime| interner.intern(Name::new(lifetime.ident.to_string())));
                let referent = Box::new(reference.elem.read_signature_type(interner)?);
                Ok(TypeReference::Reference(ReferenceType {
                    lifetime,
                    mutability,
                    referent,
                }))
            }
            syn::Type::Slice(slice) => {
                let element = Box::new(slice.elem.read_signature_type(interner)?);
                Ok(TypeReference::Slice(SliceType { element }))
            }
            syn::Type::ImplTrait(impl_trait) => {
                let mut bounds = Vec::with_capacity(impl_trait.bounds.len());
                for bound in &impl_trait.bounds {
                    let syn::TypeParamBound::Trait(trait_bound) = bound else {
                        return Err(Error::UnsupportedType {
                            construct: "a non-trait bound in an impl-Trait type",
                        });
                    };
                    if !matches!(trait_bound.modifier, syn::TraitBoundModifier::None) {
                        return Err(Error::UnsupportedType {
                            construct: "a `?Sized` bound in an impl-Trait type",
                        });
                    }
                    if trait_bound.lifetimes.is_some() {
                        return Err(Error::UnsupportedType {
                            construct: "a higher-ranked bound in an impl-Trait type",
                        });
                    }
                    bounds.push(trait_bound.path.read_type_reference(interner)?);
                }
                Ok(TypeReference::ImplTrait(ImplTraitType { bounds }))
            }
            _ => self.read(interner),
        }
    }
}

impl ReadRust for syn::Block {
    type Logos = Block;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Block, Error> {
        if self.stmts.len() != 1 {
            return Err(Error::UnsupportedStatement {
                construct: if self.stmts.is_empty() {
                    "an empty body"
                } else {
                    "a multi-statement body"
                },
            });
        }
        match &self.stmts[0] {
            syn::Stmt::Expr(expression, None) => Ok(Block {
                tail_expression: expression.read(interner)?,
            }),
            syn::Stmt::Expr(_, Some(_)) => Err(Error::UnsupportedStatement {
                construct: "a semicolon-terminated expression statement",
            }),
            syn::Stmt::Local(_) => Err(Error::UnsupportedStatement {
                construct: "a `let` binding",
            }),
            syn::Stmt::Item(_) => Err(Error::UnsupportedStatement {
                construct: "a nested item",
            }),
            syn::Stmt::Macro(_) => Err(Error::UnsupportedStatement {
                construct: "a macro statement",
            }),
        }
    }
}

impl ReadRust for syn::Expr {
    type Logos = Expression;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Expression, Error> {
        match self {
            syn::Expr::Path(path_expression) => {
                if path_expression.qself.is_some() {
                    return Err(Error::UnsupportedExpression {
                        construct: "a qualified-path value expression",
                    });
                }
                if path_expression.path.is_ident("self") {
                    return Ok(Expression::Receiver);
                }
                Ok(Expression::Path(path_expression.path.read(interner)?))
            }
            syn::Expr::Reference(reference) => {
                if reference.mutability.is_some() {
                    return Err(Error::UnsupportedExpression {
                        construct: "a `&mut` borrow",
                    });
                }
                Ok(Expression::Reference(ReferenceExpression {
                    referent: Box::new(reference.expr.read(interner)?),
                }))
            }
            syn::Expr::Field(field) => {
                let base = Box::new(field.base.read(interner)?);
                match &field.member {
                    syn::Member::Unnamed(index) => Ok(Expression::Field(TupleFieldAccess {
                        base,
                        index: index.index,
                    })),
                    syn::Member::Named(_) => Err(Error::UnsupportedExpression {
                        construct: "a named field access",
                    }),
                }
            }
            syn::Expr::Call(call) => {
                let callee = call.func.read_callee(interner)?;
                let arguments = call
                    .args
                    .iter()
                    .map(|argument| argument.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::Call(Call { callee, arguments }))
            }
            syn::Expr::MethodCall(method_call) => {
                let type_arguments = match &method_call.turbofish {
                    None => Vec::new(),
                    Some(turbofish) => turbofish.read_turbofish(interner)?,
                };
                let receiver = Box::new(method_call.receiver.read(interner)?);
                let method = interner.intern(Name::new(method_call.method.to_string()));
                let arguments = method_call
                    .args
                    .iter()
                    .map(|argument| argument.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::MethodCall(MethodCall {
                    receiver,
                    method,
                    type_arguments,
                    arguments,
                }))
            }
            syn::Expr::Lit(literal) => match &literal.lit {
                syn::Lit::Str(string) => Ok(Expression::StringLiteral(string.value())),
                syn::Lit::Int(integer) => Ok(Expression::IntegerLiteral(integer.read_integer()?)),
                _ => Err(Error::UnsupportedExpression {
                    construct: "a non-string, non-integer literal",
                }),
            },
            syn::Expr::Array(array) => {
                let elements = array
                    .elems
                    .iter()
                    .map(|element| element.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::Array(ArrayExpression { elements }))
            }
            // A braced block that is a single tail expression carries no Core meaning
            // beyond the expression itself — the braces are projection re-sugaring
            // prettyplease re-adds (a nested match-arm body is wrapped this way). A
            // labeled block or a multi-statement block is a real class-B body and
            // stays out of subset, enforced by the single-tail `syn::Block` read.
            syn::Expr::Block(block) => {
                if block.label.is_some() {
                    return Err(Error::UnsupportedExpression {
                        construct: "a labeled block expression",
                    });
                }
                Ok(block.block.read(interner)?.tail_expression)
            }
            syn::Expr::Match(match_expression) => {
                let scrutinee = Box::new(match_expression.expr.read(interner)?);
                let arms = match_expression
                    .arms
                    .iter()
                    .map(|arm| arm.read(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::Match(Match { scrutinee, arms }))
            }
            other => Err(Error::UnsupportedExpression {
                construct: other.unsupported_expression_kind(),
            }),
        }
    }
}

/// Reading a call callee — a verb on the `syn::Expr` noun that heads a call. The
/// callee is a plain path (`Self`, `Self::new`, `RecordIdentifier::new`) or a
/// trait-qualified path (`<Self as Trait>::method`).
trait ReadCallee {
    fn read_callee<Interner: NameInterner>(&self, interner: &mut Interner)
    -> Result<Callee, Error>;
}

impl ReadCallee for syn::Expr {
    fn read_callee<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Callee, Error> {
        let syn::Expr::Path(path_expression) = self else {
            return Err(Error::UnsupportedExpression {
                construct: "a non-path call callee",
            });
        };
        let Some(qself) = &path_expression.qself else {
            return Ok(Callee::Path(path_expression.path.read(interner)?));
        };
        if qself.position == 0 {
            return Err(Error::UnsupportedExpression {
                construct: "a qualified path without a trait",
            });
        }
        let self_type = qself.ty.read(interner)?;
        let mut trait_segments = Vec::new();
        let mut member = Vec::new();
        for (index, segment) in path_expression.path.segments.iter().enumerate() {
            if !matches!(segment.arguments, syn::PathArguments::None) {
                return Err(Error::UnsupportedExpression {
                    construct: "a generic argument in a qualified path",
                });
            }
            let identifier = interner.intern(Name::new(segment.ident.to_string()));
            if index < qself.position {
                trait_segments.push(identifier);
            } else {
                member.push(identifier);
            }
        }
        Ok(Callee::Qualified(QualifiedPath {
            self_type,
            trait_path: PathNode {
                segments: trait_segments,
            },
            member,
        }))
    }
}

/// Reading a turbofish argument list `::<…>` — a verb on the
/// `syn::AngleBracketedGenericArguments` noun the method call carries. The witnessed
/// turbofish is a single type (`parse::<Self>()`); a lifetime is accepted for
/// symmetry with type-application arguments.
trait ReadTurbofish {
    fn read_turbofish<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Vec<TypeReference>, Error>;
}

impl ReadTurbofish for syn::AngleBracketedGenericArguments {
    fn read_turbofish<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<Vec<TypeReference>, Error> {
        self.args
            .iter()
            .map(|argument| match argument {
                syn::GenericArgument::Type(inner) => inner.read(interner),
                syn::GenericArgument::Lifetime(lifetime) => Ok(TypeReference::Lifetime(
                    interner.intern(Name::new(lifetime.ident.to_string())),
                )),
                _ => Err(Error::UnsupportedExpression {
                    construct: "a non-type turbofish argument",
                }),
            })
            .collect()
    }
}

/// Reading an integer literal into stringless data — a verb on the `syn::LitInt`
/// noun. The value comes from the decimal normalization; the surface form (decimal
/// or zero-padded lowercase hexadecimal) becomes a closed [`IntegerRepresentation`],
/// so no raw token text is stored. A suffix, a digit separator, an uppercase-hex
/// digit, or a binary/octal radix is outside the closed modeled form.
trait ReadInteger {
    fn read_integer(&self) -> Result<IntegerLiteral, Error>;
}

impl ReadInteger for syn::LitInt {
    fn read_integer(&self) -> Result<IntegerLiteral, Error> {
        if !self.suffix().is_empty() {
            return Err(Error::UnsupportedExpression {
                construct: "a suffixed integer literal",
            });
        }
        let text = self.to_string();
        if text.contains('_') {
            return Err(Error::UnsupportedExpression {
                construct: "an integer literal with digit separators",
            });
        }
        let value =
            self.base10_digits()
                .parse::<u128>()
                .map_err(|_| Error::UnsupportedExpression {
                    construct: "an integer literal out of the 128-bit range",
                })?;
        let representation = if let Some(hexadecimal) = text.strip_prefix("0x") {
            if hexadecimal.chars().any(|digit| digit.is_ascii_uppercase()) {
                return Err(Error::UnsupportedExpression {
                    construct: "an uppercase-hexadecimal integer literal",
                });
            }
            IntegerRepresentation::Hexadecimal {
                minimum_digits: hexadecimal.len() as u16,
            }
        } else if text.starts_with("0b") || text.starts_with("0o") {
            return Err(Error::UnsupportedExpression {
                construct: "a binary or octal integer literal",
            });
        } else {
            IntegerRepresentation::Decimal
        };
        Ok(IntegerLiteral {
            value,
            representation,
        })
    }
}

impl ReadRust for syn::Arm {
    type Logos = MatchArm;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<MatchArm, Error> {
        if self.guard.is_some() {
            return Err(Error::UnsupportedPattern {
                construct: "a match guard",
            });
        }
        let pattern = self.pat.read(interner)?;
        let body = self.body.read(interner)?;
        Ok(MatchArm { pattern, body })
    }
}

impl ReadRust for syn::Pat {
    type Logos = Pattern;

    fn read<Interner: NameInterner>(&self, interner: &mut Interner) -> Result<Pattern, Error> {
        match self {
            syn::Pat::Path(path_pattern) => {
                if path_pattern.qself.is_some() {
                    return Err(Error::UnsupportedPattern {
                        construct: "a qualified-path pattern",
                    });
                }
                Ok(Pattern::Path(path_pattern.path.read(interner)?))
            }
            syn::Pat::TupleStruct(tuple_struct) => {
                if tuple_struct.qself.is_some() {
                    return Err(Error::UnsupportedPattern {
                        construct: "a qualified tuple-struct pattern",
                    });
                }
                let path = tuple_struct.path.read(interner)?;
                let elements = tuple_struct
                    .elems
                    .iter()
                    .map(|element| element.read_pattern_element(interner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Pattern::TupleVariant(TupleVariantPattern {
                    path,
                    elements,
                }))
            }
            other => Err(Error::UnsupportedPattern {
                construct: other.unsupported_pattern_kind(),
            }),
        }
    }
}

/// Reading one element of a tuple-variant pattern — a verb on the `syn::Pat` noun.
/// A wildcard or a plain identifier binding are the modeled elements.
trait ReadPatternElement {
    fn read_pattern_element<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<PatternElement, Error>;
}

impl ReadPatternElement for syn::Pat {
    fn read_pattern_element<Interner: NameInterner>(
        &self,
        interner: &mut Interner,
    ) -> Result<PatternElement, Error> {
        match self {
            syn::Pat::Wild(_) => Ok(PatternElement::Wildcard),
            syn::Pat::Ident(ident) => {
                if ident.by_ref.is_some() || ident.mutability.is_some() || ident.subpat.is_some() {
                    return Err(Error::UnsupportedPattern {
                        construct: "a complex binding sub-pattern",
                    });
                }
                Ok(PatternElement::Binding(
                    interner.intern(Name::new(ident.ident.to_string())),
                ))
            }
            _ => Err(Error::UnsupportedPattern {
                construct: "a complex tuple-variant element pattern",
            }),
        }
    }
}

/// Name the kind of an out-of-vocabulary `syn::Expr` for the loud error — a verb on
/// the expression noun.
trait UnsupportedExpressionKind {
    fn unsupported_expression_kind(&self) -> &'static str;
}

impl UnsupportedExpressionKind for syn::Expr {
    fn unsupported_expression_kind(&self) -> &'static str {
        match self {
            syn::Expr::Struct(_) => "a struct literal",
            syn::Expr::If(_) => "an if expression",
            syn::Expr::Loop(_) => "a loop expression",
            syn::Expr::While(_) => "a while expression",
            syn::Expr::ForLoop(_) => "a for expression",
            syn::Expr::Closure(_) => "a closure",
            syn::Expr::Binary(_) => "a binary-operator expression",
            syn::Expr::Unary(_) => "a unary-operator expression",
            syn::Expr::Tuple(_) => "a tuple expression",
            syn::Expr::Index(_) => "an index expression",
            syn::Expr::Return(_) => "a return expression",
            syn::Expr::Let(_) => "a let expression",
            syn::Expr::Macro(_) => "a macro expression",
            syn::Expr::Try(_) => "a try (`?`) expression",
            syn::Expr::Await(_) => "an await expression",
            syn::Expr::Cast(_) => "a cast expression",
            syn::Expr::Range(_) => "a range expression",
            syn::Expr::Paren(_) => "a parenthesized expression",
            syn::Expr::Group(_) => "a grouped expression",
            syn::Expr::Assign(_) => "an assignment expression",
            _ => "an unrecognized expression",
        }
    }
}

/// Name the kind of an out-of-vocabulary `syn::Pat` for the loud error — a verb on
/// the pattern noun.
trait UnsupportedPatternKind {
    fn unsupported_pattern_kind(&self) -> &'static str;
}

impl UnsupportedPatternKind for syn::Pat {
    fn unsupported_pattern_kind(&self) -> &'static str {
        match self {
            syn::Pat::Wild(_) => "a wildcard arm",
            syn::Pat::Ident(_) => "a binding (catch-all) arm",
            syn::Pat::Lit(_) => "a literal pattern",
            syn::Pat::Struct(_) => "a struct pattern",
            syn::Pat::Or(_) => "an or-pattern",
            syn::Pat::Tuple(_) => "a tuple pattern",
            syn::Pat::Reference(_) => "a reference pattern",
            syn::Pat::Range(_) => "a range pattern",
            syn::Pat::Rest(_) => "a rest (`..`) pattern",
            syn::Pat::Slice(_) => "a slice pattern",
            _ => "an unrecognized pattern",
        }
    }
}
