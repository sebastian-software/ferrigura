//! `@upsetjs/venn.js` 2.0.0 (MIT) ported from the JavaScript bundle: the part of the layout that
//! Mermaid's venn renderer reaches. This is the layout of Mermaid's `venn-beta` diagrams: the
//! circles of the chart and the text positions of `layout()`.
//!
//! **What is ported.** `venn` (`src/layout.js` 11-60): the initial layout, then the minimisation of
//! the squared overlap error by the downhill simplex. The initial layout is the greedy placement
//! ([`greedy_layout`]), or, from eight areas on, the constrained multidimensional scaling
//! ([`constrained_mds_layout`]) when it has the smaller loss by more than 1e-8. `addMissingAreas`
//! ([`add_missing_areas`]), `normalizeSolution` and `scaleSolution` ([`normalize_solution`],
//! [`scale_solution`]), and the text centres ([`compute_text_centres`]). The minimisers are those of
//! `fmin` as the bundle inlines them: bisection, the simplex, the Wolfe line search and conjugate
//! gradient (`fmin.rs`). The two calls Mermaid makes are [`chart`] and [`layout`].
//!
//! **Numbers.** `sin`, `cos`, `atan2` and `acos` are the defaults of `ferrigura_jsmath`, which give
//! Chromium's results bit for bit. Each expression keeps the operand order of the source, so the
//! output agrees with the library exactly, not within a tolerance. The JavaScript behaviours that
//! decide the order of the result are reproduced (`js.rs`): the order of `Object.keys` (a set id
//! that is an array index comes first, in numeric order), the stable sorts and their handling of
//! NaN, `Math.min` and `Math.max`, and the UTF-16 order of the ids.
//!
//! **Randomness.** The constrained layout draws `2n` numbers per restart (ten restarts), from the
//! random source the caller gives. Mermaid's page uses `Math.random`, which the geometry harness
//! seeds; the tests use a port of that generator with the same draw order (`tests/corpus.rs`).
//!
//! **Not ported.** The options the renderer does not pass: `distinct`, `scaleToFit`,
//! `symmetricalTextCentre`, `orientationOrder`, the logarithmic loss, a custom loss, layout or
//! initial layout, the `weight` of an area, and the history options. The `path` strings, `arcs` and
//! `distinctPath` of `layout()` (Mermaid reads none of them). The drawing (`VennDiagram`'s d3 code,
//! `wrapText`, the hand-drawn look). The `console.log` warnings. A set named `__proto__` is refused
//! ([`Error::ReservedName`]): assigning that key in a plain object changes its prototype, so the
//! library's output for it is not reproduced. Set ids that name other `Object.prototype` members
//! (`constructor`, `toString`, ...) are reproduced: the library's `in` tests see them as present.
//!
//! Errors ([`Error`]) are the cases where the library throws. Licences and sources are in
//! `NOTICE.md`.

#![forbid(unsafe_code)]

mod arrange;
mod diagram;
mod error;
mod fmin;
mod geometry;
mod js;
mod placement;
mod text;

use std::f64::consts::PI;

pub use error::Error;
pub use placement::add_missing_areas;

use js::JsObject;

/// A set (a single id) or an overlap of sets: the ids of the sets, and the size of the area. The
/// ids are as the diagram gives them, sorted; a single set has one id.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub sets: Vec<String>,
    pub size: f64,
}

/// A circle: its centre and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

/// A text centre. `disjoint` is set for an area that is not drawn anywhere: the library then gives
/// the point (0, -1000) with `disjoint: true`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Centre {
    pub x: f64,
    pub y: f64,
    pub disjoint: bool,
}

/// An object of the library: its entries (set id and circle) in the order `Object.keys` gives.
pub type Solution = Vec<(String, Circle)>;

/// A circle of an area's layout: the set it belongs to, and its position.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedCircle {
    pub set: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

/// The layout of one area of [`layout`]: its sets, its text centre, and its sets' circles in the
/// order of its sets.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaLayout {
    pub sets: Vec<String>,
    pub text: Centre,
    pub circles: Vec<PlacedCircle>,
}

/// The output of [`chart`]: the circles of the chart, and the text centre of each area, keyed by
/// its sets joined with commas, in the order `Object.keys` gives.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    pub circles: Solution,
    pub text_centres: Vec<(String, Centre)>,
}

fn to_object(solution: &[(String, Circle)]) -> JsObject<Circle> {
    let mut object = JsObject::new();
    for (set, circle) in solution {
        object.insert(set, *circle);
    }
    object
}

fn to_solution(object: &JsObject<Circle>) -> Solution {
    object
        .iter()
        .map(|(set, circle)| (set.to_owned(), *circle))
        .collect()
}

fn to_centres(object: &JsObject<Centre>) -> Vec<(String, Centre)> {
    object
        .iter()
        .map(|(key, centre)| (key.to_owned(), *centre))
        .collect()
}

/// `venn(areas)`: the circles of the diagram, before normalisation.
///
/// # Errors
///
/// See [`Error`].
pub fn venn(areas: &[Area], random: &mut impl FnMut() -> f64) -> Result<Solution, Error> {
    placement::venn(areas, random).map(|object| to_solution(&object))
}

/// `normalizeSolution(solution)`: the clusters oriented and arranged in a grid, with the
/// orientation of Mermaid (π/2).
///
/// # Errors
///
/// [`Error::NoAreas`] for an empty solution.
pub fn normalize_solution(solution: &[(String, Circle)]) -> Result<Solution, Error> {
    arrange::normalize(&to_object(solution), PI / 2.0).map(|object| to_solution(&object))
}

/// `scaleSolution(solution, width, height, padding)` with `scaleToFit` off: the solution scaled
/// into the box inside its padding, and centred.
pub fn scale_solution(
    solution: &[(String, Circle)],
    width: f64,
    height: f64,
    padding: f64,
) -> Solution {
    to_solution(&arrange::scale(
        &to_object(solution),
        width,
        height,
        padding,
    ))
}

/// `computeTextCentres(circles, areas)`: the text centre of each area.
///
/// # Errors
///
/// [`Error::UnknownSet`] when an area names a set without a circle.
pub fn compute_text_centres(
    circles: &[(String, Circle)],
    areas: &[Area],
) -> Result<Vec<(String, Centre)>, Error> {
    text::compute_text_centres(&to_object(circles), areas).map(|object| to_centres(&object))
}

/// `greedyLayout(areas)`: the greedy placement of the sets, before the minimisation. The areas are
/// used as given: [`add_missing_areas`] first gives the list the library places from.
///
/// # Errors
///
/// See [`Error`].
pub fn greedy_layout(areas: &[Area]) -> Result<Solution, Error> {
    placement::check_names(areas)?;
    placement::greedy(areas).map(|object| to_solution(&object))
}

/// `constrainedMDSLayout(areas)`: the constrained multidimensional scaling of the sets, the best of
/// ten restarts. The areas are used as given, as for [`greedy_layout`].
///
/// # Errors
///
/// See [`Error`].
pub fn constrained_mds_layout(
    areas: &[Area],
    random: &mut impl FnMut() -> f64,
) -> Result<Solution, Error> {
    placement::check_names(areas)?;
    placement::constrained_mds(areas, random).map(|object| to_solution(&object))
}

/// The chart of Mermaid's venn renderer (`VennDiagram` with the given width and height, and
/// padding 15): the circles, and the text centre of each area. Zero-size sets are left out with
/// the areas that name them.
///
/// # Errors
///
/// See [`Error`].
pub fn chart(
    areas: &[Area],
    width: f64,
    height: f64,
    padding: f64,
    random: &mut impl FnMut() -> f64,
) -> Result<Chart, Error> {
    diagram::chart(areas, width, height, padding, random).map(|layout| Chart {
        circles: to_solution(&layout.circles),
        text_centres: to_centres(&layout.text_centres),
    })
}

/// `layout(areas, { width, height, padding })`: each area's text centre and the circles of its
/// sets, for every area of the input, zero-size sets included.
///
/// # Errors
///
/// See [`Error`].
pub fn layout(
    areas: &[Area],
    width: f64,
    height: f64,
    padding: f64,
    random: &mut impl FnMut() -> f64,
) -> Result<Vec<AreaLayout>, Error> {
    diagram::layout(areas, width, height, padding, random).map(|areas| {
        areas
            .into_iter()
            .map(|(sets, text, circles)| AreaLayout {
                sets,
                text,
                circles: circles
                    .into_iter()
                    .map(|(set, circle)| PlacedCircle {
                        set,
                        x: circle.x,
                        y: circle.y,
                        radius: circle.radius,
                    })
                    .collect(),
            })
            .collect()
    })
}
