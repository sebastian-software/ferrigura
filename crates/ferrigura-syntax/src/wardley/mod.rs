//! The wardley map parser (P3): `packages/parser/src/language/wardley/` (the Langium grammar
//! `wardley-beta`) and the wardley DB of `packages/mermaid/src/diagrams/wardley/`, ported to Rust.
//! The plan, the lexer's order and the quirks kept are in `docs-port/wardley/PLAN.md`.
//!
//! The text goes through the reference's steps before the grammar sees it (`Diagram.fromText`):
//! front matter, directives, comment lines, entities, and a line break. The grammar gives the
//! statements, and `populate` makes the DB calls in the order `wardleyParser.ts` gives them. The
//! first failing call is the error.

mod db;
mod grammar;
mod scanner;

use ferrigura_model::wardley::{Flow, NodeClass, WardleyDiagram};

use self::db::{Db, NodeCall};
use self::grammar::{Ast, Label, Stage};
use crate::error::{ParseError, Source};
use crate::preprocess::{cleanup_comments, encode_entities, prepare, remove_directives};
use crate::text::js_trim;

/// Parses a wardley map, with its front matter, and returns its model.
///
/// # Errors
///
/// A lexer or parser error at the first token that the grammar does not take, or the DB's error at
/// the statement that caused it. Both are rejections, and the message and the position say why.
pub fn parse(raw: &str) -> Result<WardleyDiagram, ParseError> {
    let prepared = prepare(raw)?;
    let cleaned = cleanup_comments(&remove_directives(&prepared.text));
    let text = encode_entities(&cleaned) + "\n";
    let source = Source::new(&text, prepared.line_offset);
    let ast = grammar::parse_text(source)?;

    let mut db = Db::new();
    // `Diagram.fromText` sets the front matter title before the parse; the grammar's title
    // replaces it when the grammar gives a non-empty one (`populateCommonDb`).
    if let Some(title) = &prepared.title {
        db.set_title(title);
    }
    populate(&ast, &mut db, source)?;
    Ok(db.into_model())
}

/// The DB calls of `wardleyParser.ts`'s `populateDb`, in its order.
fn populate(ast: &Ast, db: &mut Db, source: Source<'_>) -> Result<(), ParseError> {
    let fail = |offset: usize| move |message: String| source.error(offset, message);

    if !ast.acc_descr.is_empty() {
        db.set_acc_descr(&ast.acc_descr);
    }
    if !ast.acc_title.is_empty() {
        db.set_acc_title(&ast.acc_title);
    }
    if !ast.title.is_empty() {
        db.set_title(&ast.title);
    }
    if let Some(size) = ast.size {
        db.set_size(size.width, size.height);
    }
    if let Some(evolution) = &ast.evolution {
        let stages = evolution.stages.iter().map(stage_label).collect();
        let boundaries = evolution
            .stages
            .iter()
            .filter_map(|stage| stage.boundary)
            .collect();
        db.update_axes(stages, boundaries);
    }
    for anchor in &ast.anchors {
        let context = format!("Anchor \"{}\"", anchor.name);
        let (x, y) = to_coordinates(anchor.visibility, anchor.evolution, &context)
            .map_err(fail(anchor.offset))?;
        db.add_node(NodeCall {
            id: anchor.name.clone(),
            label: anchor.name.clone(),
            x,
            y,
            class_name: NodeClass::Anchor,
            label_offset_x: None,
            label_offset_y: None,
            inertia: None,
            source_strategy: None,
        });
    }
    for component in &ast.components {
        let context = format!("Component \"{}\"", component.name);
        let (x, y) = to_coordinates(component.visibility, component.evolution, &context)
            .map_err(fail(component.offset))?;
        let (label_offset_x, label_offset_y) = label_offsets(component.label);
        db.add_node(NodeCall {
            id: component.name.clone(),
            label: component.name.clone(),
            x,
            y,
            class_name: NodeClass::Component,
            label_offset_x,
            label_offset_y,
            inertia: Some(component.inertia),
            source_strategy: component.decorator,
        });
    }
    for note in &ast.notes {
        let context = format!("Note \"{}\"", note.text);
        let (x, y) =
            to_coordinates(note.visibility, note.evolution, &context).map_err(fail(note.offset))?;
        db.add_note(note.text.clone(), x, y);
    }
    for pipeline in &ast.pipelines {
        let Some(parent_y) = db.get_node(&pipeline.parent).map(|node| node.y) else {
            return Err(source.error(
                pipeline.offset,
                format!(
                    "Pipeline \"{}\" must reference an existing component with coordinates.",
                    pipeline.parent
                ),
            ));
        };
        db.start_pipeline(&pipeline.parent);
        for component in &pipeline.components {
            let id = format!("{}_{}", pipeline.parent, component.name);
            let context = format!("Pipeline component \"{}\" evolution", component.name);
            let x = to_percent(component.evolution, &context).map_err(fail(component.offset))?;
            let (label_offset_x, label_offset_y) = label_offsets(component.label);
            db.add_node(NodeCall {
                id: id.clone(),
                label: component.name.clone(),
                x,
                y: parent_y,
                class_name: NodeClass::PipelineComponent,
                label_offset_x,
                label_offset_y,
                inertia: None,
                source_strategy: None,
            });
            db.add_pipeline_component(&pipeline.parent, &id);
        }
    }
    for link in &ast.links {
        let arrow = link.arrow.as_deref();
        let dashed = arrow.is_some_and(|arrow| arrow.contains("-.->") || arrow.contains(".-."));
        let port_flow = flow_of_port(link.from_port).or_else(|| flow_of_port(link.to_port));
        let (arrow_flow, flow_label) = flow_of_arrow(arrow);
        let flow = port_flow.or(arrow_flow);
        let label = flow_label.or_else(|| link.link_label.clone());
        let source_id = db.resolve_node_id(&link.from);
        let target_id = db.resolve_node_id(&link.to);
        db.add_link(source_id, target_id, dashed, label, flow);
    }
    for evolve in &ast.evolves {
        if let Some(node_y) = db.get_node(&evolve.component).map(|node| node.y) {
            let context = format!("Evolve target for \"{}\"", evolve.component);
            let target = to_percent(evolve.target, &context).map_err(fail(evolve.offset))?;
            db.add_trend(&evolve.component, target, node_y);
        }
    }
    if let Some(first) = ast.annotations.first() {
        let (x, y) =
            to_coordinates(first.x, first.y, "Annotations box").map_err(fail(first.offset))?;
        db.set_annotations_box(x, y);
    }
    for annotation in &ast.annotation {
        let context = format!("Annotation {}", annotation.number);
        let (x, y) = to_coordinates(annotation.x, annotation.y, &context)
            .map_err(fail(annotation.offset))?;
        db.add_annotation(annotation.number, x, y, annotation.text.clone());
    }
    for accelerator in &ast.accelerators {
        let context = format!("Accelerator \"{}\"", accelerator.name);
        let (x, y) = to_coordinates(accelerator.x, accelerator.y, &context)
            .map_err(fail(accelerator.offset))?;
        db.add_accelerator(accelerator.name.clone(), x, y);
    }
    for deaccelerator in &ast.deaccelerators {
        let context = format!("Deaccelerator \"{}\"", deaccelerator.name);
        let (x, y) = to_coordinates(deaccelerator.x, deaccelerator.y, &context)
            .map_err(fail(deaccelerator.offset))?;
        db.add_deaccelerator(deaccelerator.name.clone(), x, y);
    }
    Ok(())
}

/// The name of an evolution stage: the name, and `Name / Second` when the second name is not
/// empty. Both are trimmed.
fn stage_label(stage: &Stage) -> String {
    match &stage.second_name {
        Some(second) if !second.is_empty() => {
            format!("{} / {}", js_trim(&stage.name), js_trim(second))
        }
        _ => js_trim(&stage.name).to_owned(),
    }
}

/// The label offsets of a component or a pipeline component, with their signs.
fn label_offsets(label: Option<Label>) -> (Option<f64>, Option<f64>) {
    match label {
        Some(label) => (
            Some(sign(label.neg_x) * label.offset_x),
            Some(sign(label.neg_y) * label.offset_y),
        ),
        None => (None, None),
    }
}

fn sign(negative: bool) -> f64 {
    if negative { -1.0 } else { 1.0 }
}

/// `toPercent`: a value up to 1 is a fraction and becomes a percentage; a value above 1 is taken as
/// a percentage already. The result must be between 0 and 100.
fn to_percent(value: f64, context: &str) -> Result<f64, String> {
    let normalized = if value <= 1.0 { value * 100.0 } else { value };
    if !(0.0..=100.0).contains(&normalized) {
        return Err(format!(
            "{context} must be between 0-1 (decimal) or 0-100 (percentage). Received: {value}"
        ));
    }
    Ok(normalized)
}

/// `toCoordinates(visibility, evolution)`: the evolution is checked first, then the visibility.
/// The result is `(x, y)` = (evolution, visibility) as percentages.
fn to_coordinates(visibility: f64, evolution: f64, context: &str) -> Result<(f64, f64), String> {
    let x = to_percent(evolution, &format!("{context} evolution"))?;
    let y = to_percent(visibility, &format!("{context} visibility"))?;
    Ok((x, y))
}

/// The flow of a link's port: `+<>` is bidirectional, `+<` backward and `+>` forward.
fn flow_of_port(port: Option<&str>) -> Option<Flow> {
    match port? {
        "+<>" => Some(Flow::Bidirectional),
        "+<" => Some(Flow::Backward),
        "+>" => Some(Flow::Forward),
        _ => None,
    }
}

/// `extractFlowFromArrow`: for a `+'label'` arrow, the flow the arrow's text gives (`<>` first,
/// then `<`, then `>`, as the reference tests them) and the label between the quotes. Any other
/// arrow gives nothing.
fn flow_of_arrow(arrow: Option<&str>) -> (Option<Flow>, Option<String>) {
    let Some(arrow) = arrow.filter(|arrow| arrow.starts_with('+')) else {
        return (None, None);
    };
    let label = arrow
        .strip_prefix("+'")
        .and_then(|rest| rest.find('\'').map(|close| rest[..close].to_owned()));
    let flow = if arrow.contains("<>") {
        Some(Flow::Bidirectional)
    } else if arrow.contains('<') {
        Some(Flow::Backward)
    } else if arrow.contains('>') {
        Some(Flow::Forward)
    } else {
        None
    };
    (flow, label)
}
