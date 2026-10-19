//! The tree view renderer: the arithmetic of the rows, the description column and the highlights, the
//! size of the root, the styles, and the reproduced quirks of `packages/mermaid/src/diagrams/treeView`
//! (`renderer.ts`, `styles.ts`, `db.ts`). `docs-port/tree-view/NOTES.md`, section 6, has the reference's
//! file and line for each rule, and the numbers below are the reference's, probed or read from its
//! rendered SVG. The reference has no renderer spec, so each test here pins one rule or one quirk.

#[path = "tree_view/support.rs"]
mod support;

use ferrigura_model::config::{TreeViewDiagramConfig, effective, typed};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, measure_box, text_box};
use support::{
    IconKind, draw, draw_configured, draw_section, lines, row_named, rows, svg_of, svg_with,
};

/// The front matter that sets the tree view's configuration, as a text.
fn with_config(config: &str, body: &str) -> String {
    format!("---\nconfig:\n  treeView:\n{config}---\ntreeView-beta\n{body}")
}

/// The width of a label's text in the regular face at `size` pixels, as its `getBBox` gives it.
fn regular_width(text: &str, size: f64) -> f64 {
    measure_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        text,
        size,
    )
    .expect("the bundled face measures")
    .width()
}

/// The box height of a line of the regular face at `size` pixels.
fn regular_height(size: f64) -> f64 {
    text_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
        size,
    )
    .expect("the bundled face has metrics")
    .height
}

// ---------------------------------------------------------------------------------------------------
// The rows (`drawNode`, `positionLabel`)
// ---------------------------------------------------------------------------------------------------

/// `/` is always the first row: a directory, bold, and one row for each node below it (R1).
#[test]
fn the_root_row_is_always_drawn_as_a_bold_directory() {
    let svg = svg_of("treeView-beta\n  a");
    let root = &rows(&svg)[0];
    assert_eq!(root.label.text, "/");
    assert!(root.label.has_class("treeView-node-dir"));
    assert_eq!(root.label.number("x"), 5.0);
    assert_eq!(root.label.number("y"), 15.0);
}

/// An empty diagram is the root alone, one row high and seventeen units wide (R1, `P-empty`).
#[test]
fn an_empty_diagram_is_the_root_alone() {
    let svg = svg_of("treeView-beta\n");
    assert_eq!(svg.attr("viewBox"), Some("-0.5 0 17.078125 30"));
    assert_eq!(rows(&svg).len(), 1);
}

/// The connector of a top-level row starts ten units left of the row's indent, so it runs out of the
/// viewBox on the left (R2, `P-root-one`).
#[test]
fn the_connector_of_a_root_level_row_starts_ten_units_left() {
    let svg = svg_of("treeView-beta\n  a");
    let connector = &lines(&svg)[0];
    assert_eq!(connector.number("x1"), -10.0);
    assert_eq!(connector.number("x2"), 0.0);
    assert_eq!(connector.attr("class"), Some("treeView-node-line"));
}

/// A row's label is at its indent plus the padding; an icon moves it by eighteen units more, and the
/// icon is top-aligned at the padding, fourteen units square (R3, `renderer.ts` 70-79 and 89).
#[test]
fn an_icon_moves_its_label_by_eighteen_units_and_sits_at_the_padding() {
    let text = with_config("      showIcons: true\n", "  src/\n    App.tsx\n");
    let svg = svg_of(&text);
    let app = row_named(&svg, "App.tsx");
    assert_eq!(app.label.number("x"), 53.0, "30 indent, 5 padding, 18 icon");
    assert_eq!(app.label.number("y"), 75.0, "the row's centre: 60 + 30 / 2");
    let icon = app.icon_group.expect("showIcons draws the file icon");
    assert_eq!(icon.attr("transform"), Some("translate(35, 65)"));
    assert_eq!(icon.attr("class"), Some("treeView-node-icon"));
}

/// A row is as high as its label's text box plus twice the vertical padding. The rows stack, and the
/// viewBox is their sum (`P-root-one`: two rows of thirty units).
#[test]
fn the_rows_stack_by_their_height() {
    let svg = svg_of("treeView-beta\n  a\n  b");
    assert_eq!(svg.attr("viewBox"), Some("-0.5 0 34.0625 90"));
    assert_eq!(
        rows(&svg)[2].label.number("y"),
        75.0,
        "the third row's centre: 60 + 15"
    );
}

// ---------------------------------------------------------------------------------------------------
// The highlights and the descriptions (phases 2 and 3 of `drawTree`)
// ---------------------------------------------------------------------------------------------------

/// Each highlight is sized to the tree's width so far, less its own indent, plus 8, and it widens the
/// tree to its right edge plus 2: the second highlight is sized from the width the first one gave
/// (R4, the reference's own numbers for the class annotations fixture).
#[test]
fn each_highlight_is_sized_from_the_width_so_far() {
    let text = "treeView-beta\n  src/\n  components/\n      Button.tsx :::highlight\n      Header.tsx\n  App.tsx :::highlight\n  index.js\n  package.json\n";
    let svg = svg_of(text);
    assert_eq!(svg.attr("viewBox"), Some("-0.5 0 136.953125 240"));
    let button = row_named(&svg, "Button.tsx")
        .highlight
        .expect("a highlight");
    assert_eq!(button.number("width"), 94.953125);
    assert_eq!(button.number("x"), 30.0);
    assert_eq!(button.number("y"), 91.0, "one unit below the row's top");
    assert_eq!(button.number("height"), 28.0, "two less than the row");
    let app = row_named(&svg, "App.tsx").highlight.expect("a highlight");
    assert_eq!(
        app.number("width"),
        119.953125,
        "from the 126.953125 the first one left"
    );
    assert_eq!(app.number("x"), 15.0);
}

/// A row with a highlight widens the tree even when no rectangle is drawn after it: the tree is at
/// least the highlight's right edge plus two (R4, `P-highlight2`).
#[test]
fn a_highlight_widens_the_tree_by_ten_units_at_least() {
    let plain = svg_of("treeView-beta\n  a\n  b\n  c");
    let marked = svg_of("treeView-beta\n  a :::highlight\n  b :::highlight\n  c");
    // The widest label is `b`, 9.0625 wide, so each row is 34.0625 wide with its indent. The first
    // highlight is 27.0625 wide (34.0625 - 15 + 8) and takes the tree to 44.0625; the second is
    // 37.0625 wide and takes it to 54.0625.
    assert_eq!(width_of(&plain), 34.0625);
    assert_eq!(width_of(&marked), 54.0625);
    let first = row_named(&marked, "a").highlight.expect("a highlight");
    assert_eq!(first.number("width"), 27.0625);
    let second = row_named(&marked, "b").highlight.expect("a highlight");
    assert_eq!(second.number("width"), 37.0625);
}

/// The descriptions share one column: the widest label's right edge plus 16, the same for every row,
/// including the rows without a description (R5, the reference's descriptions fixture).
#[test]
fn the_descriptions_share_one_column_after_the_widest_label() {
    let text = "treeView-beta\n  src/\n  index.js ## app entry point\n  config.ts ## runtime configuration\n  utils/ ## shared helpers\n  package.json ## project manifest\n  README.md\n";
    let svg = svg_of(text);
    assert_eq!(svg.attr("viewBox"), Some("-0.5 0 283.890625 210"));
    let columns: Vec<f64> = rows(&svg)
        .iter()
        .filter_map(|row| row.description.as_ref().map(|d| d.number("x")))
        .collect();
    assert_eq!(columns, vec![127.765625; 4]);
}

/// The column counts the labels that have no description: the widest label of the tree sets it,
/// whichever row it is on.
#[test]
fn the_column_counts_the_labels_without_a_description() {
    let long = "bbbbbbbbbbbbbbbbbbbbbbbb";
    let svg = svg_of(&format!("treeView-beta\n  a ## x\n  {long}"));
    let description = row_named(&svg, "a").description.expect("a description");
    let long_row = row_named(&svg, long);
    let expected = long_row.label.number("x") + regular_width(long, 16.0) + 16.0;
    assert_eq!(description.number("x"), expected);
}

// ---------------------------------------------------------------------------------------------------
// The size of the root (`configureSvgSize`, the viewBox)
// ---------------------------------------------------------------------------------------------------

/// The viewBox starts at minus half the line thickness, so that a line at x = 0 is not cut (R6).
#[test]
fn the_viewbox_starts_at_minus_half_the_line_thickness() {
    let svg = svg_of(&with_config("      lineThickness: 3\n", "  a\n"));
    assert!(
        svg.attr("viewBox")
            .expect("a viewBox")
            .starts_with("-1.5 0 ")
    );
    let connector = &lines(&svg)[0];
    assert_eq!(connector.attr("stroke-width"), Some("3"));
}

/// With `useMaxWidth` the root is `100%` wide and its maximum is the tree's width, and it has no
/// height; without it the root takes the width and the height (R7).
#[test]
fn use_max_width_sets_a_percentage_and_no_height() {
    let svg = svg_of("treeView-beta\n  a");
    assert_eq!(svg.attr("width"), Some("100%"));
    assert_eq!(svg.attr("style"), Some("max-width: 33.421875px;"));
    assert_eq!(svg.attr("height"), None);

    let fixed = svg_of(&with_config("      useMaxWidth: false\n", "  a\n"));
    assert_eq!(fixed.attr("style"), None);
    assert_eq!(fixed.attr("width"), Some("33.421875"));
    assert_eq!(fixed.attr("height"), Some("60"));
}

// ---------------------------------------------------------------------------------------------------
// The labels: the text they draw and what they measure
// ---------------------------------------------------------------------------------------------------

/// An empty label measures zero high, so its row is only the vertical padding high (R8).
#[test]
fn an_empty_label_is_as_high_as_its_padding() {
    let svg = svg_of("treeView-beta\n  \"\"");
    assert_eq!(
        svg.attr("viewBox"),
        Some("-0.5 0 25 40"),
        "the indent of 15 and the padding of 10"
    );
    assert_eq!(row_named(&svg, "").label.text, "");
}

/// A name that holds two spaces keeps them, and the label is wider by the width of the space
/// (R11, `white-space: pre`).
#[test]
fn consecutive_spaces_in_a_name_are_kept_and_measured() {
    let doubled = svg_of("treeView-beta\n  a  b");
    assert_eq!(row_named(&doubled, "a  b").label.text, "a  b");
    let single = svg_of("treeView-beta\n  a b");
    let space = regular_width(" ", 16.0);
    let difference = width_of(&doubled) - width_of(&single);
    assert!(
        (difference - space).abs() < 1.0 / 64.0,
        "one more space: {difference} against {space}"
    );
}

/// An entity in a name is measured as the placeholder the encoding wrote, and drawn as the character
/// it stands for (R12-entities, `P-entity-literal-amp`): `A #amp; B` is 116.921875 units wide.
#[test]
fn an_entity_is_measured_as_its_placeholder_and_drawn_as_the_character() {
    let svg = svg_of("treeView-beta\n  A #amp; B");
    assert_eq!(svg.attr("viewBox"), Some("-0.5 0 116.921875 60"));
    assert_eq!(row_named(&svg, "A & B").label.text, "A & B");
}

/// A plain `&` is drawn as `&` and written as `&amp;` (the escaping of the markup).
#[test]
fn a_plain_ampersand_is_escaped_in_the_markup() {
    let text = draw("treeView-beta\n  E & F").expect("the tree view renders");
    assert!(text.contains(">E &amp; F</text>"), "{text}");
}

/// A title line is stored and not drawn; the accessible title and description become the `title` and
/// `desc` of the root, which the root refers to (R10, `mermaidAPI.ts` 623).
#[test]
fn the_title_is_not_drawn_and_the_accessible_texts_are() {
    let text = "treeView-beta\naccTitle: Project layout\naccDescr: The files\ntitle My Tree\n  a\n";
    let svg = svg_of(text);
    let rendered = draw(text).expect("the tree view renders");
    assert!(!rendered.contains("My Tree"), "the title is not drawn");
    let title = svg
        .children
        .iter()
        .find(|child| child.tag == "title")
        .expect("a title");
    assert_eq!(title.text_content(), "Project layout");
    let desc = svg
        .children
        .iter()
        .find(|child| child.tag == "desc")
        .expect("a desc");
    assert_eq!(desc.text_content(), "The files");
    assert_eq!(
        svg.attr("aria-labelledby"),
        Some("chart-title-tree-view-test")
    );
    assert_eq!(
        svg.attr("aria-describedby"),
        Some("chart-desc-tree-view-test")
    );
    assert_eq!(svg.attr("aria-roledescription"), Some("treeView"));
}

// ---------------------------------------------------------------------------------------------------
// The icons
// ---------------------------------------------------------------------------------------------------

/// An icon that ferrigura does not have, from a pack it does not register, is the unknown icon: the
/// reference draws the same for a pack it cannot load (R9, `rendering-util/icons.ts` 20-24).
#[test]
fn an_icon_that_does_not_resolve_is_the_unknown_icon() {
    let svg = svg_of("treeView-beta\n  a icon(fa:bell)");
    let icon = row_named(&svg, "a").icon_group.expect("an icon");
    assert_eq!(
        icon.attr("transform"),
        Some("translate(20, 35)"),
        "15 indent, 5 padding; 30 down"
    );
    assert_eq!(support::icon_kind(&icon), IconKind::Unknown);
    let markup = draw("treeView-beta\n  a icon(fa:bell)").expect("the tree view renders");
    assert!(markup.contains(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 80 80">"#
    ));
}

/// The configured icon maps decide a file's icon, and a file mapped to `none` has none.
#[test]
fn a_file_mapped_to_none_has_no_icon() {
    let text = with_config(
        "      showIcons: true\n      extensionIcons:\n        .txt: none\n",
        "  notes.txt\n  other.bin\n",
    );
    let svg = svg_of(&text);
    assert_eq!(row_named(&svg, "notes.txt").icon, None);
    assert_eq!(row_named(&svg, "other.bin").icon, Some(IconKind::File));
}

// ---------------------------------------------------------------------------------------------------
// The styles (`styles.ts`) and the theme's tree view group
// ---------------------------------------------------------------------------------------------------

/// The rules name the classes of the elements, scoped to the id (`styles.ts` 29-55).
#[test]
fn the_rules_are_scoped_to_the_diagram_and_name_the_classes() {
    let svg = svg_of("treeView-beta\n  a");
    let style = svg.style();
    assert!(style.contains("#tree-view-test .treeView-node-label{font-size:16px;fill:var(--treeViewLabelColor);white-space:pre;}"));
    assert!(style.contains("#tree-view-test .treeView-node-dir{font-weight:bold;}"));
    assert!(style.contains("#tree-view-test .treeView-highlight-bg{fill:var(--treeViewHighlightBg);stroke:var(--treeViewHighlightStroke);stroke-width:1;}"));
    assert!(style.contains("--treeViewLabelColor:black;"));
}

/// The label font size sets the text box of every label, and the rule of the labels and of the
/// descriptions alike (`labelFontSize`): two rows of the text box at 20 px and their padding.
#[test]
fn the_label_font_size_sets_the_text_box() {
    let text = "%%{init: {\"themeVariables\": {\"treeView\": {\"labelFontSize\": \"20px\"}}}}%%\ntreeView-beta\n  a\n";
    let svg = svg_of(text);
    let height = svg
        .attr("viewBox")
        .expect("a viewBox")
        .split(' ')
        .nth(3)
        .expect("a height")
        .parse::<f64>()
        .expect("a number");
    assert_close(height, 2.0 * (regular_height(20.0) + 10.0));
    assert!(
        svg.style()
            .contains("#tree-view-test .treeView-node-label{font-size:20px;")
    );
}

/// The colours of `themeVariables.treeView` are the custom properties and the presentation attributes
/// of their elements (decision 0007).
#[test]
fn the_colours_of_the_theme_group_reach_the_elements() {
    let site = site_variables(&[
        ("labelColor", "#123456"),
        ("lineColor", "#654321"),
        ("iconColor", "#abcdef"),
        ("descriptionColor", "#ff0000"),
        ("highlightBg", "#00ff00"),
        ("highlightStroke", "#0000ff"),
    ]);
    let text = with_config("      showIcons: true\n", "  a :::highlight ## d\n");
    let svg = svg_with(&text, &site);
    let style = svg.style();
    assert!(style.contains("--treeViewLabelColor:#123456;"));
    assert!(style.contains("--treeViewHighlightStroke:#0000ff;"));
    let row = row_named(&svg, "a");
    assert_eq!(row.label.attr("fill"), Some("#123456"));
    assert_eq!(
        row.description.expect("a description").attr("fill"),
        Some("#ff0000")
    );
    let icon = row.icon_group.expect("an icon");
    assert_eq!(icon.attr("color"), Some("#abcdef"));
    let highlight = row.highlight.expect("a highlight");
    assert_eq!(highlight.attr("fill"), Some("#00ff00"));
    assert_eq!(highlight.attr("stroke"), Some("#0000ff"));
    let connector = &lines(&svg)[0];
    assert_eq!(connector.attr("stroke"), Some("#654321"));
}

/// A label font size that is not in pixels is an error, since the port does not measure other units.
#[test]
fn a_font_size_that_is_not_in_pixels_is_an_error() {
    let site = site_variables(&[("labelFontSize", "2em")]);
    let result = draw_configured("treeView-beta\n  a", Some(&site));
    assert!(
        matches!(result, Err(RenderError::Unsupported(_))),
        "{result:?}"
    );
}

/// The options that are not set keep their defaults, and an option merged in from the theme group is
/// the one that is read: `themeVariables.treeView` is merged over the defaults, one key at a time.
#[test]
fn an_unset_option_keeps_its_default() {
    let site = site_variables(&[("labelColor", "#123456")]);
    let svg = svg_with("treeView-beta\n  a ## d", &site);
    let row = row_named(&svg, "a");
    assert_eq!(row.label.attr("fill"), Some("#123456"));
    assert_eq!(
        row.description.expect("a description").attr("fill"),
        Some("#6a9955")
    );
}

// ---------------------------------------------------------------------------------------------------
// The classes of a node (the reference's rules match them as they are)
// ---------------------------------------------------------------------------------------------------

/// A file with the class `treeView-node-dir` takes the directory's rule, the bold weight: its label is
/// measured as the bold directory label is (`styles.ts`, `.treeView-node-dir`).
#[test]
fn a_file_with_the_directory_class_is_bold() {
    let bold = svg_of("treeView-beta\n  a :::treeView-node-dir");
    let directory = svg_of("treeView-beta\n  a/");
    assert_eq!(bold.attr("viewBox"), directory.attr("viewBox"));
    assert!(row_named(&bold, "a").label.has_class("treeView-node-dir"));
}

/// A label with the class `treeView-node-description` takes the description's rule, the italic: its
/// width is the italic face's.
#[test]
fn a_label_with_the_description_class_is_italic() {
    let italic = svg_of("treeView-beta\n  a :::treeView-node-description");
    let label = measure_box(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Italic),
        "a",
        16.0,
    )
    .expect("the bundled face measures")
    .width();
    assert_close(width_of(&italic), 15.0 + (label + 10.0));
}

// ---------------------------------------------------------------------------------------------------
// The configuration and its errors
// ---------------------------------------------------------------------------------------------------

/// The defaults of the section are the reference's (`config.schema.yaml` 2749-2808).
#[test]
fn the_section_has_the_schema_defaults() {
    let configured = effective(None, Some("treeView"), &Value::empty_object());
    let config = typed(&configured).expect("the configuration is typed");
    let section = &config.tree_view;
    assert_eq!(section.row_indent, 10.0);
    assert_eq!(section.padding_x, 5.0);
    assert_eq!(section.padding_y, 5.0);
    assert_eq!(section.line_thickness, 1.0);
    assert!(!section.show_icons);
    assert!(section.use_max_width);
}

/// A prototype name that a map does not hold reads the inherited member, which the reference fails on
/// (`icons.ts` 58-66: `icon.includes` of a function), so the render fails too.
#[test]
fn a_prototype_name_read_from_an_icon_map_fails_the_render() {
    let text = with_config("      showIcons: true\n", "  constructor\n");
    let result = draw(&text);
    assert!(matches!(result, Err(RenderError::Config(_))), "{result:?}");
}

/// An icon map value that is not text fails the render, as the reference's `qualifyIcon` fails on it.
/// The section is built here: the typed configuration reads a map value of the wrong type as the
/// schema's default before the renderer sees it (the configuration layer's rule).
#[test]
fn an_icon_map_value_that_is_not_text_fails_the_render() {
    let section = TreeViewDiagramConfig {
        show_icons: true,
        extension_icons: Some(Value::from(serde_json::json!({ ".ts": 5 }))),
        ..TreeViewDiagramConfig::default()
    };
    let result = draw_section("treeView-beta\n  index.ts", &section);
    assert!(matches!(result, Err(RenderError::Config(_))), "{result:?}");
}

/// A falsy icon map value gives the default icon: the reference's `detected` is falsy and the default
/// of `getNodeIcon` follows (`icons.ts` 90-99).
#[test]
fn a_falsy_extension_value_gives_the_default_icon() {
    let text = with_config(
        "      showIcons: true\n      extensionIcons:\n        .ts: 0\n",
        "  index.ts\n",
    );
    assert_eq!(
        row_named(&svg_of(&text), "index.ts").icon,
        Some(IconKind::File)
    );
}

/// A falsy filename value falls through to the extension, which then hides the icon.
#[test]
fn a_falsy_filename_value_falls_through_to_the_extension() {
    let text = with_config(
        "      showIcons: true\n      filenameIcons:\n        index.ts: \"\"\n      extensionIcons:\n        .ts: none\n",
        "  index.ts\n",
    );
    assert_eq!(row_named(&svg_of(&text), "index.ts").icon, None);
}

/// The text helper of the tests: a theme group of `themeVariables.treeView` as site configuration.
fn site_variables(pairs: &[(&str, &str)]) -> Value {
    let group: serde_json::Map<String, serde_json::Value> = pairs
        .iter()
        .map(|(name, value)| {
            (
                (*name).to_owned(),
                serde_json::Value::String((*value).to_owned()),
            )
        })
        .collect();
    let variables = serde_json::json!({ "treeView": group });
    Value::from(serde_json::json!({ "themeVariables": variables }))
}

/// The width of the root of a render: the third number of the viewBox.
fn width_of(svg: &support::Element) -> f64 {
    svg.attr("viewBox")
        .expect("a viewBox")
        .split(' ')
        .nth(2)
        .expect("a width")
        .parse()
        .expect("a number")
}

/// Two numbers that are the same up to the rounding of the arithmetic.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}
