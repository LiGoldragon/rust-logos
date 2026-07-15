//! The golden-pair CoreLogos fixtures, rebuilt from the public `core-logos` API —
//! the same `CommitSequence` newtype and `DatabaseMarker` struct core-logos ships
//! as its own golden-pair support, reconstructed here so the decode∘encode law can
//! run against them. Test-only.
#![allow(dead_code)]

use core_logos::{
    Attribute, ConfigurationAttribute, ConfigurationPredicate, CoreItem, DeriveGroup, Field,
    Generics, Newtype, PathNode, Struct, TypeReference, Visibility,
};
use name_table::{Identifier, Name, NameTable};

/// Intern a dotted path as a segment vector of identifiers.
pub fn path(names: &mut NameTable, segments: &[&str]) -> PathNode {
    PathNode {
        segments: segments
            .iter()
            .map(|segment| names.intern(Name::new(*segment)))
            .collect(),
    }
}

/// Intern a single name.
pub fn identifier(names: &mut NameTable, name: &str) -> Identifier {
    names.intern(Name::new(name))
}

/// The three-attribute golden preamble carried by every wire data item, in stored
/// order: `#[rustfmt::skip]`, the feature-gated NOTA derive group, then the plain
/// rkyv derive group.
pub fn golden_preamble(names: &mut NameTable) -> Vec<Attribute> {
    vec![
        Attribute::ToolPath(path(names, &["rustfmt", "skip"])),
        Attribute::Configuration(ConfigurationAttribute {
            predicate: ConfigurationPredicate::Feature(identifier(names, "nota-text")),
            inner: Box::new(Attribute::Derive(DeriveGroup {
                paths: vec![
                    path(names, &["nota", "NotaDecode"]),
                    path(names, &["nota", "NotaDecodeTraced"]),
                    path(names, &["nota", "NotaEncode"]),
                ],
            })),
        }),
        Attribute::Derive(DeriveGroup {
            paths: vec![
                path(names, &["rkyv", "Archive"]),
                path(names, &["rkyv", "Serialize"]),
                path(names, &["rkyv", "Deserialize"]),
                path(names, &["Clone"]),
                path(names, &["Debug"]),
                path(names, &["PartialEq"]),
                path(names, &["Eq"]),
            ],
        }),
    ]
}

/// `CommitSequence` — a public newtype wrapping `Integer`, with the full preamble.
pub fn commit_sequence(names: &mut NameTable) -> CoreItem {
    let attributes = golden_preamble(names);
    let name = identifier(names, "CommitSequence");
    let wrapped = TypeReference::Path(path(names, &["Integer"]));
    CoreItem::Newtype(Newtype {
        visibility: Visibility::Public,
        attributes,
        name,
        wrapped,
    })
}

/// `DatabaseMarker` — a public struct with two public fields and one private field,
/// carrying visibility as data at both the item and field level.
pub fn database_marker(names: &mut NameTable) -> CoreItem {
    let attributes = golden_preamble(names);
    let name = identifier(names, "DatabaseMarker");
    let fields = vec![
        Field {
            visibility: Visibility::Public,
            name: identifier(names, "commit_sequence"),
            type_reference: TypeReference::Path(path(names, &["CommitSequence"])),
        },
        Field {
            visibility: Visibility::Public,
            name: identifier(names, "state_digest"),
            type_reference: TypeReference::Path(path(names, &["StateDigest"])),
        },
        Field {
            visibility: Visibility::Private,
            name: identifier(names, "secret_digest"),
            type_reference: TypeReference::Path(path(names, &["StateDigest"])),
        },
    ];
    CoreItem::Struct(Struct {
        visibility: Visibility::Public,
        attributes,
        name,
        generics: Generics::none(),
        fields,
    })
}
