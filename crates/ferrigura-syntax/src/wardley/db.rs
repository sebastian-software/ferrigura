//! The wardley DB, `wardleyDb.ts` over `wardleyBuilder.ts`: the state that the `populate` calls
//! change, and `getWardleyData` reads out. The order of the calls and the quirks kept are in
//! `docs-port/wardley/PLAN.md`, sections 2 and 3.
//!
//! The reference's maps keep insertion order, and `Map.set` on a key that is there keeps its place
//! and takes the new value, as `IndexMap::insert` does. A node's merge keeps what the new call
//! leaves undefined for the label offsets and the class, and overwrites the inertia and the
//! strategy even with nothing, because the DB passes those keys on every call.

use ferrigura_model::wardley::{
    Annotation, Axes, Flow, Link, Mark, Node, NodeClass, Note, Number, Pipeline, Point, Size,
    Strategy, Trend, WardleyDiagram,
};
use indexmap::IndexMap;

use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The arguments of `addNode`, with the values the DB passes. `None` is an `undefined` argument.
pub(crate) struct NodeCall {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub class_name: NodeClass,
    pub label_offset_x: Option<f64>,
    pub label_offset_y: Option<f64>,
    pub inertia: Option<bool>,
    pub source_strategy: Option<Strategy>,
}

/// The wardley DB's state.
#[derive(Debug, Default)]
pub(crate) struct Db {
    title: String,
    acc_title: String,
    acc_descr: String,
    size: Option<Size>,
    axes: Axes,
    nodes: IndexMap<String, Node>,
    links: Vec<Link>,
    trends: IndexMap<String, Trend>,
    pipelines: IndexMap<String, Pipeline>,
    annotations: Vec<Annotation>,
    notes: Vec<Note>,
    accelerators: Vec<Mark>,
    deaccelerators: Vec<Mark>,
    annotations_box: Option<Point>,
}

impl Db {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `setDiagramTitle`: the text is sanitised.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: the text is sanitised, and its leading whitespace is removed.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: the text is sanitised, and the whitespace after each line break is
    /// removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setSize`: replaces the size.
    pub(crate) fn set_size(&mut self, width: f64, height: f64) {
        self.size = Some(Size {
            width: Number(width),
            height: Number(height),
        });
    }

    /// `updateAxes`: `setAxes` merges the two keys, which the populate call always passes.
    pub(crate) fn update_axes(&mut self, stages: Vec<String>, boundaries: Vec<f64>) {
        self.axes = Axes {
            stages: Some(stages),
            stage_boundaries: Some(boundaries.into_iter().map(Number).collect()),
        };
    }

    /// `addNode`: the new call's fields are spread over the node that is there, except that the
    /// class and the label offsets keep the old value when the call gives none. The first position
    /// of the id is kept.
    pub(crate) fn add_node(&mut self, call: NodeCall) {
        let merged = match self.nodes.get(&call.id) {
            Some(existing) => Node {
                id: call.id.clone(),
                label: call.label,
                x: call.x,
                y: call.y,
                class_name: Some(call.class_name),
                label_offset_x: call.label_offset_x.map(Number).or(existing.label_offset_x),
                label_offset_y: call.label_offset_y.map(Number).or(existing.label_offset_y),
                inertia: call.inertia,
                source_strategy: call.source_strategy,
                in_pipeline: existing.in_pipeline,
                is_pipeline_parent: existing.is_pipeline_parent,
            },
            None => Node {
                id: call.id.clone(),
                label: call.label,
                x: call.x,
                y: call.y,
                class_name: Some(call.class_name),
                label_offset_x: call.label_offset_x.map(Number),
                label_offset_y: call.label_offset_y.map(Number),
                inertia: call.inertia,
                source_strategy: call.source_strategy,
                in_pipeline: None,
                is_pipeline_parent: None,
            },
        };
        self.nodes.insert(call.id, merged);
    }

    /// `addLink`: links are kept in the order they are added.
    pub(crate) fn add_link(
        &mut self,
        source: String,
        target: String,
        dashed: bool,
        label: Option<String>,
        flow: Option<Flow>,
    ) {
        self.links.push(Link {
            source,
            target,
            dashed,
            label,
            flow,
        });
    }

    /// `addTrend`: a map keyed by the node id, so the last trend of a node wins.
    pub(crate) fn add_trend(&mut self, node_id: &str, target_x: f64, target_y: f64) {
        self.trends.insert(
            node_id.to_owned(),
            Trend {
                node_id: node_id.to_owned(),
                target_x,
                target_y,
            },
        );
    }

    /// `startPipeline`: replaces the parent's pipeline with an empty one, and marks the node.
    pub(crate) fn start_pipeline(&mut self, node_id: &str) {
        self.pipelines.insert(
            node_id.to_owned(),
            Pipeline {
                node_id: node_id.to_owned(),
                component_ids: Vec::new(),
            },
        );
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.is_pipeline_parent = Some(true);
        }
    }

    /// `addPipelineComponent`: appends the component to its parent's pipeline, and marks the node.
    pub(crate) fn add_pipeline_component(&mut self, parent: &str, component_id: &str) {
        if let Some(pipeline) = self.pipelines.get_mut(parent) {
            pipeline.component_ids.push(component_id.to_owned());
        }
        if let Some(node) = self.nodes.get_mut(component_id) {
            node.in_pipeline = Some(true);
        }
    }

    /// `addNote`.
    pub(crate) fn add_note(&mut self, text: String, x: f64, y: f64) {
        self.notes.push(Note { text, x, y });
    }

    /// `setAnnotationsBox`: replaces the box.
    pub(crate) fn set_annotations_box(&mut self, x: f64, y: f64) {
        self.annotations_box = Some(Point { x, y });
    }

    /// `addAnnotation`.
    pub(crate) fn add_annotation(&mut self, number: f64, x: f64, y: f64, text: String) {
        self.annotations.push(Annotation {
            number: Number(number),
            coordinates: vec![Point { x, y }],
            text,
        });
    }

    /// `addAccelerator`.
    pub(crate) fn add_accelerator(&mut self, name: String, x: f64, y: f64) {
        self.accelerators.push(Mark { name, x, y });
    }

    /// `addDeaccelerator`.
    pub(crate) fn add_deaccelerator(&mut self, name: String, x: f64, y: f64) {
        self.deaccelerators.push(Mark { name, x, y });
    }

    /// `getNode`: the node with this id, if any.
    pub(crate) fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// `resolveNodeId`: an exact id first, then the first node whose label is the name, else the
    /// name itself.
    pub(crate) fn resolve_node_id(&self, name: &str) -> String {
        if self.nodes.contains_key(name) {
            return name.to_owned();
        }
        self.nodes
            .iter()
            .find(|(_, node)| node.label == name)
            .map_or_else(|| name.to_owned(), |(id, _)| id.clone())
    }

    /// `getWardleyData`: the state as the model keeps it.
    pub(crate) fn into_model(self) -> WardleyDiagram {
        WardleyDiagram {
            version: ferrigura_model::CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            size: self.size,
            axes: self.axes,
            nodes: self.nodes.into_values().collect(),
            links: self.links,
            trends: self.trends.into_values().collect(),
            pipelines: self.pipelines.into_values().collect(),
            annotations: self.annotations,
            notes: self.notes,
            accelerators: self.accelerators,
            deaccelerators: self.deaccelerators,
            annotations_box: self.annotations_box,
        }
    }
}

#[cfg(test)]
mod tests {
    //! `wardleyBuilder.spec.ts`: `WardleyBuilder.resolveNodeId`, called on the DB's state directly.
    use super::*;

    fn node(db: &mut Db, id: &str, label: &str, class_name: NodeClass) {
        db.add_node(NodeCall {
            id: id.to_owned(),
            label: label.to_owned(),
            x: 0.0,
            y: 0.0,
            class_name,
            label_offset_x: None,
            label_offset_y: None,
            inertia: None,
            source_strategy: None,
        });
    }

    #[test]
    fn returns_the_id_when_a_node_exists_with_that_exact_id() {
        let mut db = Db::new();
        node(&mut db, "API", "API", NodeClass::Component);
        assert_eq!(db.resolve_node_id("API"), "API");
    }

    #[test]
    fn falls_back_to_label_match_for_pipeline_style_synthetic_ids() {
        let mut db = Db::new();
        node(
            &mut db,
            "Kettle_Campfire Kettle",
            "Campfire Kettle",
            NodeClass::PipelineComponent,
        );
        assert_eq!(
            db.resolve_node_id("Campfire Kettle"),
            "Kettle_Campfire Kettle"
        );
    }

    #[test]
    fn returns_the_input_unchanged_when_no_id_or_label_matches() {
        let mut db = Db::new();
        node(&mut db, "API", "API", NodeClass::Component);
        assert_eq!(db.resolve_node_id("Unknown"), "Unknown");
    }

    #[test]
    fn prefers_exact_id_over_label_match_when_both_exist() {
        let mut db = Db::new();
        node(&mut db, "Alpha", "Beta", NodeClass::Component);
        node(&mut db, "Beta", "Gamma", NodeClass::Component);
        assert_eq!(db.resolve_node_id("Beta"), "Beta");
    }
}
