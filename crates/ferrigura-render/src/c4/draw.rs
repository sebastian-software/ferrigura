//! The layout and the drawing of the C4 boundaries and elements: `drawInsideBoundary`,
//! `drawC4ShapeArray` and `drawBoundary` (`c4Renderer.ts:246-560`, `svgDraw.ts:135-211`). The
//! recursion writes the elements and the boundaries of the diagram into the body in the reference's
//! order: the elements of a boundary, then its child boundaries, then the boundary itself.

use std::collections::HashMap;

use ferrigura_model::c4::{C4Boundary, C4Diagram, C4Shape};
use ferrigura_text::{Face as TextFace, FontFamily, FontStyle, FontWeight, face};

use super::config::{Conf, Font};
use super::grid::Bounds;
use super::label;
use super::legacy::{self, Face, Legacy, Paint};
use super::shape::{self, Body, Built, plain};
use crate::RenderError;
use crate::common::text::{MeasureError, TextMemo, TextStyle};
use crate::svg::{escape, number};
use crate::unified::geom::{CenterBox, Point};
use crate::unified::shapes::Intersect;

/// The placeholder of the embedded font family in the text that `byTspan` writes. The family is
/// named when the subsets are made, after the text is laid out, so the body refers to it by this token
/// and [`super::render`] replaces it with the family escaped for the attribute value it stands in.
pub const FAMILY: &str = "\u{1}c4-family\u{1}";

/// The stroke that boundaries and relations take when the diagram gives none (`#444444`).
pub const DEFAULT_STROKE: &str = "#444444";

/// The width of the screen the layout compares with (`screen.availWidth`): the harness page's 1200 px
/// (`P2-RENDERERS.md`, screen and container sizes).
pub const SCREEN_WIDTH: f64 = 1200.0;

/// Where an element or a boundary is: its centre, its size, and how a relation meets its outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The centre.
    pub centre: Point,
    /// The width of the node.
    pub width: f64,
    /// The height of the node.
    pub height: f64,
    /// The intersect of the node's outline.
    pub intersect: Intersect,
}

impl Place {
    /// The centre-based box that the intersect functions take.
    #[must_use]
    pub fn box_for_intersect(&self) -> CenterBox {
        CenterBox::new(self.centre.x, self.centre.y, self.width, self.height)
    }
}

/// The state of one draw of a C4 diagram.
pub struct Draw<'a> {
    /// The configuration.
    pub conf: &'a Conf,
    /// The diagram.
    pub model: &'a C4Diagram,
    /// The id of the SVG.
    pub id: &'a str,
    /// The corner radius of the rounded rectangle.
    pub radius: f64,
    /// `c4ShapeInRow`, from the DB.
    pub shape_in_row: f64,
    /// `c4BoundaryInRow`, from the DB.
    pub boundary_in_row: f64,
    /// The text measurement of this draw.
    pub memo: TextMemo,
    /// `globalBoundaryMaxX`: the right edge of everything drawn.
    pub max_x: f64,
    /// `globalBoundaryMaxY`: the bottom edge of everything drawn.
    pub max_y: f64,
    /// The elements and the boundaries, in the order the reference draws them.
    pub body: String,
    /// The placed elements, by alias.
    pub shapes: HashMap<String, Place>,
    /// The placed boundaries, by alias.
    pub boundaries: HashMap<String, Place>,
    /// Every text the diagram sets, for the font subsets.
    pub runs: Vec<(String, &'static TextFace)>,
}

/// The `[type]` of a boundary, when its type is not empty (`drawInsideBoundary`, line 476).
fn bracketed(text: Option<&String>) -> Option<String> {
    text.filter(|text| !text.is_empty())
        .map(|text| format!("[{text}]"))
}

/// A non-empty text of an option, as the reference's `text !== ''` checks it.
fn non_empty(text: Option<&String>) -> Option<&str> {
    text.map(String::as_str).filter(|text| !text.is_empty())
}

/// The face of a legacy text: the weight in its CSS spelling, the size and the family token.
fn legacy_face(size: f64, weight: &str) -> Face<'_> {
    Face {
        size,
        weight,
        family: FAMILY,
    }
}

/// The text face of a weight, for the font subsets.
fn text_face(bold: bool, italic: bool) -> &'static TextFace {
    face(
        FontFamily::Sans,
        if bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        },
        if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
    )
}

/// The text measurement style of a configured font.
pub fn style_of(font: &Font, size: f64, weight: f64) -> TextStyle {
    TextStyle {
        font_size: size,
        font_weight: weight,
        font_family: font.family.clone(),
    }
}

impl Draw<'_> {
    /// The alias of the boundaries whose parent is `parent`, in the DB's order (`getBoundaries`).
    pub(super) fn boundaries_of(&self, parent: &str) -> Vec<usize> {
        self.model
            .boundaries
            .iter()
            .enumerate()
            .filter(|(_, boundary)| boundary.parent_boundary == parent)
            .map(|(index, _)| index)
            .collect()
    }

    /// The indices of the elements whose parent boundary is `parent`, in the DB's order (`getC4ShapeArray`).
    fn shapes_of(&self, parent: &str) -> Vec<usize> {
        self.model
            .shapes
            .iter()
            .enumerate()
            .filter(|(_, shape)| shape.parent_boundary == parent)
            .map(|(index, _)| index)
            .collect()
    }

    /// Records the text of a legacy text for the font subset, line by line.
    pub fn record_legacy(&mut self, text: &str, bold: bool, italic: bool) {
        for line in super::legacy::lines(text) {
            self.runs.push((line.to_owned(), text_face(bold, italic)));
        }
    }

    /// `drawInsideBoundary` (`c4Renderer.ts:435-560`): lays out the child boundaries of the level whose
    /// grid is `parent`, and draws each one with its elements and its own children.
    ///
    /// # Errors
    ///
    /// Returns an error for a text the renderer does not draw, or a font that cannot be read.
    pub fn draw_inside(
        &mut self,
        parent: &mut Bounds,
        children: &[usize],
    ) -> Result<(), RenderError> {
        let model = self.model;
        let conf = self.conf;
        let margin_x = conf.diagram_margin_x;
        let margin_y = conf.diagram_margin_y;
        let width_limit = parent.width_limit / self.boundary_in_row.min(children.len() as f64);
        let mut current = Bounds::new(width_limit);

        for (position, &index) in children.iter().enumerate() {
            let boundary = &model.boundaries[index];
            let alias = boundary.alias.as_str();
            // The header: an image box of 48 px for a sprite (which is reserved, not drawn), then the
            // label, the type and the description, one under the other (lines 452-502).
            let mut y = if non_empty_sprite(boundary) {
                48.0
            } else {
                0.0
            };
            let wrap = boundary.wrap && conf.wrap;
            let label_text = plain(&boundary.label)?;
            let label_style = style_of(&conf.boundary, conf.boundary.size + 2.0, 700.0);
            let mut label =
                legacy::measure(&mut self.memo, &label_text, wrap, &label_style, width_limit)
                    .map_err(text_error)?;
            label.y = y + 8.0;
            y = label.y + label.height;

            let boundary_weight = conf.boundary.weight_number();
            let mut type_text: Option<Legacy> = None;
            if let Some(text) = bracketed(boundary.boundary_type.as_ref()) {
                let style = style_of(&conf.boundary, conf.boundary.size, boundary_weight);
                let mut measured =
                    legacy::measure(&mut self.memo, &text, wrap, &style, width_limit)
                        .map_err(text_error)?;
                measured.y = y + 5.0;
                y = measured.y + measured.height;
                type_text = Some(measured);
            }
            let mut descr_text: Option<Legacy> = None;
            if let Some(text) = non_empty(boundary.descr.as_ref()) {
                let style = style_of(&conf.boundary, conf.boundary.size - 2.0, boundary_weight);
                let mut measured = legacy::measure(&mut self.memo, text, wrap, &style, width_limit)
                    .map_err(text_error)?;
                measured.y = y + 20.0;
                y = measured.y + measured.height;
                descr_text = Some(measured);
            }

            // The position: a new row at the left of the level for the first boundary and every
            // `c4BoundaryInRow`-th one, else the next column (lines 504-519).
            if position == 0 || (position as f64) % self.boundary_in_row == 0.0 {
                let x = parent.data.startx.unwrap_or(0.0) + margin_x;
                let row_y = parent.data.stopy.unwrap_or(0.0) + margin_y + y;
                current.set_data(x, x, row_y, row_y);
            } else {
                let x = if current.data.stopx != current.data.startx {
                    current.data.stopx.unwrap_or(0.0) + margin_x
                } else {
                    current.data.startx.unwrap_or(0.0)
                };
                let row_y = current.data.starty.unwrap_or(0.0);
                current.set_data(x, x, row_y, row_y);
            }

            let shapes = self.shapes_of(alias);
            if !shapes.is_empty() {
                self.draw_shapes(&mut current, &shapes)?;
            }
            let children = self.boundaries_of(alias);
            if !children.is_empty() {
                self.draw_inside(&mut current, &children)?;
            }
            if alias != "global" {
                self.draw_boundary(
                    &current,
                    boundary,
                    &label,
                    type_text.as_ref(),
                    descr_text.as_ref(),
                )?;
            }

            let bump = conf.shape_margin;
            let parent_stopy = parent.data.stopy.unwrap_or(0.0);
            let parent_stopx = parent.data.stopx.unwrap_or(0.0);
            parent.data.stopy = Some((current.data.stopy.unwrap_or(0.0) + bump).max(parent_stopy));
            parent.data.stopx = Some((current.data.stopx.unwrap_or(0.0) + bump).max(parent_stopx));
            self.max_x = self.max_x.max(parent.data.stopx.unwrap_or(0.0));
            self.max_y = self.max_y.max(parent.data.stopy.unwrap_or(0.0));
        }
        Ok(())
    }

    /// `drawC4ShapeArray` (`c4Renderer.ts:282-350`): measures the elements of a level, places them on its
    /// grid, and draws them, each in a group translated to its centre.
    ///
    /// # Errors
    ///
    /// Returns an error for an element the renderer does not draw.
    pub fn draw_shapes(
        &mut self,
        current: &mut Bounds,
        indices: &[usize],
    ) -> Result<(), RenderError> {
        let model = self.model;
        let mut built: Vec<Built> = Vec::with_capacity(indices.len());
        for &index in indices {
            let element: &C4Shape = &model.shapes[index];
            built.push(shape::build(self.conf, self.radius, element)?);
        }
        let margin = self.conf.shape_margin;
        let mut corners = Vec::with_capacity(built.len());
        for element in &built {
            let corner = current.insert(
                element.width,
                element.height,
                margin,
                self.shape_in_row,
                self.conf.next_line_padding_x,
            );
            corners.push(corner);
        }
        for (element, (x, y)) in built.into_iter().zip(corners) {
            let centre = Point::new(x + element.width / 2.0, y + element.height / 2.0);
            let group = element_group(self.id, &element, centre);
            for section in &element.label.sections {
                for run in section.layout.text_runs() {
                    self.runs.push((run.text.to_owned(), run.face));
                }
            }
            self.body.push_str(&group);
            self.shapes.insert(
                element.alias.clone(),
                Place {
                    centre,
                    width: element.width,
                    height: element.height,
                    intersect: element.intersect,
                },
            );
        }
        current.bump_last_margin(margin);
        Ok(())
    }

    /// `drawBoundary` (`svgDraw.ts:135-211`): the dashed box of a boundary, with its label, its type and
    /// its description as legacy texts. The `global` boundary is not drawn (`c4Renderer.ts:546`), so the
    /// caller does not pass it here.
    ///
    /// # Errors
    ///
    /// Returns an error for a font that cannot be read.
    pub fn draw_boundary(
        &mut self,
        current: &Bounds,
        boundary: &C4Boundary,
        label: &Legacy,
        boundary_type: Option<&Legacy>,
        descr: Option<&Legacy>,
    ) -> Result<(), RenderError> {
        let conf = self.conf;
        let x = current.data.startx.unwrap_or(0.0);
        let y = current.data.starty.unwrap_or(0.0);
        let width = current.data.stopx.unwrap_or(0.0) - x;
        let height = current.data.stopy.unwrap_or(0.0) - y;
        let fill = non_empty(boundary.bg_color.as_ref()).unwrap_or("none");
        let stroke = non_empty(boundary.border_color.as_ref()).unwrap_or(DEFAULT_STROKE);
        let dashed = non_empty(boundary.node_type.as_ref()).is_none();
        let attributes = if dashed {
            " stroke-width=\"1\" stroke-dasharray=\"7.0,7.0\""
        } else {
            " stroke-width=\"1\""
        };
        let mut out = format!(
            "<g><rect x=\"{x}\" y=\"{y}\" fill=\"{fill}\" stroke=\"{stroke}\" width=\"{width}\" height=\"{height}\" rx=\"2.5\" ry=\"2.5\"{attributes}></rect>",
            x = number(x),
            y = number(y),
            fill = escape(fill),
            stroke = escape(stroke),
            width = number(width),
            height = number(height),
        );

        let weight = conf.boundary.weight.clone();
        let label_text = label.text.clone();
        self.record_legacy(&label_text, true, false);
        out.push_str(&legacy::by_tspan(
            &label_text,
            x,
            y + label.y,
            width,
            legacy_face(conf.boundary.size + 2.0, "bold"),
            Paint {
                fill: DEFAULT_STROKE,
                italic: false,
            },
        ));
        if let Some(text) = boundary_type {
            self.record_legacy(&text.text, false, false);
            out.push_str(&legacy::by_tspan(
                &text.text,
                x,
                y + text.y,
                width,
                legacy_face(conf.boundary.size, &weight),
                Paint {
                    fill: DEFAULT_STROKE,
                    italic: false,
                },
            ));
        }
        if let Some(text) = descr {
            self.record_legacy(&text.text, false, false);
            out.push_str(&legacy::by_tspan(
                &text.text,
                x,
                y + text.y,
                width,
                legacy_face(conf.boundary.size - 2.0, &weight),
                Paint {
                    fill: DEFAULT_STROKE,
                    italic: false,
                },
            ));
        }
        out.push_str("</g>");
        self.body.push_str(&out);

        // The boundary's box is what a relation that ends at it meets (`drawBoundary`, lines 256-265).
        self.boundaries.insert(
            boundary.alias.clone(),
            Place {
                centre: Point::new(x + width / 2.0, y + height / 2.0),
                width,
                height,
                intersect: Intersect::Rect,
            },
        );
        Ok(())
    }
}

/// Whether a boundary has a sprite, which reserves its image box (`currentBoundary.sprite`).
fn non_empty_sprite(boundary: &C4Boundary) -> bool {
    non_empty(boundary.sprite.as_ref()).is_some()
}

/// The renderer's error for a measurement error: the font's error, or the reference's failure to
/// measure a line of white space (`svg element not in render tree`).
pub fn text_error(error: MeasureError) -> RenderError {
    match error {
        MeasureError::Text(text) => RenderError::Text(text),
        MeasureError::NotInRenderTree => {
            RenderError::Layout("svg element not in render tree".to_owned())
        }
    }
}

/// The `<g>` of an element: positioned at its centre, with the node group and its body and label.
fn element_group(id: &str, element: &Built, centre: Point) -> String {
    let body = match &element.body {
        Body::Rounded {
            radius,
            x,
            y,
            width,
            height,
        } => format!(
            "<rect class=\"basic label-container\" style=\"{style}\" rx=\"{r}\" ry=\"{r}\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\"></rect>",
            style = element.body_style,
            r = number(*radius),
            x = number(*x),
            y = number(*y),
            width = number(*width),
            height = number(*height),
        ),
        Body::Person {
            width,
            body_top,
            body_height,
            body_radius,
            head_centre_y,
            head_radius,
        } => format!(
            "<g class=\"basic label-container\"><rect x=\"{x}\" y=\"{top}\" width=\"{width}\" height=\"{height}\" rx=\"{r}\" ry=\"{r}\" style=\"{style}\"></rect><circle cx=\"0\" cy=\"{cy}\" r=\"{hr}\" style=\"{style}\"></circle></g>",
            x = number(-width / 2.0),
            top = number(*body_top),
            width = number(*width),
            height = number(*body_height),
            r = number(*body_radius),
            style = element.body_style,
            cy = number(*head_centre_y),
            hr = number(*head_radius),
        ),
        Body::Polygon { points, transform } => {
            let style = if element.body_style.is_empty() {
                String::new()
            } else {
                format!(" style=\"{}\"", element.body_style)
            };
            format!(
                "<polygon points=\"{points}\" class=\"label-container\" transform=\"{transform}\"{style}></polygon>",
                points = escape(points),
                transform = escape(transform),
            )
        }
        Body::Path { data, transform } => format!(
            "<path d=\"{data}\" class=\"basic label-container outer-path\" style=\"{style}\" transform=\"{transform}\"></path>",
            data = escape(data),
            style = element.body_style,
            transform = escape(transform),
        ),
    };
    let label = label::group(&element.label, element.label_translate, &element.text_fill);
    format!(
        "<g transform=\"translate({cx}, {cy})\"><g class=\"node {classes}\" id=\"{id}-{alias}\">{body}{label}</g></g>",
        cx = number(centre.x),
        cy = number(centre.y),
        classes = element.classes,
        id = escape(id),
        alias = escape(&element.alias),
    )
}
