//! The blocks as nodes and the edges between them: `getNodeFromBlock` (`renderHelpers.ts` 16-136),
//! the measure pass (`calculateBlockSize`, 138-156), the placed pass (`insertBlockPositioned`, 158-187)
//! and `insertEdges` (215-302).

use std::collections::HashMap;

use ferrigura_model::block::{ClassDef, Edge as ModelEdge};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use super::{Anchor, Anchors, Block, Size, font_size};
use crate::RenderError;
use crate::unified::data::{BlockFields, Edge, Node};
use crate::unified::edges;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::{Anchor as TextAnchor, LabelLayout};
use crate::unified::palette::Palette;
use crate::unified::shapes::{self, LinkForm, ShapeInput, ShapeOutput, styles_from_array};

/// What every block of one render reads: the configuration, the theme, the `classDef`s and the palette.
pub(super) struct Context<'a> {
    /// The effective configuration.
    pub config: &'a MermaidConfig,
    /// The theme variables that give the sizes.
    pub theme: &'a Value,
    /// The `classDef`s of the diagram, in the order they were declared.
    pub class_defs: &'a [ClassDef],
    /// The diagram's id.
    pub id: &'a str,
    /// The palette of a colour theme, `None` for every other theme (`isColorTheme`).
    pub palette: Option<Palette>,
}

/// What the placed pass has drawn so far: the groups of the blocks and the edges, the box they cover,
/// the labels, and what the edges need of each block.
#[derive(Default)]
pub(super) struct Drawing {
    /// The `g.block` content: the blocks in tree order, then the edges and their labels.
    pub body: String,
    /// The box the drawing covers, `getBBox` of `g.block` (before the float32 rounding).
    pub bounds: Option<BBox>,
    /// The labels the drawing writes, for the font subsets.
    pub labels: Vec<LabelLayout>,
    /// The size of every block, by id.
    pub sizes: HashMap<String, Size>,
    /// The intersect of every drawn block, by id.
    pub anchors: Anchors,
}

/// A number that JavaScript reads as true: not zero and not NaN.
fn truthy(value: f64) -> Option<f64> {
    (value != 0.0 && !value.is_nan()).then_some(value)
}

/// The shape a block type draws (`getNodeFromBlock`, 33-97): the shape's name, and the radius of a
/// rounded rectangle.
fn shape_of(block_type: Option<&str>) -> (&'static str, Option<f64>) {
    match block_type {
        Some("round") => ("rect", Some(5.0)),
        Some("composite") => ("composite", None),
        Some("diamond") => ("question", None),
        Some("hexagon") => ("hexagon", None),
        Some("block_arrow") => ("block_arrow", None),
        Some("odd" | "rect_left_inv_arrow") => ("rect_left_inv_arrow", None),
        Some("lean_right") => ("lean_right", None),
        Some("lean_left") => ("lean_left", None),
        Some("trapezoid") => ("trapezoid", None),
        Some("inv_trapezoid") => ("inv_trapezoid", None),
        Some("circle") => ("circle", None),
        Some("ellipse") => ("ellipse", None),
        Some("stadium") => ("stadium", None),
        Some("subroutine") => ("subroutine", None),
        Some("cylinder") => ("cylinder", None),
        Some("doublecircle") => ("doublecircle", None),
        _ => ("rect", None),
    }
}

/// `getNodeFromBlock`: the node the shapes draw for a block. `placed` is the block's size after the
/// layout; the measure pass has none, so the label does not wrap (`wrappingWidth` is infinite).
fn node_of(block: &Block, context: &Context<'_>, placed: Option<Size>) -> Node {
    let mut classes = if block.classes.is_empty() {
        "default".to_owned()
    } else {
        block.classes.join(" ")
    };
    classes.push_str(" flowchart-label");
    let compiled: Vec<String> = block
        .classes
        .iter()
        .flat_map(|name| {
            context
                .class_defs
                .iter()
                .filter(move |class| class.id == *name)
                .flat_map(|class| class.styles.iter().cloned())
        })
        .collect();
    let (shape, radius) = shape_of(block.block_type.as_deref());
    let composite = block.block_type.as_deref() == Some("composite");
    let (_, label_style) = styles_from_array(&block.styles);
    let width = placed.and_then(|size| truthy(size.width));
    let wrapping_width = width.unwrap_or(f64::INFINITY);
    Node {
        id: block.id.clone(),
        dom_id: block.id.clone(),
        label: block.label.clone(),
        label_style,
        css_styles: block.styles.clone(),
        css_compiled_styles: compiled,
        css_classes: classes,
        look: context.config.look.clone(),
        shape: shape.to_owned(),
        // A composite has no padding of its own (`composite`, 'padding = 0' in `getNodeFromBlock`).
        padding: if composite {
            0.0
        } else {
            context.config.block.padding
        },
        rx: radius,
        ry: radius,
        width,
        height: placed.and_then(|size| truthy(size.height)),
        wrapping_width: Some(wrapping_width),
        x: placed.map(|size| size.x),
        y: placed.map(|size| size.y),
        color_index: block.color_index,
        block: Some(BlockFields {
            directions: block.directions.clone(),
            width_in_columns: block.width_in_columns.unwrap_or(1.0),
            positioned: placed.is_some(),
        }),
        ..Node::default()
    }
}

/// The shape of a block, laid out and drawn, with its label (`insertNode` and its handler).
fn draw_block(
    block: &Block,
    context: &Context<'_>,
    placed: Option<Size>,
    width: Option<f64>,
) -> Result<(Node, ShapeOutput), RenderError> {
    let node = node_of(block, context, placed);
    let label = shapes::node_label(
        &node,
        context.config,
        context.theme,
        width,
        TextAnchor::Middle,
    )?;
    let shape = shapes::draw(&ShapeInput {
        node: &node,
        label: label.as_ref(),
        config: context.config,
        theme: context.theme,
        dir: None,
    })?;
    Ok((node, shape))
}

/// `calculateBlockSize` (`renderHelpers.ts` 138-156): the size of a block drawn with nothing wrapping,
/// the float32 `getBBox` of its group.
fn measure(block: &Block, context: &Context<'_>) -> Result<Size, RenderError> {
    let (_, shape) = draw_block(block, context, None, Some(f64::INFINITY))?;
    let bbox = shape.bbox.browser();
    Ok(Size {
        width: bbox.width,
        height: bbox.height,
        x: 0.0,
        y: 0.0,
    })
}

/// The measure pass over the blocks, in tree order: each block is measured before its children.
pub(super) fn measure_all(block: &mut Block, context: &Context<'_>) -> Result<(), RenderError> {
    for child in &mut block.children {
        child.size = Some(measure(child, context)?);
        measure_all(child, context)?;
    }
    Ok(())
}

/// The placed pass over the blocks, in tree order, parents before their children: a block that is not a
/// space cell is drawn at its final size, and its anchor is kept for the edges (`insertBlockPositioned`).
pub(super) fn place_all(
    block: &Block,
    context: &Context<'_>,
    drawing: &mut Drawing,
) -> Result<(), RenderError> {
    for child in &block.children {
        let size = child.size.unwrap_or_default();
        drawing.sizes.insert(child.id.clone(), size);
        if !child.is_space() {
            place(child, size, context, drawing)?;
        }
        place_all(child, context, drawing)?;
    }
    Ok(())
}

/// Draws one block at its size, with the palette slot of a composite (`stampColorSlot`), and records
/// what it covers and how edges meet it.
fn place(
    block: &Block,
    size: Size,
    context: &Context<'_>,
    drawing: &mut Drawing,
) -> Result<(), RenderError> {
    let (node, mut shape) = draw_block(block, context, Some(size), None)?;
    if let (Some(index), Some(palette)) = (
        node.color_index,
        context
            .palette
            .as_ref()
            .filter(|palette| !palette.border.is_empty()),
    ) {
        let slot = crate::unified::palette::palette_slot(Some(index), palette).unwrap_or(0);
        shape.group_attributes = format!(" data-color-id=\"color-{slot}\"");
    }
    let x = node.x.unwrap_or(0.0);
    let y = node.y.unwrap_or(0.0);
    let svg = shapes::node_svg(&node, &shape, context.id, LinkForm::Flowchart);
    drawing.body.push_str(&svg);
    crate::unified::pipeline::cover(
        &mut drawing.bounds,
        shape.extent.unwrap_or(shape.bbox).translate(x, y),
    );
    drawing.labels.extend(shape.label.clone());
    drawing.labels.extend(shape.texts.iter().cloned());
    drawing.anchors.insert(
        block.id.clone(),
        Anchor {
            intersect: shape.intersect.clone(),
            element: shape.element,
            centre: Point::new(x, y),
        },
    );
    Ok(())
}

/// `insertEdges` (`renderHelpers.ts` 215-302): each edge with both ends drawn joins the middle of the
/// two blocks, clipped to their outlines. The edge's label, when it has one, is placed at the middle.
pub(super) fn insert_edges(
    edges_model: &[ModelEdge],
    context: &Context<'_>,
    drawing: &mut Drawing,
) -> Result<(), RenderError> {
    let font = font_size(context.theme);
    let mut built: Vec<(Edge, String, Option<LabelLayout>)> = Vec::new();
    for model in edges_model {
        let (Some(start), end) = (model.start.as_deref(), model.end.as_str()) else {
            continue;
        };
        let (Some(start_size), Some(end_size)) = (
            drawing.sizes.get(start).copied(),
            drawing.sizes.get(end).copied(),
        ) else {
            continue;
        };
        let start_centre = Point::new(start_size.x, start_size.y);
        let mid = Point::new(
            start_size.x + (end_size.x - start_size.x) / 2.0,
            start_size.y + (end_size.y - start_size.y) / 2.0,
        );
        let end_centre = Point::new(end_size.x, end_size.y);
        // `insertEdge` clips the edge to the outlines of both blocks, through their intersect functions;
        // the clipping of the default layout takes the middle point only (`edges.js` 699-703).
        let points = match (drawing.anchors.get(start), drawing.anchors.get(end)) {
            (Some(tail), Some(head)) => vec![tail.meet(mid), mid, head.meet(mid)],
            _ => vec![start_centre, mid, end_centre],
        };

        let prefixed = format!("{}-{}", context.id, model.id);
        let thick = model.thickness == "thick";
        let dotted = model.pattern == "dotted";
        let thickness_class = if thick {
            "edge-thickness-thick"
        } else {
            "edge-thickness-normal"
        };
        let pattern_class = if dotted {
            "edge-pattern-dotted"
        } else {
            "edge-pattern-solid"
        };
        let edge = Edge {
            id: format!("{}-{prefixed}", context.id),
            start: start.to_owned(),
            end: end.to_owned(),
            label: model.label.clone(),
            thickness: model.thickness.clone(),
            pattern: model.pattern.clone(),
            classes: format!("{thickness_class} {pattern_class} flowchart-link LS-a1 LE-b1"),
            arrow_type_start: model.arrow_type_start.clone(),
            arrow_type_end: model.arrow_type_end.clone(),
            // `edge.style` is undefined for a block edge, so the path's style reads `undefined;;;undefined`.
            style: vec!["undefined".to_owned()],
            look: context.config.look.clone(),
            points,
            x: Some(mid.x),
            y: Some(mid.y),
            label_style: vec![
                "stroke: #333".to_owned(),
                "stroke-width: 1.5px".to_owned(),
                "fill:none".to_owned(),
            ],
            ..Edge::default()
        };
        let label = edges::label_with_font_size(&edge, font)?;
        built.push((edge, model.id.clone(), label));
    }

    let paths: Vec<Edge> = built.iter().map(|(edge, _, _)| edge.clone()).collect();
    for path in edges::paint_paths(&paths, context.id, "block", context.config) {
        crate::unified::pipeline::cover(&mut drawing.bounds, path.bbox);
        drawing.body.push_str(&path.svg);
    }
    for (edge, model_id, label) in &built {
        let Some(label) = label else {
            continue;
        };
        // The label's `data-id` is the edge's id as the model gives it, without the diagram's prefix.
        let labelled = Edge {
            id: model_id.clone(),
            ..edge.clone()
        };
        let painted = edges::paint_label(&labelled, label, context.config);
        crate::unified::pipeline::cover(&mut drawing.bounds, painted.bbox);
        drawing.body.push_str(&painted.svg);
        drawing.labels.push(label.clone());
    }
    Ok(())
}
