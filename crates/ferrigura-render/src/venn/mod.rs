//! The venn diagram (`venn-beta`): a port of `packages/mermaid/src/diagrams/venn/vennRenderer.ts`
//! (`draw`) and `styles.ts`. The circles and the intersections are drawn from the layout of
//! `ferrigura-venn` (`@upsetjs/venn.js` 2.0.0), the text of the sets and the text nodes are placed
//! in the reference's grid, and the research with the reference's file and line for each rule is in
//! `docs-port/venn/NOTES.md` (section 6). The decisions are in `docs-port/venn/PLAN.md`.
//!
//! Two departures from the reference's markup, both on the text, as the other SVG-text renderers
//! make them (`docs-port/decisions/0004`): a text node is an SVG `text` in the lines the browser
//! breaks its HTML into, where the reference has a `foreignObject` holding an `xhtml:span`; and a
//! set label is the one `tspan` the reference's `wrapText` leaves. The look `handDrawn` draws the
//! classic geometry, as ishikawa does, since roughjs is not ported.

mod colour;
mod data;
mod random;
mod style;
mod text;

use std::collections::{HashMap, HashSet};

use ferrigura_model::config::VennDiagramConfig;
use ferrigura_model::venn::{StyleEntry, TextNode, VennDiagram};
use ferrigura_text::{
    EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face, text_box,
};
use ferrigura_venn::{AreaLayout, Centre, Circle, IntersectionPath};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::d3::js_number;
use crate::common::style::base_style_sheet;
use crate::common::text::collapse_white_space;
use crate::common::viewbox::configure_svg_size;
use crate::context::Context;
use crate::journey::text::Line;
use crate::svg::number;
use colour::Schemes;
use data::{Datum, datums, ensure_pairwise_subsets, js_max, js_min, js_min_all, stable_key};
use style::Declarations;
use text::{set_label_text, text_node_lines};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The seed of the venn layout's random numbers when the render gives none. The reference's are
/// random for each render (`Math.random`); the port's are the harness's generator with this seed, so
/// that a render is the same each time.
pub const DEFAULT_SEED: u32 = 1;

/// The width the sizes of the diagram are scaled from (`vennRenderer.ts` 66): `scale` is the width
/// over this, so 0.5 at the default width of 800.
const REFERENCE_WIDTH: f64 = 1600.0;

/// The padding of the chart, which the reference's `VennDiagram` keeps at its default of 15
/// (`vennRenderer.ts` 91-95 pass only the size).
const CHART_PADDING: f64 = 15.0;

/// The palette slots the `venn-set-N` classes count (`i % 8`, `vennRenderer.ts` 130).
const PALETTE_SLOTS: usize = 8;

/// What the venn renderer reads besides its diagram: its configuration section, and the seed of the
/// random numbers the layout draws when it has eight areas or more. The reference draws them from
/// `Math.random`; the port draws the harness's generator from this seed, so that the draws match
/// the reference's under the harness (`random`).
#[derive(Debug, Clone, Copy)]
pub struct VennSection<'a> {
    /// The venn section of the configuration, in the diagram's scope.
    pub config: &'a VennDiagramConfig,
    /// The seed of the generator.
    pub seed: u32,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if the layout cannot place the sets (the reference throws on the same input,
/// for instance an empty diagram or a negative size), if a colour of the theme or of a style is not
/// a colour the colour functions read, or if a bundled font cannot be read or subset.
pub fn render(input: &Context<'_, VennDiagram, VennSection<'_>>) -> Result<String, RenderError> {
    let config = input.section.config;
    let diagram = input.diagram;
    let id = input.id;
    let width = config.width;
    let height = config.height;
    let scale = width / REFERENCE_WIDTH;
    let title_height = if diagram.title.is_empty() {
        0.0
    } else {
        48.0 * scale
    };
    let chart_height = height - title_height;
    let schemes = Schemes::new(input.theme)?;
    let styles = StyleLookup::new(&diagram.styles);

    // The layout: the chart's circles and centres, and the layout's areas for the text nodes. The
    // chart is laid out first and draws first, as the reference's calls do (`vennRenderer.ts` 95, 102).
    let render_sets = ensure_pairwise_subsets(&datums(&diagram.subsets));
    let areas: Vec<ferrigura_venn::Area> = render_sets
        .iter()
        .map(|datum| ferrigura_venn::Area {
            sets: datum.sets.clone(),
            size: datum.size,
        })
        .collect();
    let mut random = random::seeded(input.section.seed);
    let chart = ferrigura_venn::chart(&areas, width, chart_height, CHART_PADDING, &mut random)
        .map_err(layout_error)?;
    let layout = ferrigura_venn::layout(&areas, width, chart_height, config.padding, &mut random)
        .map_err(layout_error)?;

    let circles: HashMap<&str, Circle> = chart
        .circles
        .iter()
        .map(|(set, circle)| (set.as_str(), *circle))
        .collect();
    let centres: HashMap<&str, Centre> = chart
        .text_centres
        .iter()
        .map(|(key, centre)| (key.as_str(), *centre))
        .collect();
    // The layout's area of a key, and the datum it belongs to: the last one of each key wins, as
    // the reference's `layoutByKey` map does (`vennRenderer.ts` 109-113).
    let mut layout_by_key: HashMap<String, &AreaLayout> = HashMap::new();
    for area in &layout {
        layout_by_key.insert(stable_key(&data::sorted(&area.sets)), area);
    }
    let mut labelled_by_key: HashMap<String, bool> = HashMap::new();
    for datum in &render_sets {
        labelled_by_key.insert(datum.key(), !datum.label.is_empty());
    }

    // The areas the chart draws: the zero-size single sets are left out with every area that names
    // one (`VennDiagram`'s chart, `src/diagram.js` 78-84).
    let zero: HashSet<&str> = render_sets
        .iter()
        .filter(|datum| datum.sets.len() == 1 && datum.size == 0.0)
        .map(|datum| datum.sets[0].as_str())
        .collect();
    let drawn: Vec<&Datum> = render_sets
        .iter()
        .filter(|datum| !datum.sets.iter().any(|set| zero.contains(set.as_str())))
        .collect();

    // The labels of the chart: the last labelled area of each sets key wins (`src/diagram.js` 91-95).
    let mut labels: HashMap<String, &str> = HashMap::new();
    for datum in &drawn {
        if !datum.label.is_empty() {
            labels.insert(datum.sets.join(","), datum.label.as_str());
        }
    }

    let mut texts: Vec<String> = Vec::new();
    let mut vars = Vars::default();
    let mut drawing = Element::new("g").with(
        "transform",
        format!("translate(0, {})", js_number(title_height)),
    );
    let mut circle_index = 0;
    for datum in &drawn {
        let key = datum.key();
        let custom = styles.of(&key);
        let is_circle = datum.sets.len() == 1;
        let class = if is_circle {
            format!(
                "venn-area venn-circle venn-set-{}",
                circle_index % PALETTE_SLOTS
            )
        } else {
            "venn-area venn-intersection".to_owned()
        };
        let mut group = Element::new("g")
            .with("class", class)
            .with("data-venn-sets", datum.sets.join("_"));

        let area_circles: Vec<Circle> = datum
            .sets
            .iter()
            .map(|set| {
                circles
                    .get(set.as_str())
                    .copied()
                    .ok_or_else(|| RenderError::Layout(format!("the set {set} has no circle")))
            })
            .collect::<Result<_, _>>()?;
        let path = path_data(&ferrigura_venn::intersection_path(&area_circles));

        let label_text = label_of(datum, &labels).unwrap_or_default();
        let shown = set_label_text(&label_text);
        let centre = centres
            .get(datum.sets.join(",").as_str())
            .copied()
            .ok_or_else(|| RenderError::Layout("an area has no text centre".to_owned()))?;
        let label_x = js_number(centre.x.floor());
        let label_y = js_number(centre.y.floor());
        texts.push(shown.clone());
        let font_size = format!("{}px", js_number(48.0 * scale));

        if is_circle {
            let index = circle_index;
            circle_index += 1;
            let custom_fill = custom.fill.filter(|fill| !fill.is_empty());
            let custom_stroke = custom.stroke.filter(|stroke| !stroke.is_empty());
            let custom_label = custom.color.filter(|colour| !colour.is_empty());
            let base_light = schemes.light.base(index, custom_fill.as_deref());
            let base_dark = schemes
                .dark
                .as_ref()
                .map(|scheme| scheme.base(index, custom_fill.as_deref()));
            // The fill: the style's colour, or the palette's of the scheme (a custom property).
            let (fill_light, fill_css) = match &custom_fill {
                Some(fill) => (Some(fill.clone()), Some(fill.clone())),
                None => {
                    let dark = base_dark.clone().flatten();
                    let light = base_light.clone();
                    if light.is_some() || dark.is_some() {
                        vars.set(format!("--vennFill-{index}"), light.clone(), dark);
                        (light, Some(format!("var(--vennFill-{index})")))
                    } else {
                        (None, None)
                    }
                }
            };
            let stroke_light = custom_stroke.clone().or_else(|| fill_light.clone());
            let stroke_css = custom_stroke.clone().or_else(|| fill_css.clone());
            let fill_opacity = custom
                .fill_opacity
                .clone()
                .unwrap_or_else(|| "0.1".to_owned());
            let stroke_width = custom
                .stroke_width
                .clone()
                .filter(|width| !width.is_empty())
                .unwrap_or_else(|| js_number(5.0 * scale));

            // The label: the style's colour, or the base colour lightened or darkened per scheme.
            let (label_light, label_css) = match custom_label {
                Some(colour) => (Some(colour.clone()), Some(colour)),
                None => {
                    let light = schemes.light.label(base_light.as_deref())?;
                    let dark = match &schemes.dark {
                        Some(scheme) => Some(scheme.label(base_dark.clone().flatten().as_deref())?),
                        None => None,
                    };
                    vars.set(format!("--vennLabel-{index}"), Some(light.clone()), dark);
                    (Some(light), Some(format!("var(--vennLabel-{index})")))
                }
            };

            let path_style = style_list(&[
                ("fill-opacity", Some(fill_opacity.clone())),
                ("fill", fill_css.clone()),
                ("stroke", stroke_css.clone()),
                ("stroke-width", Some(stroke_width.clone())),
                ("stroke-opacity", Some("0.95".to_owned())),
            ]);
            // The presentation attributes are the light scheme's colours, which the style overrides
            // when the dark scheme is set (`docs-port/decisions/0007`). A colour that is not set is
            // not written, as the reference's `undefined` removes it.
            let mut circle_path = Element::new("path")
                .with("d", path.clone())
                .with("fill-opacity", fill_opacity)
                .with("stroke-width", stroke_width)
                .with("stroke-opacity", "0.95")
                .with("style", path_style);
            if let Some(fill) = fill_light {
                circle_path.set("fill", fill);
            }
            if let Some(stroke) = stroke_light {
                circle_path.set("stroke", stroke);
            }
            group.push_child(circle_path);
            group.push_child(label_element(
                &shown,
                &label_x,
                &label_y,
                label_light.as_deref(),
                &style_list(&[
                    ("fill", label_css.clone()),
                    ("font-size", Some(font_size.clone())),
                ]),
            ));
        } else {
            let custom_fill = custom.fill.clone().filter(|fill| !fill.is_empty());
            let opacity = if custom_fill.is_some() { "1" } else { "0" };
            let fill = custom_fill
                .clone()
                .unwrap_or_else(|| "transparent".to_owned());
            group.push_child(
                Element::new("path")
                    .with("d", path)
                    .with("fill", fill.clone())
                    .with("fill-opacity", opacity)
                    .with(
                        "style",
                        style_list(&[
                            ("fill-opacity", Some(opacity.to_owned())),
                            ("fill", Some(fill)),
                        ]),
                    ),
            );
            // The label: the style's colour (`??`), else `vennSetTextColor`, else the default text
            // colour, each as the scheme gives it.
            let (label_light, label_css) = match custom.color.clone() {
                Some(colour) => (Some(colour.clone()), Some(colour)),
                None => {
                    let light = schemes.light.intersection_text();
                    let dark = schemes
                        .dark
                        .as_ref()
                        .map(|scheme| scheme.intersection_text());
                    if light.is_some() || dark.clone().flatten().is_some() {
                        vars.set(
                            "--vennSetTextColor".to_owned(),
                            light.clone(),
                            dark.flatten(),
                        );
                        (light, Some("var(--vennSetTextColor)".to_owned()))
                    } else {
                        (None, None)
                    }
                }
            };
            group.push_child(label_element(
                &shown,
                &label_x,
                &label_y,
                label_light.as_deref(),
                &style_list(&[("fill", label_css), ("font-size", Some(font_size.clone()))]),
            ));
        }
        drawing.push_child(group);
    }

    // The text nodes, grouped by the area they are in, in the order of their first node
    // (`vennRenderer.ts` 259-269).
    let mut order: Vec<String> = Vec::new();
    let mut by_area: HashMap<String, Vec<&TextNode>> = HashMap::new();
    for node in &diagram.text_nodes {
        let key = stable_key(&node.sets);
        if !by_area.contains_key(&key) {
            order.push(key.clone());
        }
        by_area.entry(key).or_default().push(node);
    }
    if !diagram.text_nodes.is_empty() {
        let mut text_group = Element::new("g").with("class", "venn-text-nodes");
        for key in &order {
            let Some(area) = layout_by_key.get(key.as_str()) else {
                continue;
            };
            let nodes = &by_area[key];
            text_group.push_child(text_area(
                area,
                nodes,
                &TextAreaContext {
                    scale,
                    use_debug_layout: config.use_debug_layout,
                    styles: &styles,
                    set_text: schemes.light.set_text(),
                    set_text_dark: schemes
                        .dark
                        .as_ref()
                        .and_then(|scheme| scheme.set_text().map(str::to_owned)),
                    has_label: labelled_by_key.get(key.as_str()).copied().unwrap_or(false),
                },
                &mut vars,
                &mut texts,
            )?);
        }
        drawing.push_child(text_group);
    }

    // The title: `text.venn-title` before the group of the drawing (`vennRenderer.ts` 76-87).
    let mut children: Vec<Element> = Vec::new();
    if !diagram.title.is_empty() {
        let title_light = schemes.light.title().map(str::to_owned);
        let title_dark = schemes
            .dark
            .as_ref()
            .map(|scheme| scheme.title().map(str::to_owned));
        let mut title_style = String::new();
        if title_light.is_some() || title_dark.clone().flatten().is_some() {
            vars.set(
                "--vennTitleTextColor".to_owned(),
                title_light.clone(),
                title_dark.flatten(),
            );
            title_style = "fill: var(--vennTitleTextColor);".to_owned();
        }
        texts.push(diagram.title.clone());
        let mut title = Element::new("text")
            .with("class", "venn-title")
            .with("font-size", format!("{}px", js_number(32.0 * scale)))
            .with("text-anchor", "middle")
            .with("dominant-baseline", "middle")
            .with("x", "50%")
            .with("y", js_number(32.0 * scale))
            .text(diagram.title.as_str());
        if let Some(fill) = title_light {
            title.set("fill", fill);
        }
        if !title_style.is_empty() {
            title.set("style", title_style);
        }
        children.push(title);
    }
    children.push(drawing);

    // The font: the subset of the text the diagram draws, and the family its style sheet names.
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    let collapsed: Vec<String> = texts
        .iter()
        .map(|text| collapse_white_space(text))
        .collect();
    let runs: Vec<Run<'_>> = collapsed
        .iter()
        .map(|text| Run { text, face: sans })
        .collect();
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let tokens = input.theme.light;
    let font_family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let text_size = tokens.font_size.px().unwrap_or(16.0);
    let own = style::own_rules(id, &font_family, &vars.light, vars.dark.as_ref());
    let style_sheet = base_style_sheet(
        id,
        input.theme,
        &font_family,
        &format!("{}px", number(text_size)),
        &families,
        &own,
    );

    let mut root = Element::new("svg")
        .with("id", id)
        .with("xmlns", SVG_NAMESPACE);
    configure_svg_size(&mut root, height, width, config.use_max_width);
    root.set(
        "viewBox",
        format!("0 0 {} {}", js_number(width), js_number(height)),
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "venn");
    let mut root = root
        .child(Element::new("style").raw_text(style_sheet))
        // The empty group `mermaidAPI` appends before the renderer draws.
        .child(Element::new("g"));
    for child in children {
        root = root.child(child);
    }
    Ok(root.render())
}

/// The layout's error as a render error: the reference throws the same messages from its layout.
fn layout_error(error: ferrigura_venn::Error) -> RenderError {
    RenderError::Layout(error.to_string())
}

/// `stableSetsKey`-free lookup of a datum's label on the chart: the label of its sets, else for a
/// single set its id, else nothing (`src/diagram.js` 109-116).
fn label_of<'a>(datum: &'a Datum, labels: &HashMap<String, &'a str>) -> Option<String> {
    if let Some(label) = labels.get(&datum.sets.join(",")) {
        return Some((*label).to_owned());
    }
    (datum.sets.len() == 1).then(|| datum.sets[0].clone())
}

/// The `d` of an intersection path as the reference writes it (`circlePath`, `arcsToPath`,
/// `src/diagram.js` 661-706). The library's string starts with a line feed, which the rendered
/// SVG does not keep (`docs-port/venn/NOTES.md` L5).
fn path_data(path: &IntersectionPath) -> String {
    let tokens: Vec<String> = match path {
        IntersectionPath::Empty => return "M 0 0".to_owned(),
        IntersectionPath::Circle(circle) => {
            let (x, y, r) = (
                js_number(circle.x),
                js_number(circle.y),
                js_number(circle.radius),
            );
            let minus_r = js_number(-circle.radius);
            let two_r = js_number(circle.radius * 2.0);
            let minus_two_r = js_number(-circle.radius * 2.0);
            vec![
                format!("\nM {x} {y}"),
                format!("\nm {minus_r} 0"),
                format!("\na {r} {r} 0 1 0 {two_r} 0"),
                format!("\na {r} {r} 0 1 0 {minus_two_r} 0"),
            ]
        }
        IntersectionPath::Arcs { start, arcs } => {
            let mut tokens = vec![format!("\nM {} {}", js_number(start.x), js_number(start.y))];
            for arc in arcs {
                tokens.push(format!(
                    "\nA {r} {r} 0 {large} {sweep} {x} {y}",
                    r = js_number(arc.radius),
                    large = u8::from(arc.large),
                    sweep = u8::from(arc.sweep),
                    x = js_number(arc.end.x),
                    y = js_number(arc.end.y),
                ));
            }
            tokens
        }
    };
    let joined = tokens.join(" ");
    joined
        .strip_prefix('\n')
        .map_or(joined.clone(), str::to_owned)
}

/// The `text` of a set label: its class, its anchor and baseline, its style, and one `tspan`.
fn label_element(text: &str, x: &str, y: &str, fill: Option<&str>, style: &str) -> Element {
    let mut label = Element::new("text")
        .with("class", "label")
        .with("text-anchor", "middle")
        .with("dy", ".35em")
        .with("x", x)
        .with("y", y);
    if let Some(fill) = fill {
        label.set("fill", fill);
    }
    label.with("style", style).child(
        Element::new("tspan")
            .with("x", x)
            .with("y", y)
            .with("dy", "0.35em")
            .text(text),
    )
}

/// `style` text of the properties that are set: `name: value;` for each, in order.
fn style_list(properties: &[(&str, Option<String>)]) -> String {
    properties
        .iter()
        .filter_map(|(name, value)| value.as_ref().map(|value| format!("{name}: {value};")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The custom properties of the diagram, per scheme: the light value and the dark one. A name with
/// a value in one scheme only is declared in that scheme only.
#[derive(Default)]
struct Vars {
    light: Declarations,
    dark: Option<Declarations>,
}

impl Vars {
    /// Declares `name` with its value per scheme. A name declared again (a shared colour, such as
    /// `vennSetTextColor` of every intersection) keeps one declaration, with the last value.
    fn set(&mut self, name: String, light: Option<String>, dark: Option<String>) {
        if let Some(value) = light {
            upsert(&mut self.light, name.clone(), value);
        }
        if let Some(value) = dark {
            upsert(self.dark.get_or_insert_with(Vec::new), name, value);
        }
    }
}

/// Sets the value of `name` in `declarations`, keeping the place of an existing declaration.
fn upsert(declarations: &mut Declarations, name: String, value: String) {
    match declarations
        .iter_mut()
        .find(|(existing, _)| *existing == name)
    {
        Some(entry) => entry.1 = value,
        None => declarations.push((name, value)),
    }
}

/// The style statements by their key (`buildStyleByKey`, `vennRenderer.ts` 17-29): the properties
/// of the statements with the same key are merged, the later value of a property winning.
struct StyleLookup {
    by_key: HashMap<String, CustomStyle>,
}

/// The properties a style statement sets that the diagram reads.
#[derive(Default, Clone)]
struct CustomStyle {
    fill: Option<String>,
    fill_opacity: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<String>,
    color: Option<String>,
}

impl StyleLookup {
    fn new(entries: &[StyleEntry]) -> Self {
        let mut by_key: HashMap<String, CustomStyle> = HashMap::new();
        for entry in entries {
            let style = by_key.entry(entry.targets.join("|")).or_default();
            for (name, value) in &entry.styles {
                let slot = match name.as_str() {
                    "fill" => &mut style.fill,
                    "fill-opacity" => &mut style.fill_opacity,
                    "stroke" => &mut style.stroke,
                    "stroke-width" => &mut style.stroke_width,
                    "color" => &mut style.color,
                    _ => continue,
                };
                *slot = Some(value.clone());
            }
        }
        Self { by_key }
    }

    fn of(&self, key: &str) -> CustomStyle {
        self.by_key.get(key).cloned().unwrap_or_default()
    }
}

/// What the text areas take from the diagram.
struct TextAreaContext<'a> {
    scale: f64,
    use_debug_layout: bool,
    styles: &'a StyleLookup,
    /// `vennSetTextColor` of the light scheme, the colour of the text nodes.
    set_text: Option<&'a str>,
    /// `vennSetTextColor` of the dark scheme, when there is one.
    set_text_dark: Option<String>,
    /// Whether the area's datum has a label, which moves the grid down (`vennRenderer.ts` 310-311).
    has_label: bool,
}

/// One area of the text nodes (`renderTextNodes`, `vennRenderer.ts` 277-373): the centre and the
/// inner radius of the area, the grid of its nodes, and each node as a text in its cell.
fn text_area(
    area: &AreaLayout,
    nodes: &[&TextNode],
    context: &TextAreaContext<'_>,
    vars: &mut Vars,
    texts: &mut Vec<String>,
) -> Result<Element, RenderError> {
    let scale = context.scale;
    let centre_x = area.text.x;
    let centre_y = area.text.y;
    let min_circle_radius = js_min_all(area.circles.iter().map(|circle| circle.radius));
    let inner_raw = js_min_all(area.circles.iter().map(|circle| {
        circle.radius - ferrigura_jsmath::hypot(centre_x - circle.x, centre_y - circle.y)
    }));
    let mut inner_radius = if inner_raw.is_finite() {
        js_max(0.0, inner_raw)
    } else {
        0.0
    };
    if inner_radius == 0.0 && min_circle_radius.is_finite() {
        inner_radius = min_circle_radius * 0.6;
    }

    let mut group = Element::new("g")
        .with("class", "venn-text-area")
        .with("font-size", format!("{}px", js_number(40.0 * scale)));
    if context.use_debug_layout {
        group.push_child(
            Element::new("circle")
                .with("class", "venn-text-debug-circle")
                .with("cx", js_number(centre_x))
                .with("cy", js_number(centre_y))
                .with("r", js_number(inner_radius))
                .with("fill", "none")
                .with("stroke", "purple")
                .with("stroke-width", js_number(1.5 * scale))
                .with(
                    "stroke-dasharray",
                    format!("{} {}", js_number(6.0 * scale), js_number(4.0 * scale)),
                ),
        );
    }

    let inner_width = js_max(80.0 * scale, inner_radius * 2.0 * 0.95);
    let inner_height = js_max(60.0 * scale, inner_radius * 2.0 * 0.95);
    let has_label = context.has_label;
    let label_offset_base = if has_label {
        js_min(32.0 * scale, inner_radius * 0.25)
    } else {
        0.0
    };
    let label_offset = label_offset_base + if nodes.len() <= 2 { 30.0 * scale } else { 0.0 };
    let start_x = centre_x - inner_width / 2.0;
    let start_y = centre_y - inner_height / 2.0 + label_offset;
    let count = nodes.len();
    let cols = ((count as f64).sqrt().ceil()).max(1.0) as usize;
    let rows = (count.div_ceil(cols)).max(1);
    let cell_width = inner_width / cols as f64;
    let cell_height = inner_height / rows as f64;

    for (index, node) in nodes.iter().enumerate() {
        let col = index % cols;
        let row = index / cols;
        let x = start_x + cell_width * (col as f64 + 0.5);
        let y = start_y + cell_height * (row as f64 + 0.5);

        if context.use_debug_layout {
            group.push_child(
                Element::new("rect")
                    .with("class", "venn-text-debug-cell")
                    .with("x", js_number(start_x + cell_width * col as f64))
                    .with("y", js_number(start_y + cell_height * row as f64))
                    .with("width", js_number(cell_width))
                    .with("height", js_number(cell_height))
                    .with("fill", "none")
                    .with("stroke", "teal")
                    .with("stroke-width", js_number(scale))
                    .with(
                        "stroke-dasharray",
                        format!("{} {}", js_number(4.0 * scale), js_number(3.0 * scale)),
                    ),
            );
        }

        let box_width = cell_width * 0.9;
        let shown = if node.label.is_empty() {
            node.id.clone()
        } else {
            node.label.clone()
        };
        let drawn = collapse_white_space(&shown);
        let font_size = 40.0 * scale;
        let lines: Vec<Line> = text_node_lines(&drawn, box_width, font_size)?;
        let metrics = text_box(
            face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
            font_size,
        )
        .map_err(RenderError::Text)?;
        let block = lines.len() as f64 * metrics.height;
        let top = y - block / 2.0;
        let left = x - box_width / 2.0;

        // The colour: the node's own style colour, else `vennSetTextColor`, as a custom property.
        let custom = context
            .styles
            .of(&node.id)
            .color
            .filter(|colour| !colour.is_empty());
        let (fill_light, fill_css) = match custom {
            Some(colour) => (Some(colour.clone()), Some(colour)),
            None => {
                let light = context.set_text.map(str::to_owned);
                let dark = context.set_text_dark.clone();
                if light.is_some() || dark.is_some() {
                    vars.set("--vennTextNodeColor".to_owned(), light.clone(), dark);
                    (light, Some("var(--vennTextNodeColor)".to_owned()))
                } else {
                    (None, None)
                }
            }
        };

        let mut text = Element::new("text").with("class", "venn-text-node");
        if let Some(fill) = fill_light {
            text.set("fill", fill);
        }
        if let Some(css) = fill_css {
            text.set("style", format!("fill: {css};"));
        }
        for (line_index, line) in lines.iter().enumerate() {
            let line_x = left + floor_layout_unit((box_width - line.width) / 2.0);
            let line_y = top + line_index as f64 * metrics.height + metrics.top;
            texts.push(line.text.clone());
            text = text.child(
                Element::new("tspan")
                    .with("x", js_number(line_x))
                    .with("y", js_number(line_y))
                    .text(line.text.as_str()),
            );
        }
        group.push_child(text);
    }
    Ok(group)
}

/// A length rounded down to the layout unit of 1/64 px, as Chromium keeps a position.
fn floor_layout_unit(length: f64) -> f64 {
    (length * 64.0 + 1e-6).floor() / 64.0
}
