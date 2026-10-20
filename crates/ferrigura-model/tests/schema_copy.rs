//! The crate's copy of Mermaid's configuration schema, which `build.rs` reads, is the source byte for
//! byte. The source is outside the crate, so the copy goes into the crate's package on crates.io;
//! in the repository the two must not drift apart. After a change of the source, copy it again:
//! `cp packages/mermaid/src/schemas/config.schema.yaml crates/ferrigura-model/schema/`.

use std::path::Path;

#[test]
fn the_copy_of_the_schema_is_the_source() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = crate_dir.join("../../packages/mermaid/src/schemas/config.schema.yaml");
    let copy = crate_dir.join("schema/config.schema.yaml");
    let Ok(source) = std::fs::read(&source) else {
        // Outside the repository, as in the crate's package, there is no source to compare with.
        return;
    };
    let copy = std::fs::read(&copy).expect("the crate's copy of the schema");
    assert!(
        source == copy,
        "crates/ferrigura-model/schema/config.schema.yaml differs from packages/mermaid/src/schemas/config.schema.yaml; copy the source again"
    );
}
