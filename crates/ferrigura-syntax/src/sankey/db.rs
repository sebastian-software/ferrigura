//! The sankey DB, `sankeyDB.ts`: the nodes in the order the DB created them, the links in input order,
//! and the title. The DB sanitises every name with the flowchart's DOMPurify subset, and nodes are
//! keyed by the sanitised name, so two names that sanitise alike are one node.

use std::collections::HashMap;

use ferrigura_model::sankey::{SankeyDiagram, SankeyLink, SankeyNode};

use crate::sanitize::sanitize_text;

/// The state the sankey grammar builds.
#[derive(Debug, Default)]
pub(crate) struct SankeyDb {
    title: String,
    nodes: Vec<String>,
    index: HashMap<String, usize>,
    links: Vec<RawLink>,
}

/// A link as the DB holds it: the nodes it joins, by their position in `nodes`.
#[derive(Debug)]
struct RawLink {
    source: usize,
    target: usize,
    value: f64,
}

impl SankeyDb {
    /// `setDiagramTitle`: the title sanitised.
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `findOrCreateNode`: the name is sanitised, and the node is created when no node has that id.
    /// Returns the node's position.
    pub(crate) fn find_or_create_node(&mut self, name: &str) -> usize {
        let id = sanitize_text(name);
        if let Some(&index) = self.index.get(&id) {
            return index;
        }
        let index = self.nodes.len();
        self.index.insert(id.clone(), index);
        self.nodes.push(id);
        index
    }

    /// `addLink`: the link is kept, duplicates included.
    pub(crate) fn add_link(&mut self, source: usize, target: usize, value: f64) {
        self.links.push(RawLink {
            source,
            target,
            value,
        });
    }

    /// The model: the title, the nodes in creation order and the links in input order, with the
    /// node ids that `getGraph` gives them.
    pub(crate) fn into_model(self) -> SankeyDiagram {
        let mut model = SankeyDiagram::new();
        model.title = self.title;
        model.nodes = self
            .nodes
            .iter()
            .map(|id| SankeyNode { id: id.clone() })
            .collect();
        model.links = self
            .links
            .iter()
            .map(|link| SankeyLink {
                source: self.nodes[link.source].clone(),
                target: self.nodes[link.target].clone(),
                value: link.value,
            })
            .collect();
        model
    }
}
