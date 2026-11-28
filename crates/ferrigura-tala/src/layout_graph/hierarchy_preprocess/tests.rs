use serde_json::Value;

use super::super::hierarchy_preprocess_checkpoint;
use crate::Graph;

#[test]
fn measured_library_hierarchy_scopes_match_pinned_go() {
    let capture: Value = serde_json::from_str(include_str!("library.json")).unwrap();
    let graph: Graph = serde_json::from_value(capture["input"].clone()).unwrap();
    let expected = &capture["expected"];
    let actual = serde_json::to_value(hierarchy_preprocess_checkpoint(&graph).unwrap()).unwrap();
    assert_eq!(actual["scopes"], expected["scopes"]);
    assert_eq!(actual["candidates"], expected["candidates"]);
}
