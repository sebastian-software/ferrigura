//! Port of `packages/mermaid/src/diagrams/er/erDb.spec.js`. Test names follow the spec's titles in
//! snake_case. The DB is driven directly, as the spec drives `ErDB`; the spec's `beforeEach` is a
//! fresh DB. The two `getData` tests check the render data, which the ER renderer ports, in
//! `ferrigura-render/tests/er_get_data_spec.rs`.

use ferrigura_model::er::{Cardinality, Identification, RelSpec};
use ferrigura_syntax::er::{ErDb, Member};

fn names(values: &[&str]) -> Vec<Member> {
    values.iter().map(|value| Member::from(*value)).collect()
}

fn fresh_db() -> ErDb {
    let mut db = ErDb::new();
    db.clear();
    db
}

fn spec(card_a: Cardinality, rel_type: Identification, card_b: Cardinality) -> RelSpec {
    RelSpec {
        card_a,
        rel_type,
        card_b,
    }
}

mod er_db_subgraph_behavior {
    use super::*;

    mod add_subgraph {
        use super::*;

        #[test]
        fn adds_subgraphs_and_returns_provided_id_title_and_nodes() {
            let mut db = fresh_db();
            let id = db.add_subgraph("sub1", &names(&["A", "B"]), "Group One");
            let subgraphs = db.get_subgraphs();

            assert_eq!(id, "sub1");
            assert_eq!(subgraphs.len(), 1);
            assert_eq!(subgraphs[0].id, "sub1");
            assert_eq!(subgraphs[0].title, "Group One");
            assert_eq!(subgraphs[0].nodes, ["A", "B"]);
        }

        #[test]
        fn handles_subgraph_without_explicit_title_fallback() {
            let mut db = fresh_db();
            db.add_subgraph("sub1", &names(&["A"]), "");
            let subgraph = &db.get_subgraphs()[0];

            assert_eq!(subgraph.id, "sub1");
            assert_eq!(subgraph.title, "");
        }
    }

    mod make_uniq_behavior {
        use super::*;

        #[test]
        fn prevents_nodes_from_being_added_to_subsequent_subgraphs_if_already_present() {
            let mut db = fresh_db();
            db.add_subgraph("sub1", &names(&["A", "B"]), "sub1");
            db.add_subgraph("sub2", &names(&["A", "C"]), "sub2");
            let subgraphs = db.get_subgraphs();

            assert_eq!(subgraphs.len(), 2);
            // sub2 should not contain 'A' because it was already in sub1
            assert!(subgraphs[1].nodes.contains(&"C".to_owned()));
            assert!(!subgraphs[1].nodes.contains(&"A".to_owned()));
        }

        #[test]
        fn allows_full_removal_if_all_nodes_already_exist_elsewhere() {
            let mut db = fresh_db();
            db.add_subgraph("sub1", &names(&["A"]), "sub1");
            db.add_subgraph("sub2", &names(&["A"]), "sub2");
            let sub2 = db.get_subgraphs().iter().find(|s| s.id == "sub2");

            assert!(sub2.is_some_and(|subgraph| subgraph.nodes.is_empty()));
        }
    }

    mod relationship_behavior {
        use super::*;

        #[test]
        fn stores_relationships_with_subgraph_id_when_entity_is_a_subgraph() {
            let mut db = fresh_db();
            db.add_subgraph("sub1", &names(&["X"]), "sub1");
            let rel_spec = spec(
                Cardinality::OnlyOne,
                Identification::Identifying,
                Cardinality::ZeroOrOne,
            );
            db.add_relationship("sub1", "roleA", "Y", rel_spec);
            let relationships = db.get_relationships();

            assert_eq!(relationships.len(), 1);
            assert_eq!(relationships[0].entity_a, "sub1");
            let y = db.get_entity("Y").expect("entity Y");
            assert_eq!(relationships[0].entity_b, y.id);
        }
    }

    // The `getData cluster behavior` cases are the render data of the ER renderer: they are in
    // `ferrigura-render/tests/er_get_data_spec.rs`.

    mod css_styles_and_classes_behavior {
        use super::*;

        #[test]
        fn applies_classes_and_styles_to_subgraphs() {
            let mut db = fresh_db();
            db.add_subgraph("sub1", &names(&["A"]), "sub1");
            db.set_class(&["sub1".to_owned()], &["B".to_owned()]);
            db.add_css_styles(&["sub1".to_owned()], &["fill:red".to_owned()]);
            let subgraphs = db.get_subgraphs();

            assert_eq!(subgraphs.len(), 1);
            assert!(subgraphs[0].classes.contains(&"B".to_owned()));
            assert!(subgraphs[0].styles.contains(&"fill:red".to_owned()));
        }
    }
}
