//! The corpus as the capture tool wrote it: 131 cases in the three groups, the named exclusions, and the key set.

mod support;

use std::fs;

use serde_json::Value;
use support::corpus::{corpus_dir, load_cases};

#[test]
fn the_manifest_lists_every_case_file() {
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(corpus_dir().join("manifest.json")).expect("the manifest exists"),
    )
    .expect("the manifest is JSON");
    let cases = load_cases();
    let listed = manifest["cases"].as_array().expect("cases is an array");
    assert_eq!(
        cases.len(),
        listed.len(),
        "case files and manifest entries differ"
    );
    assert_eq!(listed.len(), 131);
    for case in &cases {
        assert!(
            listed
                .iter()
                .any(|entry| entry["name"] == case.name.as_str()),
            "{} is not in the manifest",
            case.name
        );
    }
}

#[test]
fn the_groups_and_the_exclusions_match_the_brief() {
    let cases = load_cases();
    let count = |group: &str| cases.iter().filter(|case| case.group == group).count();
    assert_eq!(count("fixture"), 44);
    assert_eq!(count("ddlt"), 27);
    assert_eq!(count("generated"), 60);
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(corpus_dir().join("manifest.json")).expect("the manifest exists"),
    )
    .expect("the manifest is JSON");
    assert_eq!(manifest["excluded"].as_array().map(Vec::len), Some(11));
    assert_eq!(manifest["keySet"].as_array().map(Vec::len), Some(3460));
}

#[test]
fn every_case_has_the_stages_in_order_and_an_output() {
    for case in load_cases() {
        assert_eq!(case.stages[0].0, "sugiyama", "{}: first stage", case.name);
        assert_eq!(case.stages[1].0, "writeback", "{}: second stage", case.name);
        assert_eq!(case.stages[2].0, "routed", "{}: third stage", case.name);
        assert!(
            case.stages[3..]
                .iter()
                .all(|(stage, _)| stage.starts_with("post:")),
            "{}: stages after routed are post-processing passes",
            case.name
        );
        assert!(
            case.output.is_object(),
            "{}: the output is a snapshot",
            case.name
        );
    }
}
