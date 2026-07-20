//! The generated-module prelude retains only its cfg-gated NOTA import. Scalar
//! aliases are not part of the encoded Logos algebra and are rejected at the Rust
//! codec boundary.

use name_table::{NameResolver, NameTable};
use textual_rust::{DecodeAtomically, RustSource};

/// The cfg-gated NOTA import carried by generated modules.
const NOTA_IMPORT: &str = "#[rustfmt::skip]\n#[cfg(feature = \"nota-text\")]\npub use nota::{NotaDecodeError, NotaEncode, NotaSource};\n";

/// Decode every top-level item in `text` and project the whole batch back through a
/// single prettyplease pass.
fn round_trip_block(text: &str) -> String {
    let source = RustSource::new(text);
    let mut table = NameTable::new(name_table::IdentifierNamespace::Logos);
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
fn the_nota_import_round_trips_byte_exact() {
    assert_eq!(round_trip_block(NOTA_IMPORT), NOTA_IMPORT);
}
