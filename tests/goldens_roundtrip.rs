//! The golden-corpus harness: `decode(golden item) → EncodedLogos → encode` must
//! reproduce the item's byte-exact golden text, for every in-subset item. Coverage
//! is reported honestly — in-subset (round-tripped) versus out-of-subset (the
//! trait/impl frontier) — per fixture and in total.

use name_table::NameTable;
use textual_rust::{Coverage, RustSource};

/// The four copied schema-rust goldens (see `tests/fixtures/PROVENANCE.md`).
const GOLDENS: &[(&str, &str)] = &[
    (
        "standard_newtype_impls_generated.rs",
        include_str!("fixtures/standard_newtype_impls_generated.rs"),
    ),
    (
        "collections_generated.rs",
        include_str!("fixtures/collections_generated.rs"),
    ),
    (
        "spirit_generated.rs",
        include_str!("fixtures/spirit_generated.rs"),
    ),
    (
        "runner_generated.rs",
        include_str!("fixtures/runner_generated.rs"),
    ),
];

/// Survey each golden, assert every in-subset item round-trips byte-exact against
/// the real golden bytes, and print the honest coverage counts.
#[test]
fn every_in_subset_golden_item_round_trips_byte_exact() {
    let mut total_in_subset = 0usize;
    let mut total_out_of_subset = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (name, source_text) in GOLDENS {
        let source = RustSource::new(*source_text);
        let mut table = NameTable::new();
        let coverage = Coverage::survey(&source, &mut table).expect("golden parses and surveys");

        let in_subset = coverage.in_subset_count();
        let out_of_subset = coverage.out_of_subset_count();
        total_in_subset += in_subset;
        total_out_of_subset += out_of_subset;

        for item in coverage.in_subset() {
            // (1) Round-trip reproduces the item's canonical prettyplease text.
            if !item.is_byte_exact() {
                failures.push(format!(
                    "[{name}] round-trip diverged:\n--- canonical ---\n{}\n--- reencoded ---\n{}",
                    item.canonical.as_str(),
                    item.reencoded.as_str(),
                ));
                continue;
            }
            // (2) The canonical text is really present in the golden bytes — proving
            //     byte-exactness against the original, not merely against a
            //     self-derived canonical form.
            let canonical = item.canonical.as_str();
            if !source_text.contains(canonical.trim_end()) {
                failures.push(format!(
                    "[{name}] canonical form not found verbatim in golden:\n{canonical}"
                ));
            }
        }

        println!("{name}: {in_subset} in-subset (byte-exact), {out_of_subset} out-of-subset");
    }

    println!(
        "TOTAL: {total_in_subset} in-subset round-tripped byte-exact, \
         {total_out_of_subset} out-of-subset frontier"
    );

    assert!(
        failures.is_empty(),
        "byte-exact round-trip failures:\n{}",
        failures.join("\n\n")
    );
    assert!(
        total_in_subset > 0,
        "the corpus must contain in-subset items"
    );
}
