//! Port of the `getData cluster behavior` cases of `packages/mermaid/src/diagrams/er/erDb.spec.js`:
//! the two tests that check the render data, which the ER renderer ports (M6). Test names follow the
//! spec's titles in snake_case.
//!
//! The spec drives `ErDB` directly, and so does this test: the DB's state is read as the model
//! (`ErDb::into_model`), and `getData` of that model is checked.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::er::{Cardinality, Identification, RelSpec};
use ferrigura_render::er::data::{LayoutData, Node, get_data};
use ferrigura_syntax::er::{ErDb, Member};

fn names(values: &[&str]) -> Vec<Member> {
    values.iter().map(|value| Member::from(*value)).collect()
}

fn spec(card_a: Cardinality, rel_type: Identification, card_b: Cardinality) -> RelSpec {
    RelSpec {
        card_a,
        rel_type,
        card_b,
    }
}

/// The fresh DB of the spec's `beforeEach`.
fn fresh_db() -> ErDb {
    let mut db = ErDb::new();
    db.clear();
    db
}

/// The layout data of the DB's state, with the default configuration.
fn layout(db: ErDb) -> LayoutData {
    get_data(&db.into_model(), &MermaidConfig::default())
}

/// The node with the label, as the spec finds its nodes by `label`.
fn node_labelled<'a>(data: &'a LayoutData, label: &str) -> Option<&'a Node> {
    data.nodes.iter().find(|node| node.label == label)
}

/// The node with the id.
fn node_with_id<'a>(data: &'a LayoutData, id: &str) -> Option<&'a Node> {
    data.nodes.iter().find(|node| node.id == id)
}

mod get_data_cluster_behavior {
    use super::*;

    #[test]
    fn returns_nested_cluster_nodes_with_correct_parent_relationships() {
        let mut db = fresh_db();
        let outer_id = db.add_subgraph("sub1", &names(&["sub1_1", "A"]), "Group One");
        let inner_id = db.add_subgraph("sub1_1", &names(&["B"]), "Group Two");
        db.add_entity("A", "");
        db.add_entity("B", "");

        let data = layout(db);
        let outer = node_with_id(&data, &outer_id).expect("the outer cluster");
        let inner = node_with_id(&data, &inner_id).expect("the inner cluster");
        let customer = node_labelled(&data, "A").expect("node A");
        let account = node_labelled(&data, "B").expect("node B");

        assert_eq!(data.nodes.len(), 4);
        assert!(outer.is_group);
        assert!(inner.is_group);
        assert_eq!(outer.label, "Group One");
        assert_eq!(inner.label, "Group Two");
        assert_eq!(inner.parent_id.as_deref(), Some(outer_id.as_str()));
        assert_eq!(customer.parent_id.as_deref(), Some(outer_id.as_str()));
        assert!(!customer.is_group);
        assert_eq!(account.parent_id.as_deref(), Some(inner_id.as_str()));
        assert!(!account.is_group);
    }

    #[test]
    fn returns_edges_connected_to_subgraphs() {
        let mut db = fresh_db();
        db.add_subgraph("sub1", &names(&["A"]), "Group One");
        db.add_entity("A", "");
        db.add_entity("B", "");
        let entity_b = db.get_entity("B").expect("entity B").id.clone();
        db.add_relationship(
            "sub1",
            "roleA",
            "B",
            spec(
                Cardinality::OnlyOne,
                Identification::Identifying,
                Cardinality::ZeroOrMore,
            ),
        );

        let data = layout(db);

        assert_eq!(data.edges.len(), 1);
        assert_eq!(data.edges[0].start, "sub1");
        assert_eq!(data.edges[0].end, entity_b);
    }
}
