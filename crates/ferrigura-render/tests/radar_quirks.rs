//! One test per quirk of the radar renderer that changes the output (`docs-port/radar/NOTES.md`,
//! section 6.11, and the probes `Q1` to `Q25` of section 6). Each renders a text through the same path
//! as the facade, and checks the markup that the reference writes for it.

use ferrigura_model::config::{effective, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::radar::render;

const ID: &str = "tst";

/// Renders a radar text with the given light and dark variables (`dark` is the second scheme).
fn draw_schemes(text: &str, light: &Value, dark: Option<&Value>) -> String {
    let directive = ferrigura_syntax::config::read(text).expect("the directives read");
    let configured = effective(None, Some("radar"), &directive);
    let config = typed(&configured).expect("the configuration is typed");
    let diagram = ferrigura_syntax::radar::parse(text).expect("the radar parses");
    let light_tokens = ThemeTokens::from_value(light);
    let dark_tokens = dark.map(ThemeTokens::from_value);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.radar,
        globals: &globals,
        theme: ThemeView {
            light: &light_tokens,
            dark: dark_tokens.as_ref(),
        },
        id: ID,
    })
    .expect("the radar renders")
}

/// Renders a radar text with the redux colour theme.
fn draw(text: &str) -> String {
    draw_schemes(text, &Theme::ReduxColor.variables(None), None)
}

/// Renders a radar text with the redux colour theme and the given overrides of `themeVariables`.
fn draw_with(text: &str, overrides: Value) -> String {
    draw_schemes(text, &Theme::ReduxColor.variables(Some(&overrides)), None)
}

/// The start tags of the elements with the class, in document order.
fn tags_with_class(svg: &str, class: &str) -> Vec<String> {
    let needle = format!("class=\"{class}\"");
    let mut tags = Vec::new();
    let mut from = 0;
    while let Some(found) = svg[from..].find(&needle) {
        let at = from + found;
        let start = svg[..at]
            .rfind('<')
            .expect("an element starts before its class");
        let end = at + svg[at..].find('>').expect("an element ends") + 1;
        tags.push(svg[start..end].to_owned());
        from = end;
    }
    tags
}

/// The number of elements with the class.
fn count_class(svg: &str, class: &str) -> usize {
    tags_with_class(svg, class).len()
}

/// The `r` of each graticule circle, in document order.
fn ring_radii(svg: &str) -> Vec<String> {
    tags_with_class(svg, "radarGraticule")
        .iter()
        .filter_map(|tag| attribute(tag, "r"))
        .collect()
}

/// The value of an attribute of a start tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = start + tag[start..].find('"')?;
    Some(tag[start..end].to_owned())
}

/// The `d` of the curve with the index, from its path.
fn curve_path(svg: &str, index: usize) -> String {
    let tag = tags_with_class(svg, &format!("radarCurve-{index}"))
        .into_iter()
        .find(|tag| tag.starts_with("<path"))
        .expect("the curve is a path");
    attribute(&tag, "d").expect("the path has its data")
}

/// The text of the first element with the class, between its tags.
fn text_of(svg: &str, class: &str) -> String {
    let needle = format!("class=\"{class}\"");
    let at = svg.find(&needle).expect("the class is written");
    let open = at + svg[at..].find('>').expect("the start tag ends") + 1;
    let close = open + svg[open..].find("</text>").expect("the text ends");
    svg[open..close].to_owned()
}

/// The `<style>` element of the SVG.
fn style_of(svg: &str) -> &str {
    let start = svg.find("<style>").expect("the SVG has a style element");
    let end = svg[start..].find("</style>").expect("the style ends") + start;
    &svg[start..end]
}

/// The first rule of the style that begins with the selector.
fn rule(style: &str, selector: &str) -> Option<String> {
    let start = style.find(&format!("#{ID} {selector}{{"))?;
    let end = start + style[start..].find('}')?;
    Some(style[start..=end].to_owned())
}

/// The root element's start tag.
fn root_tag(svg: &str) -> &str {
    &svg[..svg.find('>').expect("the root starts") + 1]
}

const SIMPLE: &str = "radar-beta\n  axis A, B, C\n  curve c1{1, 2, 3}\n";

/// `ticks` is clamped to 32 by the DB, so 40 draws 32 circles, the last at the full radius (Q3).
#[test]
fn ticks_above_32_are_clamped_to_32() {
    let svg = draw(&format!("{SIMPLE}  ticks 40\n"));
    let radii = ring_radii(&svg);
    assert_eq!(radii.len(), 32);
    assert_eq!(radii[0], "9.375");
    assert_eq!(radii[31], "300");
}

/// A fraction of ticks draws one more ring: 2.5 gives rings at 120, 240 and 360, beyond the frame
/// (Q4).
#[test]
fn a_fraction_of_ticks_draws_one_more_ring() {
    let svg = draw(&format!("{SIMPLE}  ticks 2.5\n"));
    assert_eq!(ring_radii(&svg), ["120", "240", "360"]);
}

/// Zero ticks draw no ring (Q5).
#[test]
fn zero_ticks_draw_no_ring() {
    let svg = draw(&format!("{SIMPLE}  ticks 0\n"));
    assert!(ring_radii(&svg).is_empty());
}

/// `showLegend false` removes the legend and nothing else (Q6).
#[test]
fn show_legend_false_removes_only_the_legend() {
    let svg = draw(&format!(
        "{SIMPLE}  curve c2{{3, 2, 1}}\n  showLegend false\n"
    ));
    assert_eq!(count_class(&svg, "radarLegendBox-0"), 0);
    assert_eq!(count_class(&svg, "radarLegendText"), 0);
    assert_eq!(count_class(&svg, "radarAxisLine"), 3);
    assert_eq!(count_class(&svg, "radarCurve-1"), 1);
    assert_eq!(count_class(&svg, "radarTitle"), 1);
}

/// A diagram with no axis and no curve draws the five rings and the title, and nothing else (Q7).
#[test]
fn a_header_only_diagram_draws_the_rings_and_the_title() {
    let svg = draw("radar-beta\n");
    assert_eq!(ring_radii(&svg).len(), 5);
    assert_eq!(count_class(&svg, "radarAxisLine"), 0);
    assert_eq!(count_class(&svg, "radarCurve-0"), 0);
    assert!(
        svg.contains("<text class=\"radarTitle\" x=\"0\" y=\"-350\"></text>"),
        "{svg}"
    );
}

/// The title element is always written, empty when the diagram has no title (Q19).
#[test]
fn the_title_element_is_written_even_when_empty() {
    let svg = draw(SIMPLE);
    assert_eq!(count_class(&svg, "radarTitle"), 1);
    assert_eq!(text_of(&svg, "radarTitle"), "");
}

/// All values zero with no maximum: the scale's maximum and minimum are both zero, so the radius is
/// `0 / 0`, and the path is written with `NaN` (Q1).
#[test]
fn all_zero_values_without_a_maximum_draw_nan() {
    let svg = draw("radar-beta\n  axis A, B, C\n  curve c1{0, 0, 0}\n");
    assert!(
        curve_path(&svg, 0).starts_with("MNaN,NaN CNaN,NaN"),
        "{svg}"
    );
}

/// A maximum equal to the minimum gives the same `NaN` (NOTES, section 6.11, item 1).
#[test]
fn a_maximum_equal_to_the_minimum_draws_nan() {
    let svg = draw("radar-beta\n  axis A, B, C\n  curve c1{2, 2, 2}\n  min 2\n  max 2\n");
    assert!(curve_path(&svg, 0).starts_with("MNaN,NaN"), "{svg}");
}

/// A maximum below the minimum divides by a negative range, and the value that equals the maximum
/// draws at the full radius (NOTES, section 9, the edge cases of the parser).
#[test]
fn a_maximum_below_the_minimum_divides_by_the_negative_range() {
    use ferrigura_render::radar::geometry::relative_radius;
    assert_eq!(relative_radius(1.0, 5.0, 1.0, 300.0), 300.0);
}

/// Values below the minimum draw at the centre, and values above the maximum at the frame (Q11,
/// Q12): the first point of `c1{0, 4, 15, 0}` on a scale of 1 to 10 is the centre.
#[test]
fn values_are_clipped_to_the_scale() {
    let svg = draw("radar-beta\n  axis A, B, C, D\n  curve c1{0, 4, 15, 0}\n  min 1\n  max 10\n");
    assert!(curve_path(&svg, 0).starts_with("M0,0 "), "{svg}");
}

/// A curve with the wrong number of entries draws nothing, but its legend item is drawn, and the other
/// curves keep their index (Q2, Q8).
#[test]
fn a_curve_with_a_missing_entry_is_skipped_and_its_legend_is_drawn() {
    let svg = draw("radar-beta\n  axis A, B, C\n  curve bad{1, 2}\n  curve good{1, 2, 3}\n");
    assert_eq!(count_class(&svg, "radarCurve-0"), 0);
    assert_eq!(count_class(&svg, "radarCurve-1"), 1);
    assert_eq!(count_class(&svg, "radarLegendBox-0"), 1);
    assert_eq!(count_class(&svg, "radarLegendBox-1"), 1);
}

/// The style has a rule for each of the first twelve curve colours only, so the thirteenth curve has
/// no rule and draws with the root's fill, and no presentation attribute (computed-probe, section 6.7).
#[test]
fn a_thirteenth_curve_has_no_colour_rule() {
    let mut text = String::from("radar-beta\n  axis A, B, C\n");
    for index in 1..=13 {
        text.push_str(&format!(
            "  curve c{index}{{{index}, {}, {}}}\n",
            index + 1,
            13 - index
        ));
    }
    let svg = draw(&text);
    let style = style_of(&svg);
    assert!(rule(style, ".radarCurve-11").is_some(), "{style}");
    assert!(rule(style, ".radarCurve-12").is_none(), "{style}");
    let tag = tags_with_class(&svg, "radarCurve-12").remove(0);
    assert!(!tag.contains("fill="), "{tag}");
    assert!(!tag.contains("stroke="), "{tag}");
}

/// A title without markup is not sanitised (DOMPurify returns text without `<` as it is), so its `&`
/// is written once and shows as typed.
#[test]
fn a_title_without_markup_shows_its_ampersand_as_typed() {
    let svg = draw("radar-beta\n  title Hello & more\n  axis A, B, C\n");
    assert_eq!(text_of(&svg, "radarTitle"), "Hello &amp; more");
}

/// A title with markup is sanitised, which writes its `&` as `&amp;`; the reference writes that text
/// as it is, so the title shows `&amp;` where the author wrote `&`, and the `<b>` as text (NOTES,
/// section 6.5, and O5). The markup escapes the sanitised text once more.
#[test]
fn a_title_with_markup_shows_the_sanitised_text_double_encoded() {
    let svg = draw("radar-beta\n  title Hello <b>bold</b> & more\n  axis A, B, C\n");
    assert_eq!(
        text_of(&svg, "radarTitle"),
        "Hello &lt;b&gt;bold&lt;/b&gt; &amp;amp; more"
    );
}

/// An entity such as `#35;` in a label is decoded when the SVG is written, so the label shows `#`
/// (Q9).
#[test]
fn an_entity_in_a_label_shows_its_character() {
    let svg = draw("radar-beta\n  axis A[\"Item #35; one\"], B, C\n");
    assert_eq!(text_of(&svg, "radarAxisLabel"), "Item # one");
}

/// A label is literal text: raw HTML in it is escaped and shown as typed (Q10).
#[test]
fn a_label_is_literal_text() {
    let svg = draw("radar-beta\n  axis A[\"<b>Dog</b>\"], B, C\n");
    assert_eq!(text_of(&svg, "radarAxisLabel"), "&lt;b&gt;Dog&lt;/b&gt;");
}

/// The axis labels are anchored by their side: the first axis is above the centre, the second on the
/// right and below it, the third on the left and below it (renderer.ts 118-119).
#[test]
fn axis_labels_are_anchored_by_their_side() {
    let svg = draw(SIMPLE);
    let labels = tags_with_class(&svg, "radarAxisLabel");
    assert_eq!(
        attribute(&labels[0], "text-anchor").as_deref(),
        Some("middle")
    );
    assert_eq!(
        attribute(&labels[0], "dominant-baseline").as_deref(),
        Some("auto")
    );
    assert_eq!(
        attribute(&labels[1], "text-anchor").as_deref(),
        Some("start")
    );
    assert_eq!(
        attribute(&labels[1], "dominant-baseline").as_deref(),
        Some("hanging")
    );
    assert_eq!(attribute(&labels[2], "text-anchor").as_deref(), Some("end"));
}

/// The legend is in a column at the top right, and the title sits above the chart (renderer.ts 220-222,
/// 41-45).
#[test]
fn the_legend_sits_at_the_top_right_and_the_title_above_the_chart() {
    let svg = draw(&format!("{SIMPLE}  curve c2{{3, 2, 1}}\n"));
    assert!(
        svg.contains("<g transform=\"translate(262.5, -262.5)\">"),
        "{svg}"
    );
    assert!(
        svg.contains("<g transform=\"translate(262.5, -242.5)\">"),
        "{svg}"
    );
}

/// A numeric `fontSize` is written without a unit, which the browser drops, so the title keeps the
/// inherited size (computed-probe, section 6.7).
#[test]
fn a_numeric_font_size_is_written_unitless() {
    let variables = Theme::ReduxColor.variables(Some(&Value::from(serde_json::json!({
        "fontSize": 80,
    }))));
    let svg = draw_schemes(SIMPLE, &variables, None);
    assert!(
        style_of(&svg).contains(".radarTitle{font-size:80;"),
        "{}",
        style_of(&svg)
    );
}

/// `legendBoxSize` is not read: the legend box is 12 by 12 whatever the theme says (Q17).
#[test]
fn legend_box_size_is_ignored() {
    let svg = draw_with(
        &format!("{SIMPLE}  curve c2{{3, 2, 1}}\n"),
        serde_json::json!({"radar": {"legendBoxSize": 30}}).into(),
    );
    let boxes = tags_with_class(&svg, "radarLegendBox-0");
    assert_eq!(attribute(&boxes[0], "width").as_deref(), Some("12"));
    assert_eq!(attribute(&boxes[0], "height").as_deref(), Some("12"));
}

/// `useMaxWidth: false` writes the absolute `width` and `height`, and no `max-width` (Q15).
#[test]
fn use_max_width_false_writes_the_absolute_size() {
    let svg = draw(&format!(
        "---\nconfig:\n  radar:\n    useMaxWidth: false\n---\n{SIMPLE}"
    ));
    let root = root_tag(&svg);
    assert!(root.contains("width=\"700\""), "{root}");
    assert!(root.contains("height=\"700\""), "{root}");
    assert!(!root.contains("max-width"), "{root}");
}

/// A width of 0 gives a radius of 0, and the total width is the two margins (Q25).
#[test]
fn a_zero_width_gives_a_zero_radius() {
    let svg = draw(&format!(
        "---\nconfig:\n  radar:\n    width: 0\n---\n{SIMPLE}"
    ));
    assert!(
        root_tag(&svg).contains("max-width: 100px;"),
        "{}",
        root_tag(&svg)
    );
    assert!(svg.contains("viewBox=\"0 0 100 700\""), "{svg}");
    assert!(ring_radii(&svg).iter().all(|radius| radius == "0"), "{svg}");
}

/// A tension of 0 draws straight segments: the control points of each segment are its end points
/// (Q16). With the scale 0 to 4, the first point is at (0, -75) and the second at (150, 0).
#[test]
fn a_tension_of_0_draws_straight_segments() {
    let svg = draw(
        "---\nconfig:\n  radar:\n    curveTension: 0\n---\nradar-beta\n  axis A, B, C, D\n  curve c1{1, 2, 3, 4}\n  min 0\n  max 4\n",
    );
    assert!(
        curve_path(&svg, 0).contains("C0,-75 150,0 150,0"),
        "{}",
        curve_path(&svg, 0)
    );
}

/// A single axis draws a closed curve through its one point (Q22).
#[test]
fn one_axis_draws_a_degenerate_closed_curve() {
    let svg = draw("radar-beta\n  axis Only\n  curve c1{4}\n");
    let path = curve_path(&svg, 0);
    assert!(path.starts_with('M') && path.ends_with(" Z"), "{path}");
}

/// A value that is an infinite number makes the scale's maximum infinite, and the point of that value
/// is `NaN`, as the reference's is (NOTES, section 9).
#[test]
fn an_infinite_entry_draws_nan() {
    let big = "1".repeat(400);
    let svg = draw(&format!(
        "radar-beta\n  axis A, B, C\n  curve c1{{1, {big}, 3}}\n"
    ));
    assert!(curve_path(&svg, 0).contains("NaN"), "{svg}");
}

/// The polygon graticule draws each ring as a polygon with a point for each axis (Q13).
#[test]
fn the_polygon_graticule_draws_polygons_with_a_point_for_each_axis() {
    let svg = draw(
        "radar-beta\n  axis A, B, C, D, E\n  curve c1{1, 2, 3, 4, 5}\n  ticks 3\n  graticule polygon\n",
    );
    let rings = tags_with_class(&svg, "radarGraticule");
    assert_eq!(rings.len(), 3);
    for ring in &rings {
        assert!(ring.starts_with("<polygon"), "{ring}");
        let points = attribute(ring, "points").expect("the polygon has points");
        assert_eq!(points.split(' ').count(), 5);
    }
    assert!(tags_with_class(&svg, "radarCurve-0")[0].starts_with("<polygon"));
}

/// The theme's group of the radar options sets a value over the default (`styles.ts` 35-38): an axis
/// colour from `themeVariables.radar` is in the custom property and in the axis line.
#[test]
fn a_radar_option_of_the_theme_overrides_the_default() {
    let svg = draw_with(
        SIMPLE,
        serde_json::json!({"radar": {"axisColor": "#FF0000", "axisStrokeWidth": 5}}).into(),
    );
    assert!(
        style_of(&svg).contains("--radarAxisColor:#FF0000;"),
        "{}",
        style_of(&svg)
    );
    let line = tags_with_class(&svg, "radarAxisLine").remove(0);
    assert_eq!(attribute(&line, "stroke").as_deref(), Some("#FF0000"));
    assert_eq!(attribute(&line, "stroke-width").as_deref(), Some("5"));
}

/// The redux themes give no `radar` section, so the axis is the default theme's `#333333`, and the
/// graticule is `#DEDEDE` (`theme-default.js` 340-349).
#[test]
fn the_redux_themes_draw_with_the_default_radar_options() {
    let svg = draw(SIMPLE);
    assert!(style_of(&svg).contains("--radarAxisColor:#333333;--radarGraticuleColor:#DEDEDE;"));
    assert!(style_of(&svg).contains(".radarAxisLabel{font-size:12px;"));
}

/// The first curve takes the theme's `cScale0`, in its custom property and in its presentation
/// attributes (`styles.ts` 16-22, decision 0007).
#[test]
fn the_curves_take_the_colour_scales() {
    let svg = draw(SIMPLE);
    assert!(style_of(&svg).contains("--radarCurveColor0:#f4a8ff;"));
    let path = tags_with_class(&svg, "radarCurve-0").remove(0);
    assert_eq!(attribute(&path, "fill").as_deref(), Some("#f4a8ff"));
    assert_eq!(attribute(&path, "fill-opacity").as_deref(), Some("0.5"));
}

/// The colour limit is `THEME_COLOR_LIMIT`: with 2, only two curves have a rule (`styles.ts` 12).
#[test]
fn the_colour_limit_is_the_theme_colour_limit() {
    let svg = draw_with(SIMPLE, serde_json::json!({"THEME_COLOR_LIMIT": 2}).into());
    let style = style_of(&svg);
    assert!(rule(style, ".radarCurve-1").is_some());
    assert!(rule(style, ".radarCurve-2").is_none());
}

/// With both schemes, the dark colours are declared under the `prefers-color-scheme: dark` query.
#[test]
fn the_dark_scheme_is_declared_under_the_media_query() {
    let light = Theme::ReduxColor.variables(None);
    let dark = Theme::ReduxDarkColor.variables(None);
    let svg = draw_schemes(SIMPLE, &light, Some(&dark));
    let style = style_of(&svg);
    assert!(
        style.contains("@media (prefers-color-scheme: dark){#tst{--radarTitleColor:"),
        "{style}"
    );
    assert!(style.contains("--radarAxisColor:#333333;"), "{style}");
}

/// The accessible title and description are the first children of the root, and the root refers to
/// them.
#[test]
fn the_accessible_names_are_the_first_children_of_the_root() {
    let svg = draw("radar-beta\n  accTitle: Radar\n  accDescr: About the radar\n  axis A, B, C\n");
    let root = root_tag(&svg);
    assert!(
        root.contains("aria-labelledby=\"chart-title-tst\""),
        "{root}"
    );
    assert!(
        root.contains("aria-describedby=\"chart-desc-tst\""),
        "{root}"
    );
    assert!(root.contains("aria-roledescription=\"radar\""), "{root}");
    assert!(svg.find("<title").expect("a title") < svg.find("<style>").expect("a style"));
}
