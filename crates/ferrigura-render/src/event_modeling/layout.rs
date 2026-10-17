//! The layout of an event modeling diagram: the texts of the boxes, the boxes, the lanes and the
//! relations, as the reference's DB and renderer compute them (`db.ts` 87-642, `renderer.ts`
//! 52-97). The research is in `docs-port/eventmodeling/NOTES.md`: sections 3.4 to 3.6 for the
//! placement, 6.4.1 for the text of a box, 6.5 for the relations and 6.7 for the measurement.
//!
//! The model gives the frames, the lanes and the relations, and the data text of each frame already
//! cut and sanitised (`EventModelingDiagram`). The measurement is `calculateTextDimensions` of the
//! literal content (`TextMemo`): the markup, the `&nbsp;` joins and the placeholders count as
//! characters, as the reference measures them.

use ferrigura_model::event_modeling::{DataEntity, EventModelingDiagram, Frame};
use ferrigura_syntax::sanitize::sanitize_text;

use crate::RenderError;
use crate::common::{MeasureError, TextMemo, TextStyle, WrapStyle};

/// The wrap width of a name and of data: `textMaxWidth`, 450 less 2 × 10 (`db.ts` 153-175).
pub const TEXT_MAX_WIDTH: f64 = 430.0;

/// The font of the measurement: `wrapLabelConfig` and the `TextDimensionConfig` of `db.ts`
/// 339-346 and 383-387. It is set in code, not by the configuration, and is always this one.
pub const MEASURE_FONT_SIZE: f64 = 16.0;

/// The bold weight of the measurement (`fontWeight: 700`).
pub const MEASURE_FONT_WEIGHT: f64 = 700.0;

/// The family list of the measurement, as `db.ts` 342 and 383 write it (the reference names it
/// itself, so the harness maps it onto the bundled face).
pub const MEASURE_FONT_FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";

/// The text of a box as `calculateTextProps` builds it (`db.ts` 330-397).
#[derive(Debug, Clone, PartialEq)]
pub struct BoxText {
    /// The name, wrapped at [`TEXT_MAX_WIDTH`] and joined with `<br/>`, as the reference's
    /// `wrappedName` has it. It keeps the entity placeholders (`ﬂ°…¶ß`).
    pub name: String,
    /// The data the box shows: the HTML of its `code` element, or `None` when it shows none.
    pub data: Option<String>,
    /// The width of the text, as `decidePositionFrame` takes it: the measured width, divided by
    /// three when the box shows data (`db.ts` 385-388).
    pub width: f64,
    /// The height of the text: the measured height (`db.ts` 390-394).
    pub height: f64,
}

/// A box of the diagram: its position and size, and its text. The boxes are in the order of the
/// frames, so a box's index is its frame's.
#[derive(Debug, Clone, PartialEq)]
pub struct Boxed {
    /// The index of the frame's lane in [`Layout::lanes`].
    pub lane: usize,
    /// The left edge (`calculateX`, `db.ts` 419-437).
    pub x: f64,
    /// The width of the box (`evolveFramePositioned`, `db.ts` 473-486).
    pub width: f64,
    /// The height of the box.
    pub height: f64,
    /// The entity type as written, which picks the colours of the box (`db.ts` 290-328).
    pub entity_type: String,
    /// The text of the box.
    pub text: BoxText,
}

/// A lane: its index and label, and its place and size once every frame is placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    /// The index of the lane, which orders the lanes.
    pub index: usize,
    /// The label drawn for the lane.
    pub label: String,
    /// The top edge: 0 for the first lane, then the previous lane's bottom plus a gap of 10.
    pub y: f64,
    /// The height: the tallest box of the lane, at least 70, plus 30 (`db.ts` 489-495).
    pub height: f64,
}

/// A relation between two boxes, with the points it joins (`renderer.ts` 52-97).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Relation {
    /// The start of the path.
    pub from: (f64, f64),
    /// The end of the path, with the arrowhead.
    pub to: (f64, f64),
}

/// The placed diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The lanes, in index order.
    pub lanes: Vec<Lane>,
    /// The boxes, in the order of the text.
    pub boxes: Vec<Boxed>,
    /// The relations, in the order the DB creates them.
    pub relations: Vec<Relation>,
    /// The DB's `maxR` (`db.ts` 490): the largest of the lanes' right edges, as they stand before
    /// the last frame, and the last frame's `r`. The lanes are drawn `maxR + 15` wide. It is not the
    /// largest `r` of every box: a box placed left of an earlier, wider box can stick out of the
    /// lanes, and the reference draws it so.
    pub max_r: f64,
}

/// `Math.max(lower, Math.min(upper, value))`, the clamp of `db.ts` 473-486.
fn clamp(value: f64, lower: f64, upper: f64) -> f64 {
    lower.max(upper.min(value))
}

/// The measurement's errors as the renderer's: a line of white space is not in the render tree
/// (the reference throws there too).
pub fn measured(error: MeasureError) -> RenderError {
    match error {
        MeasureError::NotInRenderTree => {
            RenderError::Layout("svg element not in render tree".to_owned())
        }
        MeasureError::Text(error) => RenderError::Text(error),
    }
}

/// The measurement's style for the name and the data: 16 px bold in the family of the measurement.
#[must_use]
pub fn measure_style() -> TextStyle {
    TextStyle {
        font_size: MEASURE_FONT_SIZE,
        font_weight: MEASURE_FONT_WEIGHT,
        font_family: MEASURE_FONT_FAMILY.to_owned(),
    }
}

/// The style of `wrapLabel` for the name and the data: the measurement's style, joined with `<br/>`.
#[must_use]
pub fn wrap_style() -> WrapStyle {
    WrapStyle {
        text: measure_style(),
        join_with: "<br/>".to_owned(),
    }
}

/// `calculateTextProps` (`db.ts` 330-397): the text of a box and its measured size.
///
/// # Errors
///
/// Returns an error if the measurement fails (a bundled font cannot be read).
pub fn box_text(
    frame: &Frame,
    data_entities: &[DataEntity],
    memo: &mut TextMemo,
) -> Result<BoxText, RenderError> {
    let style = wrap_style();
    let name_text = sanitize_text(&frame.entity_name);
    let name = memo
        .wrap_label(&name_text, TEXT_MAX_WIDTH, &style)
        .map_err(measured)?;
    let mut content = format!("<b>{name}</b>");

    // The block the frame names is the data when it exists; its trailing `<br/>` is added after
    // the blanks are joined (`db.ts` 358-378).
    let from_block = frame
        .data_reference
        .as_deref()
        .is_some_and(|reference| data_entities.iter().any(|entity| entity.name == reference));
    let data = match &frame.data {
        None => None,
        Some(text) => {
            let mut html = memo
                .wrap_label(text, TEXT_MAX_WIDTH, &style)
                .map_err(measured)?
                .replace(' ', "&nbsp;");
            if from_block {
                html.push_str("<br/>");
            }
            Some(html)
        }
    };
    if let Some(html) = &data {
        content.push_str(&format!(
            "<br/><br/><code style=\"text-align: left; display: block;max-width:{}px\">{html}</code>",
            crate::svg::number(TEXT_MAX_WIDTH)
        ));
    }

    let dimensions = memo
        .dimensions(&content, &measure_style())
        .map_err(measured)?;
    // The width is divided by three without rounding, as `db.ts` 385-388 does.
    let width = if data.is_some() {
        dimensions.width / 3.0
    } else {
        dimensions.width
    };
    Ok(BoxText {
        name,
        data,
        width,
        height: dimensions.height,
    })
}

/// The place of a lane in the list, by its index; the list is in index order.
fn lane_position(lanes: &[Lane], index: usize) -> Result<usize, RenderError> {
    lanes
        .binary_search_by_key(&index, |lane| lane.index)
        .map_err(|_| RenderError::Layout(format!("the lane {index} of a frame is missing")))
}

/// `calculateX` (`db.ts` 419-437): the first box is at 250; a box in the lane of the previous box
/// follows that lane's last box; any other box starts 80 px left of the last box's right edge.
fn calculate_x(
    lane_right: f64,
    lane: usize,
    previous_lane: Option<usize>,
    last_box_right: Option<f64>,
) -> f64 {
    const CONTENT_START_X: f64 = 250.0;
    const BOX_PADDING: f64 = 10.0;
    const BOX_OVERLAP: f64 = 90.0;
    let Some(previous) = previous_lane else {
        return CONTENT_START_X;
    };
    if previous == lane && lane_right != 0.0 {
        return lane_right + BOX_PADDING;
    }
    match last_box_right {
        Some(right) => right - BOX_OVERLAP + BOX_PADDING,
        None => CONTENT_START_X,
    }
}

/// The lanes and boxes of the diagram, placed as `getState` places them (`db.ts` 87-137).
///
/// # Errors
///
/// Returns an error if a measurement fails, or a frame names a lane the diagram does not have.
pub fn lay_out(diagram: &EventModelingDiagram, memo: &mut TextMemo) -> Result<Layout, RenderError> {
    const SWIMLANE_MIN_HEIGHT: f64 = 70.0;
    const SWIMLANE_PADDING: f64 = 15.0;
    const SWIMLANE_GAP: f64 = 10.0;
    const BOX_PADDING: f64 = 10.0;
    const BOX_TEXT_PADDING: f64 = 10.0;
    const BOX_MIN_WIDTH: f64 = 80.0;
    const BOX_MAX_WIDTH: f64 = 450.0;
    const BOX_MIN_HEIGHT: f64 = 80.0;
    const BOX_MAX_HEIGHT: f64 = 750.0;

    // Per lane: the right edge of its last box (`r`, 0 until it has a box), and the tallest box
    // (`maxHeight`, from 70). The label and index come from the diagram.
    let mut lanes: Vec<Lane> = diagram
        .lanes
        .iter()
        .map(|lane| Lane {
            index: lane.index,
            label: lane.label.clone(),
            y: 0.0,
            height: SWIMLANE_MIN_HEIGHT,
        })
        .collect();
    let mut lane_right = vec![0.0_f64; lanes.len()];
    let mut lane_max_height = vec![SWIMLANE_MIN_HEIGHT; lanes.len()];

    let mut boxes = Vec::with_capacity(diagram.frames.len());
    let mut previous_lane: Option<usize> = None;
    let mut last_box_right: Option<f64> = None;
    let mut max_r = 0.0_f64;

    for frame in &diagram.frames {
        let text = box_text(frame, &diagram.data_entities, memo)?;
        let lane = lane_position(&lanes, frame.lane)?;

        // `decidePositionFrame` adds the box's text padding, then `evolveFramePositioned` clamps
        // the size and adds the padding of the box on each side.
        let width = clamp(
            text.width + 2.0 * BOX_TEXT_PADDING,
            BOX_MIN_WIDTH,
            BOX_MAX_WIDTH,
        ) + 2.0 * BOX_PADDING;
        let height = clamp(
            text.height + 2.0 * BOX_TEXT_PADDING,
            BOX_MIN_HEIGHT,
            BOX_MAX_HEIGHT,
        ) + 2.0 * BOX_PADDING;

        let x = calculate_x(lane_right[lane], lane, previous_lane, last_box_right);
        let right = x + width + BOX_PADDING;
        // `calculateMaxRight` reads the lanes before this frame moves its lane (`db.ts` 459-469).
        max_r = lane_right.iter().copied().fold(right, f64::max);

        lane_right[lane] = x + width;
        lane_max_height[lane] = lane_max_height[lane].max(height);

        boxes.push(Boxed {
            lane,
            x,
            width,
            height,
            entity_type: frame.model_entity_type.clone(),
            text,
        });
        previous_lane = Some(lane);
        last_box_right = Some(right);
    }

    // The heights are final now, so the lanes' heights and tops are too: a lane is its tallest box
    // plus 30, and each lane starts 10 below the one above it (`db.ts` 489-495, 523-536).
    for (position, lane) in lanes.iter_mut().enumerate() {
        lane.height = lane_max_height[position].max(SWIMLANE_MIN_HEIGHT) + 2.0 * SWIMLANE_PADDING;
    }
    for position in 1..lanes.len() {
        let above = &lanes[position - 1];
        lanes[position].y = above.y + above.height + SWIMLANE_GAP;
    }

    let relations = diagram
        .relations
        .iter()
        .map(|relation| {
            let source = &boxes[relation.source_frame];
            let target = &boxes[relation.target_frame];
            let source_top = lanes[source.lane].y + SWIMLANE_PADDING;
            let target_top = lanes[target.lane].y + SWIMLANE_PADDING;
            let source_x = source.x + source.width * 2.0 / 3.0;
            let target_x = target.x + target.width / 3.0;
            // `dirUpwards` is strict: a relation inside one lane goes down (`renderer.ts` 52-84).
            let (source_y, target_y) = if source_top > target_top {
                (source_top, target_top + target.height)
            } else {
                (source_top + source.height, target_top)
            };
            Relation {
                from: (source_x, source_y),
                to: (target_x, target_y),
            }
        })
        .collect();

    Ok(Layout {
        lanes,
        boxes,
        relations,
        max_r,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_box_starts_at_the_content_start_and_a_box_in_its_lane_follows_it() {
        // The first frame has no previous lane: 250. The second, in the same lane, follows the
        // lane's right edge plus 10.
        assert_eq!(calculate_x(0.0, 0, None, None), 250.0);
        assert_eq!(calculate_x(400.0, 0, Some(0), Some(410.0)), 410.0);
        // Another lane: 80 px left of the last box's right edge.
        assert_eq!(calculate_x(0.0, 1, Some(0), Some(410.0)), 330.0);
    }

    #[test]
    fn the_clamp_is_the_one_of_the_size_rules() {
        assert_eq!(clamp(10.0, 80.0, 450.0), 80.0);
        assert_eq!(clamp(500.0, 80.0, 450.0), 450.0);
        assert_eq!(clamp(120.0, 80.0, 450.0), 120.0);
    }
}
