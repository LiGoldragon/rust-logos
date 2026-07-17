//! The generated-module prelude round-trips byte-exact: the four scalar type
//! aliases (as one blank-free block) and the cfg-gated NOTA import each decode from
//! their golden text into CoreLogos and project back to the exact golden bytes.
//! This is the block-level acceptance the fixed module prelude depends on.

use name_table::{NameResolver, NameTable};
use textual_rust::{DecodeAtomically, RustSource};

/// The scalar-alias block exactly as it heads every schema-rust golden — four
/// `#[rustfmt::skip]` type aliases packed with no blank line between them.
const SCALAR_ALIASES: &str = "#[rustfmt::skip]\npub type String = std::string::String;\n#[rustfmt::skip]\npub type Integer = u64;\n#[rustfmt::skip]\npub type Boolean = bool;\n#[rustfmt::skip]\npub type Path = std::string::String;\n";

/// The cfg-gated NOTA import exactly as it heads every schema-rust golden.
const NOTA_IMPORT: &str = "#[rustfmt::skip]\n#[cfg(feature = \"nota-text\")]\npub use nota::{NotaDecodeError, NotaEncode, NotaSource};\n";

/// Decode every top-level item in `text` and project the whole batch back through a
/// single prettyplease pass — the block-level round-trip.
fn round_trip_block(text: &str) -> String {
    let source = RustSource::new(text);
    let mut table = NameTable::new();
    let items = source
        .parse_items()
        .expect("prelude block parses")
        .iter()
        .map(|item| item.decode_atomically(&mut table).expect("in subset"))
        .collect::<Vec<_>>();
    RustSource::project_items(&items, &table as &dyn NameResolver)
        .expect("project")
        .into_string()
}

#[test]
fn the_scalar_alias_block_round_trips_byte_exact() {
    assert_eq!(round_trip_block(SCALAR_ALIASES), SCALAR_ALIASES);
}

#[test]
fn the_nota_import_round_trips_byte_exact() {
    assert_eq!(round_trip_block(NOTA_IMPORT), NOTA_IMPORT);
}
