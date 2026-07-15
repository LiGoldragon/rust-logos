//! The decode∘encode law and the writer's byte-exact output.
//!
//! `encode(core) → text → decode(text)` reproduces the CoreLogos value, and its
//! content identity does not move through text — the hash is stable across the
//! text round trip because decode interns over the same continuous identifier
//! space, so the reconstructed value is identical, not merely equivalent.

mod support;

use name_table::NameTable;
use textual_rust::{DecodeAtomically, RustSource};

/// The exact `CommitSequence` golden, as schema-rust emits it.
const COMMIT_SEQUENCE_GOLDEN: &str = "\
#[rustfmt::skip]
#[cfg_attr(
    feature = \"nota-text\",
    derive(nota::NotaDecode, nota::NotaDecodeTraced, nota::NotaEncode)
)]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommitSequence(Integer);
";

/// The writer projects the `CommitSequence` fixture to its byte-exact golden text.
#[test]
fn the_writer_projects_commit_sequence_to_its_byte_exact_golden() {
    let mut names = NameTable::new();
    let core = support::commit_sequence(&mut names);
    let text = RustSource::project_item(&core, &names).expect("project");
    assert_eq!(text.as_str(), COMMIT_SEQUENCE_GOLDEN);
}

/// `encode → decode` reproduces the CoreLogos value and its content identity, for
/// both golden-pair fixtures (the newtype and the field-visibility-bearing struct).
#[test]
fn decode_after_encode_is_identity_on_core_with_a_stable_hash() {
    let builders: [fn(&mut NameTable) -> core_logos::CoreItem; 2] =
        [support::commit_sequence, support::database_marker];
    for build in builders {
        let mut names = NameTable::new();
        let core = build(&mut names);
        let before = core.content_identity().expect("hash before");

        // Encode with the resolver, then decode over the same continuous identifier
        // space (an extension of the encode table), so pre-existing names re-intern
        // to their original identifiers and the reconstructed value is identical.
        let text = RustSource::project_item(&core, &names).expect("project");
        let mut decode_table = NameTable::extend_from(&names);
        let items = text.parse_items().expect("parse");
        assert_eq!(items.len(), 1, "one item per fixture");
        let decoded = items[0]
            .decode_atomically(&mut decode_table)
            .expect("in-subset decode");

        assert_eq!(decoded, core, "the CoreLogos value round-trips exactly");
        let after = decoded.content_identity().expect("hash after");
        assert_eq!(
            before.bytes(),
            after.bytes(),
            "content identity does not move through text",
        );
    }
}
