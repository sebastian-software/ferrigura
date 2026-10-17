//! The block diagram: a port of `diagrams/block/blockRenderer.ts`, `renderHelpers.ts`, `layout.ts`
//! and `styles.ts`, with the node shapes, the edges and the markers of the unified renderer
//! (`unified/`), where the reference uses the shared rendering code.
//!
//! The steps follow the reference: every block is measured with nothing wrapping (`calculateBlockSizes`),
//! the blocks are laid out by the type's own arithmetic (`layout.rs`), each block is drawn at its final
//! size, where its label wraps (`insertBlocks`), the edges join the drawn blocks (`insertEdges`), and the
//! root `svg` takes the box that the drawing covers (`blockRenderer.ts` 62-79). Every size is the float32
//! `getBBox` of the drawn group, as the browser gives it. See `docs-port/block/NOTES.md`, section 6.

pub mod layout;
mod nodes;
pub mod style;

use std::collections::HashMap;

use ferrigura_model::block::{Block as ModelBlock, BlockDiagram};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_text::Run;

use crate::RenderError;
use crate::a11y;
use crate::flowchart::style::look_defs;
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::unified::geom::{BBox, CenterBox, Point};
use crate::unified::markers;
use crate::unified::shapes::Intersect;
use crate::variables::px;

/// What the block renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct BlockInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a BlockDiagram,
    /// The effective configuration, in the block diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// The size of a block: its width and height, and the centre the layout gives it (`block.size`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The x of the centre, after the layout.
    pub x: f64,
    /// The y of the centre, after the layout.
    pub y: f64,
}

/// A block of the tree the renderer lays out: a block, a space cell, or a composite with its children,
/// as the model gives them. The root has the id `root`; the first block of a chained link, which has no
/// id, has `undefined` (`docs-port/block/PLAN.md`, B2).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The id, as the DB holds it.
    pub id: String,
    /// The type: `na`, `square`, `round`, `composite`, `space`, `block_arrow`, … The id-less block has
    /// none.
    pub block_type: Option<String>,
    /// The label as the renderer draws it; empty when the block has none.
    pub label: String,
    /// The columns of a composite (`block.columns`), `None` when it has no `columns` statement.
    pub columns: Option<f64>,
    /// The span of the block (`widthInColumns`), `None` when the statement gives none.
    pub width_in_columns: Option<f64>,
    /// The directions of a block arrow, as written.
    pub directions: Vec<String>,
    /// The classes applied with `class`, in order.
    pub classes: Vec<String>,
    /// The styles of the block, as written by `style`.
    pub styles: Vec<String>,
    /// The palette slot of a composite (`colorIndex`).
    pub color_index: Option<u32>,
    /// The blocks inside it, empty for a leaf.
    pub children: Vec<Block>,
    /// The size: the measure step gives it, and the layout then sets it.
    pub size: Option<Size>,
}

impl Block {
    /// Whether the block is a space cell: it takes cells of the grid but is not drawn.
    #[must_use]
    pub fn is_space(&self) -> bool {
        self.block_type.as_deref() == Some("space")
    }

    /// The tree of a model block and its children.
    fn from_model(model: &ModelBlock) -> Self {
        Self {
            id: model.id.clone().unwrap_or_else(|| "undefined".to_owned()),
            block_type: model.block_type.clone(),
            label: model.label.clone().unwrap_or_default(),
            columns: model.columns,
            width_in_columns: model.width_in_columns,
            directions: model.directions.clone().unwrap_or_default(),
            classes: model.classes.clone().unwrap_or_default(),
            styles: model.styles.clone().unwrap_or_default(),
            color_index: model.color_index,
            children: model
                .children
                .iter()
                .flatten()
                .map(Self::from_model)
                .collect(),
            size: None,
        }
    }

    /// The root: the diagram's top-level blocks, with the diagram's `columns`.
    fn root(diagram: &BlockDiagram) -> Self {
        Self {
            id: "root".to_owned(),
            block_type: Some("composite".to_owned()),
            label: String::new(),
            columns: diagram.columns,
            width_in_columns: None,
            directions: Vec::new(),
            classes: Vec::new(),
            styles: Vec::new(),
            color_index: None,
            children: diagram.blocks.iter().map(Self::from_model).collect(),
            size: None,
        }
    }
}

/// Renders a block diagram as an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, a shape is unknown, or the grid has zero columns
/// (the reference throws for it: `Columns must be an integer !== 0.`).
pub fn render(input: &BlockInput<'_>) -> Result<String, RenderError> {
    let config = input.config;
    let theme = input.themes.primary();
    let context = nodes::Context {
        config,
        theme,
        class_defs: &input.diagram.class_defs,
        id: input.id,
        palette: crate::unified::palette::palette(config, theme),
    };

    let mut root = Block::root(input.diagram);
    // `calculateBlockSizes`: every block is measured, in tree order, with nothing wrapping.
    nodes::measure_all(&mut root, &context)?;
    // `layout`: the sizes and positions of the blocks.
    let layout_bounds = layout::layout(&mut root, config.block.padding)?;

    // `insertBlocks` and `insertEdges`: the blocks, then the edges with their labels.
    let mut drawing = nodes::Drawing::default();
    nodes::place_all(&root, &context, &mut drawing)?;
    nodes::insert_edges(&input.diagram.edges, &context, &mut drawing)?;

    // The box the drawing covers is `getBBox` of `g.block`; the layout's bounds are the fallback when
    // that box is not finite.
    let drawn = drawing.bounds.unwrap_or_default().browser();
    let bounds = if [drawn.x, drawn.y, drawn.width, drawn.height]
        .iter()
        .all(|value| value.is_finite())
    {
        drawn
    } else {
        BBox::new(
            layout_bounds.x,
            layout_bounds.y,
            layout_bounds.width,
            layout_bounds.height,
        )
    };
    let height = bounds.height + magic_factor(&bounds) + 10.0;
    let width = bounds.width + 10.0;
    let size_attributes = if config.block.use_max_width {
        format!("width=\"100%\" style=\"max-width: {}px;\"", number(width))
    } else {
        format!("height=\"{}\" width=\"{}\"", number(height), number(width))
    };

    let texts: Vec<Run<'_>> = drawing
        .labels
        .iter()
        .flat_map(crate::unified::label::LabelLayout::text_runs)
        .collect();
    let families = ferrigura_text::embed_runs(&texts)?;
    let style = style::style_sheet_with_fonts(
        input.id,
        input.themes,
        config,
        &input.diagram.class_defs,
        &families,
    );
    let accessible = a11y::root_attributes(input.id, "block", "", "");
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} viewBox=\"{x} {y} {w} {h}\"{accessible}><style>{style}</style><g></g>{defs}{markers}<g class=\"block\">{body}</g></svg>",
        id = escape(input.id),
        x = number(bounds.x - 5.0),
        y = number(bounds.y - 5.0),
        w = number(bounds.width + 10.0),
        h = number(bounds.height + 10.0),
        defs = look_defs(input.id, input.themes, config),
        markers = markers::markers(input.id, "block", config),
        body = drawing.body,
    ))
}

/// `magicFactor` (`blockRenderer.ts` 70): `max(1, round(0.125 * width / height))`, which is NaN for a
/// drawing with no height. JavaScript's `Math.max` keeps a NaN, where Rust's `f64::max` would not.
fn magic_factor(bounds: &BBox) -> f64 {
    let factor = (0.125 * (bounds.width / bounds.height) + 0.5).floor();
    if factor.is_nan() {
        factor
    } else {
        factor.max(1.0)
    }
}

/// The intersect of a drawn block, and the size and centre an edge meets it with.
#[derive(Debug, Clone)]
pub(super) struct Anchor {
    /// The shape's intersect function.
    pub intersect: Intersect,
    /// The shape's own box, which `updateNodeBounds` reads as the node's size.
    pub element: BBox,
    /// The centre of the block.
    pub centre: Point,
}

impl Anchor {
    /// The point where the line from the block's centre towards `point` leaves the block.
    pub(super) fn meet(&self, point: Point) -> Point {
        let node = CenterBox::new(
            self.centre.x,
            self.centre.y,
            self.element.width,
            self.element.height,
        );
        self.intersect.intersect(node, point)
    }
}

/// The anchor of every drawn block, by id.
pub(super) type Anchors = HashMap<String, Anchor>;

/// `px` of the theme's `fontSize`, the size of the edge labels (`createText`'s font size).
pub(super) fn font_size(theme: &Value) -> f64 {
    px(theme, "fontSize").unwrap_or(16.0)
}
