//! Port of the `getData` case of `packages/mermaid/src/diagrams/requirement/requirementDb.spec.ts`:
//! "should create unique edge ids for parallel relationships". The edge ids are made by `getData`,
//! the renderer's stage, so this test parses the text that makes the two relationships and reads the
//! layout data of the model, as the reference's spec reads its DB's `getData()`.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::requirement::data::get_data;
use ferrigura_syntax::requirement::parse;

#[test]
fn should_create_unique_edge_ids_for_parallel_relationships() {
    let diagram = parse(
        "requirementDiagram\nrequirement \"src\" {\n}\nrequirement \"dst\" {\n}\nsrc - traces -> dst\nsrc - contains -> dst",
    )
    .expect("parses");
    let data = get_data(&diagram, &MermaidConfig::default());

    let edge_ids: Vec<&str> = data.edges.iter().map(|edge| edge.id.as_str()).collect();
    assert_eq!(edge_ids, ["src-dst-0", "src-dst-1"]);
}
