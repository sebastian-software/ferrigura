//! The two calls of Mermaid's venn renderer: the chart (`VennDiagram`, `src/diagram.js` 78-97) that
//! draws the circles, and `layout()` (`src/diagram.js` 717-783) that gives the text positions.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::Area;
use crate::Centre;
use crate::Circle;
use crate::Error;
use crate::arrange::{normalize, scale};
use crate::js::JsObject;
use crate::placement::{check, venn};
use crate::text::compute_text_centres;

/// The output of the chart: the circles in `Object.keys` order, and the text centre of each area
/// (keyed by its sets joined with commas).
pub(crate) struct ChartLayout {
    pub circles: JsObject<Circle>,
    pub text_centres: JsObject<Centre>,
}

/// `VennDiagram`'s layout: the zero-size single sets are removed with every area that names one,
/// then `venn`, `normalizeSolution` (orientation π/2), `scaleSolution` (padding `padding`, no fit to
/// a diameter) and `computeTextCentres`. With no areas left, nothing is laid out.
pub(crate) fn chart(
    areas: &[Area],
    width: f64,
    height: f64,
    padding: f64,
    random: &mut impl FnMut() -> f64,
) -> Result<ChartLayout, Error> {
    let zero: std::collections::HashSet<&str> = areas
        .iter()
        .filter(|area| area.sets.len() == 1 && area.size == 0.0)
        .map(|area| area.sets[0].as_str())
        .collect();
    let data: Vec<Area> = areas
        .iter()
        .filter(|area| !area.sets.iter().any(|set| zero.contains(set.as_str())))
        .cloned()
        .collect();

    if data.is_empty() {
        return Ok(ChartLayout {
            circles: JsObject::new(),
            text_centres: JsObject::new(),
        });
    }
    check(&data)?;

    let solution = venn(&data, random)?;
    let normalized = normalize(&solution, PI / 2.0)?;
    let circles = scale(&normalized, width, height, padding);
    let text_centres = compute_text_centres(&circles, &data)?;
    Ok(ChartLayout {
        circles,
        text_centres,
    })
}

/// `layout(data, { width, height, padding })`: `venn`, `normalizeSolution` and `scaleSolution`
/// (with `scaleToFit` off), then the text centres. Every area is kept, zero-size sets included.
pub(crate) fn layout(
    areas: &[Area],
    width: f64,
    height: f64,
    padding: f64,
    random: &mut impl FnMut() -> f64,
) -> Result<LayoutAreas, Error> {
    let solution = venn(areas, random)?;
    let normalized = normalize(&solution, PI / 2.0)?;
    let circles = scale(&normalized, width, height, padding);
    let text_centres = compute_text_centres(&circles, areas)?;

    let lookup: HashMap<&str, Circle> =
        circles.iter().map(|(set, circle)| (set, *circle)).collect();
    let mut result = Vec::with_capacity(areas.len());
    for area in areas {
        let mut placed = Vec::with_capacity(area.sets.len());
        for set in &area.sets {
            let circle = lookup
                .get(set.as_str())
                .ok_or_else(|| Error::UnknownSet(set.clone()))?;
            placed.push((set.clone(), *circle));
        }
        let text = *text_centres
            .get(&area.sets.join(","))
            .ok_or_else(|| Error::UnknownSet(area.sets.join(",")))?;
        result.push((area.sets.clone(), text, placed));
    }
    Ok(result)
}

/// Per area, in input order: its sets, its text centre, and the circles of its sets in the order
/// of its sets.
pub(crate) type LayoutAreas = Vec<(Vec<String>, Centre, Vec<(String, Circle)>)>;
