// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[test]
fn preserves_the_unsigned_singleton_hash() {
    let ids = allocate_ids([b"a".as_slice()]).unwrap();
    assert_eq!(ids, [3_826_002_220]);
    assert!(ids[0] > i64::from(i32::MAX));
    assert!(ids[0] > 0 && ids[0] != -1);
}

#[test]
fn resolves_collisions_deterministically() {
    let a = b"costarring".as_slice();
    let b = b"liquid".as_slice();
    let single = b"a".as_slice();
    assert_eq!(fnv32(a), fnv32(b));
    assert_eq!(
        allocate_ids([a, b, single]).unwrap(),
        [FIRST_SPILL_ID, FIRST_SPILL_ID + 1, i64::from(fnv32(single))]
    );
    assert_eq!(
        allocate_ids([single, b, a]).unwrap(),
        [i64::from(fnv32(single)), FIRST_SPILL_ID + 1, FIRST_SPILL_ID]
    );
}

#[test]
fn spills_the_reserved_zero_hash() {
    let zero = [0xcc, 0x24, 0x31, 0xc4];
    assert_eq!(fnv32(&zero), 0);
    assert_eq!(allocate_ids([zero.as_slice()]).unwrap(), [FIRST_SPILL_ID]);
}

#[test]
fn rejects_duplicate_identity() {
    let duplicate = b"duplicate".as_slice();
    assert_eq!(
        allocate_ids([duplicate, duplicate]),
        Err(Error::InvalidGraph)
    );
}

#[test]
fn preserves_d2_object_visit_order() {
    let node = |id: &str, parent_id: Option<&str>| Node {
        id: id.into(),
        parent_id: parent_id.map(str::to_owned),
        direction: None,
        is_group: id == "a",
        shape: "rect".into(),
        width: 10.0,
        height: 10.0,
        label: String::new(),
        label_width: 0.0,
        label_height: 0.0,
    };
    let graph = Graph {
        direction: None,
        nodes: vec![
            node("a", None),
            node("aa", Some("a")),
            node("b", None),
            node("ab", Some("a")),
        ],
        edges: vec![],
    };
    let prepared = LayoutGraph::translate(&graph).unwrap();
    assert_eq!(
        prepared
            .nodes
            .iter()
            .map(|n| graph.nodes[n.input_index].id.as_str())
            .collect::<Vec<_>>(),
        ["a", "aa", "ab", "b"]
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    name: String,
    input: Graph,
    translated: LayoutGraph,
    prescaled_sizes: Vec<[f64; 2]>,
}

#[test]
fn translated_and_prescaled_graphs_match_pinned_go() {
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!("fixtures.json")).unwrap();
    assert!(!fixtures.is_empty());
    for Fixture {
        name,
        input,
        translated,
        prescaled_sizes,
    } in fixtures
    {
        let mut graph = LayoutGraph::translate(&input).unwrap();
        assert_eq!(graph, translated, "{name} translated");
        let mut expected = translated;
        assert_eq!(expected.nodes.len(), prescaled_sizes.len(), "{name}");
        for (node, [width, height]) in expected.nodes.iter_mut().zip(prescaled_sizes) {
            node.width = width;
            node.height = height;
        }
        graph.prescale();
        assert_eq!(graph, expected, "{name} prescaled");
    }
}
