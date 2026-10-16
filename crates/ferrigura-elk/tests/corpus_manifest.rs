//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//!
//! The corpus is only as good as its manifest: a case file that the manifest does not list is never
//! compared, and a manifest entry without a file fails the comparison for the wrong reason. These
//! tests keep the two in step, so that a dropped case is caught without a browser.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
}

/// The case names the manifest lists, in order, and the names of the case files on disk.
fn names_on_disk_and_in_manifest() -> (Vec<String>, BTreeSet<String>) {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(corpus_dir().join("manifest.json")).expect("manifest.json"),
    )
    .expect("manifest.json parses");
    let listed: Vec<String> = manifest["cases"]
        .as_array()
        .expect("manifest cases")
        .iter()
        .map(|case| case["name"].as_str().expect("case name").to_string())
        .collect();
    let mut on_disk = BTreeSet::new();
    for entry in std::fs::read_dir(corpus_dir()).expect("corpus directory") {
        let path = entry.expect("directory entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && path.file_name().is_some_and(|name| name != "manifest.json")
        {
            on_disk.insert(
                path.file_stem()
                    .expect("file stem")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    (listed, on_disk)
}

#[test]
fn every_case_file_is_listed_in_the_manifest() {
    let (listed, on_disk) = names_on_disk_and_in_manifest();
    let listed_set: BTreeSet<String> = listed.iter().cloned().collect();
    let unlisted: Vec<&String> = on_disk.difference(&listed_set).collect();
    assert!(
        unlisted.is_empty(),
        "case files missing from manifest.json (they would never be compared): {unlisted:?}"
    );
}

#[test]
fn every_manifest_entry_has_a_case_file() {
    let (listed, on_disk) = names_on_disk_and_in_manifest();
    let missing: Vec<&String> = listed
        .iter()
        .filter(|name| !on_disk.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "manifest.json lists cases with no file in tests/corpus: {missing:?}"
    );
}

#[test]
fn manifest_names_are_unique() {
    let (listed, _) = names_on_disk_and_in_manifest();
    let unique: BTreeSet<&String> = listed.iter().collect();
    assert_eq!(
        unique.len(),
        listed.len(),
        "duplicate names in manifest.json"
    );
}
