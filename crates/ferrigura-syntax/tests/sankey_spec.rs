//! Port of `packages/mermaid/src/diagrams/sankey/parser/sankey.spec.ts`. The reference's tests parse
//! the texts, and the one with the issue #7528 sample reads the DB's graph; here the model is the DB
//! state, so the same checks read the model. The reference has no DB spec and no render spec for
//! sankey (`docs-port/sankey/NOTES.md`, section 5).

use ferrigura_model::sankey::SankeyDiagram;
use ferrigura_syntax::sankey::parse;

/// The CSV sample of the reference's spec, read from the reference's own folder.
const ENERGY_CSV: &str =
    include_str!("../../../packages/mermaid/src/diagrams/sankey/parser/energy.csv");

fn parses(text: &str) -> SankeyDiagram {
    parse(text).expect("parses")
}

fn node_ids(diagram: &SankeyDiagram) -> Vec<&str> {
    diagram.nodes.iter().map(|node| node.id.as_str()).collect()
}

#[test]
fn parses_csv_with_sankey_beta_syntax() {
    let diagram = parses(&format!("sankey-beta\n\n {ENERGY_CSV}"));
    // The spec parses the sample without asserting on it; the counts are those of the rendered
    // sample in the reference's probes (`docs-port/sankey/NOTES.md`, section 5).
    assert_eq!(diagram.nodes.len(), 52);
    assert_eq!(diagram.links.len(), 70);
}

#[test]
fn parses_csv_with_sankey_syntax() {
    let diagram = parses(&format!("sankey\n\n {ENERGY_CSV}"));
    assert_eq!(diagram.links.len(), 70);
}

#[test]
fn allows_proto_as_id_with_sankey_beta_syntax() {
    let diagram = parses("sankey-beta\n      __proto__,A,0.597\n      A,__proto__,0.403\n      ");
    assert_eq!(node_ids(&diagram), ["__proto__", "A"]);
    assert_eq!(diagram.links.len(), 2);
}

#[test]
fn allows_proto_as_id_with_sankey_syntax() {
    let diagram = parses("sankey\n      __proto__,A,0.597\n      A,__proto__,0.403\n      ");
    assert_eq!(node_ids(&diagram), ["__proto__", "A"]);
    assert_eq!(diagram.links.len(), 2);
}

#[test]
fn parses_the_issue_7528_sample_with_sankey_beta_syntax() {
    parses_the_issue_7528_sample("sankey-beta");
}

#[test]
fn parses_the_issue_7528_sample_with_sankey_syntax() {
    parses_the_issue_7528_sample("sankey");
}

/// `parses the issue #7528 sample with ${syntax} syntax`, which the spec runs for both headers.
fn parses_the_issue_7528_sample(header: &str) {
    let sample = format!(
        "{header}
      Agricultural 'waste',Bio-conversion,124.729
      Bio-conversion,Liquid,0.597
      Bio-conversion,Losses,26.862
      Bio-conversion,Solid,280.322
      Bio-conversion,Gas,81.144
      Biofuel imports,Liquid,35
      Biomass imports,Solid,35
      Coal imports,Coal,11.606
      Coal reserves,Coal,63.965
      Coal,Solid,75.571
      District heating,Industry,10.639
      District heating,Heating and cooling - homes,22.505
      District heating,Heating and cooling - commercial,46.184
      Electricity grid,Over generation / exports,104.453
      Electricity grid,Lighting & appliances - commercial,113.726
      Electricity grid,Lighting & appliances - homes,27.14
      "
    );
    let diagram = parses(&sample);
    assert_eq!(diagram.nodes.len(), 19);
    assert_eq!(diagram.links.len(), 16);
    for id in [
        "Agricultural 'waste'",
        "Heating and cooling - homes",
        "Heating and cooling - commercial",
        "Over generation / exports",
        "Lighting & appliances - commercial",
        "Lighting & appliances - homes",
    ] {
        assert!(node_ids(&diagram).contains(&id), "missing node {id:?}");
    }
}
