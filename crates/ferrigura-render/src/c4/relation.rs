//! The relations of a C4 diagram: `drawRels` (`c4Renderer.ts:387-433`) and `svgDraw.ts:26-126`. A
//! relation's label and techn are measured, its ends are where the line from one element's centre to the
//! other meets their outlines, and the first relation is a straight line while the others are curves.

use ferrigura_model::c4::C4Relation;

use super::config::Conf;
use super::draw::{DEFAULT_STROKE, Draw, FAMILY, Place, style_of, text_error};
use super::legacy::{self, Face, Legacy, Paint};
use super::shape::plain;
use crate::RenderError;
use crate::common::is_js_whitespace;
use crate::svg::{escape, number};
use crate::unified::geom::Point;

/// The `parseInt` of a relation's offset, as `svgDraw.ts:35-36` reads it: a positional offset that is
/// `NaN` is falsy, so it draws at zero, and a named offset is parsed from its text (`5px` is 5, `abc` is
/// `NaN`, which the reference writes as `NaN`).
fn offset(value: Option<&str>) -> f64 {
    match value {
        None | Some("" | "NaN") => 0.0,
        Some(text) => parse_int(text),
    }
}

/// JavaScript's `parseInt(text, 10)`: the leading blanks, a sign and the digits, `NaN` when there are no
/// digits.
fn parse_int(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(is_js_whitespace);
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1.0, rest),
        None => (1.0, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    sign * digits.parse::<f64>().unwrap_or(f64::NAN)
}

/// A relation after its measurement, with the points where it meets its two elements.
struct Measured {
    label: Legacy,
    techn: Option<Legacy>,
    start: Point,
    end: Point,
}

impl Draw<'_> {
    /// The placed element or boundary with `alias`: a shape first, then a boundary (`getC4Shape`).
    fn placed(&self, alias: &str) -> Result<&Place, RenderError> {
        self.shapes
            .get(alias)
            .or_else(|| self.boundaries.get(alias))
            .ok_or_else(|| {
                RenderError::Layout(format!(
                    "C4 relation {alias} references an unknown shape or boundary"
                ))
            })
    }

    /// `getIntersectPoints` (`c4Renderer.ts:375-385`): the start is where the line from the source's
    /// centre to the target's meets the source's outline, and the end is the mirror of that.
    fn ends(&self, from: &str, to: &str) -> Result<(Point, Point), RenderError> {
        let source = self.placed(from)?;
        let target = self.placed(to)?;
        let start = source
            .intersect
            .intersect(source.box_for_intersect(), target.centre);
        let end = target
            .intersect
            .intersect(target.box_for_intersect(), source.centre);
        Ok((start, end))
    }

    /// `drawRels`: measures every relation, then draws them in one group, in the order given.
    ///
    /// # Errors
    ///
    /// Returns an error for a relation whose ends are not drawn, or a label that is not drawn.
    pub fn draw_rels(&mut self) -> Result<String, RenderError> {
        let model = self.model;
        let conf = self.conf;
        let dynamic = model.c4_type == "C4Dynamic";
        let message = style_of(
            &conf.message,
            conf.message.size,
            conf.message.weight_number(),
        );

        let mut measured = Vec::with_capacity(model.rels.len());
        for (index, rel) in model.rels.iter().enumerate() {
            // A named label of a dynamic diagram is numbered as the object's text (probed:
            // `1: [object Object]`); elsewhere the reference fails on it (`plain`).
            let text = match &rel.label {
                Some(ferrigura_model::c4::C4Text::Named(_)) if dynamic => {
                    "[object Object]".to_owned()
                }
                other => plain(other)?,
            };
            let text = if dynamic {
                format!("{}: {text}", index + 1)
            } else {
                text
            };
            let wrap = rel.wrap && conf.wrap;
            let limit = self.memo.width(&text, &message).map_err(text_error)?;
            let label = legacy::measure(&mut self.memo, &text, wrap, &message, limit)
                .map_err(text_error)?;
            let techn = match rel.techn.as_deref().filter(|techn| !techn.is_empty()) {
                Some(techn) => {
                    let limit = self.memo.width(techn, &message).map_err(text_error)?;
                    Some(
                        legacy::measure(&mut self.memo, techn, wrap, &message, limit)
                            .map_err(text_error)?,
                    )
                }
                None => None,
            };
            let (start, end) = self.ends(&rel.from, &rel.to)?;
            measured.push(Measured {
                label,
                techn,
                start,
                end,
            });
        }

        let mut out = String::from("<g>");
        for (index, (rel, measured)) in model.rels.iter().zip(measured).enumerate() {
            out.push_str(&self.relation(index == 0, rel, measured));
        }
        out.push_str("</g>");
        Ok(out)
    }

    /// One relation's line or curve, its label and its techn. The first relation is a `line`, the
    /// others a `path` (`svgDraw.ts:39-86`).
    fn relation(&mut self, first: bool, rel: &C4Relation, measured: Measured) -> String {
        let conf: &Conf = self.conf;
        let id = escape(self.id);
        let text_colour = non_empty(rel.text_color.as_deref()).unwrap_or(DEFAULT_STROKE);
        let stroke = non_empty(rel.line_color.as_deref()).unwrap_or(DEFAULT_STROKE);
        let offset_x = offset(rel.offset_x.as_deref());
        let offset_y = offset(rel.offset_y.as_deref());
        let (start, end) = (measured.start, measured.end);

        let marker_end = if rel.relation_type == "rel_b" {
            String::new()
        } else {
            format!(" marker-end=\"url(#{id}-arrowhead)\"")
        };
        let marker_start = if rel.relation_type == "birel" || rel.relation_type == "rel_b" {
            format!(" marker-start=\"url(#{id}-arrowend)\"")
        } else {
            String::new()
        };
        let mut out = if first {
            format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"1\" stroke=\"{}\"{marker_end}{marker_start} style=\"fill: none;\"></line>",
                number(start.x),
                number(start.y),
                number(end.x),
                number(end.y),
                escape(stroke),
            )
        } else {
            let control_x = start.x + (end.x - start.x) / 2.0 - (end.x - start.x) / 4.0;
            let control_y = start.y + (end.y - start.y) / 2.0;
            format!(
                "<path fill=\"none\" stroke-width=\"1\" stroke=\"{}\" d=\"M{},{} Q{},{} {},{}\"{marker_end}{marker_start}></path>",
                escape(stroke),
                number(start.x),
                number(start.y),
                number(control_x),
                number(control_y),
                number(end.x),
                number(end.y),
            )
        };

        // The labels sit at the middle of the two ends, offset by the relation's offsets.
        let mid_x = start.x.min(end.x) + (end.x - start.x).abs() / 2.0 + offset_x;
        let mid_y = start.y.min(end.y) + (end.y - start.y).abs() / 2.0 + offset_y;
        let face = Face {
            size: conf.message.size,
            weight: conf.message.weight.as_str(),
            family: FAMILY,
        };
        let label_width = measured.label.width;
        self.record_legacy(&measured.label.text, false, false);
        out.push_str(&legacy::by_tspan(
            &measured.label.text,
            mid_x,
            mid_y,
            label_width,
            face,
            Paint {
                fill: text_colour,
                italic: false,
            },
        ));
        if let Some(techn) = &measured.techn {
            let text = format!("[{}]", techn.text);
            self.record_legacy(&text, false, true);
            let y = start.y.min(end.y)
                + (end.y - start.y).abs() / 2.0
                + conf.message.size
                + 5.0
                + offset_y;
            out.push_str(&legacy::by_tspan(
                &text,
                mid_x,
                y,
                label_width.max(techn.width),
                face,
                Paint {
                    fill: text_colour,
                    italic: true,
                },
            ));
        }
        out
    }
}

/// The colour of an option, when it is not empty (`rel.textColor ? … : '#444444'`).
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}
