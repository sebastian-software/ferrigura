//! `redux-dark-color`, the dark theme: `packages/mermaid/src/themes/theme-redux-dark-color.js`.
//!
//! The same shape as [`super::redux_color`], with other values. Several of its derived values are
//! computed in the constructor, before the overrides are copied in, and several are assigned
//! whatever they held, as the reference does. The theme never sets `darkMode` itself, so the
//! `darkMode` branches take the light path unless an override sets it.

use crate::value::Value;

use super::Res;
use super::helpers::{
    Vars, adjust, calculate, cycle, darken, first_truthy, invert, is_dark, lighten, mk_border,
    scale_indices, set_xy_chart, text, text_array, to_number,
};
use super::khroma::{self, Channel};

/// The Fuchsia, Teal and other 300 shades of the categorical scale, `cScale0` to `cScale11`.
const SCALE: [&str; 12] = [
    "#f4a8ff", "#46ecd5", "#ffb86a", "#dab2ff", "#7bf1a8", "#c4b4ff", "#ffa2a2", "#ffdf20",
    "#a3b3ff", "#bbf451", "#74d4ff", "#ffa1ad",
];

/// The Violet and other 400 shades of the border palette, `borderColorArray`.
const BORDER_COLORS: [&str; 12] = [
    "#E879F9", "#2DD4BF", "#FB923C", "#22D3EE", "#4ADE80", "#A78BFA", "#F87171", "#FACC15",
    "#818CF8", "#A3E635 ", "#38BDF8", "#FB7185",
];

/// The Fuchsia to Violet shades of 900, the fills of the user journey sections, `fillType0` to
/// `fillType7`. The background palette of the light theme is empty in this one.
const JOURNEY_FILLS: [&str; 8] = [
    "#701a75", "#134e4a", "#7c2d12", "#581c87", "#14532d", "#4c1d95", "#7f1d1d", "#713f12",
];

/// The variables of the dark theme, with the user's overrides applied.
pub(crate) fn variables(overrides: Option<&Value>) -> Res<Value> {
    let mut vars = constructor()?;
    calculate(&mut vars, overrides, update_colors)?;
    Ok(Value::Object(vars.into_properties()))
}

/// The constructor of the reference's class: the base values, several of them derived from the
/// base colours already, with `darkMode` unset.
fn constructor() -> Res<Vars> {
    let mut vars = Vars::default();

    /* Base variables */
    vars.text("background", "#333");
    vars.text("primaryColor", "#1f2020");
    let secondary = lighten(&vars.get("primaryColor"), 16.0)?;
    vars.set("secondaryColor", secondary);
    let tertiary = adjust(&vars.get("primaryColor"), &[(Channel::H, -160.0)])?;
    vars.set("tertiaryColor", tertiary);
    let primary_border = invert(&vars.get("background"))?;
    vars.set("primaryBorderColor", primary_border);
    let dark = vars.truthy("darkMode");
    let secondary_border = mk_border(&vars.get("secondaryColor"), dark)?;
    vars.set("secondaryBorderColor", secondary_border);
    let tertiary_border = mk_border(&vars.get("tertiaryColor"), dark)?;
    vars.set("tertiaryBorderColor", tertiary_border);
    let primary_text = invert(&vars.get("primaryColor"))?;
    vars.set("primaryTextColor", primary_text);
    let secondary_text = invert(&vars.get("secondaryColor"))?;
    vars.set("secondaryTextColor", secondary_text);
    let tertiary_text = invert(&vars.get("tertiaryColor"))?;
    vars.set("tertiaryTextColor", tertiary_text);

    vars.text("mainBkg", "#111113");
    // Resolved in updateColors(), from the main background.
    vars.text("secondBkg", "calculated");
    vars.text("mainContrastColor", "lightgrey");
    let inverted = khroma::invert("#323D47", 100.0)?;
    vars.text("darkTextColor", &khroma::lighten(&inverted, 10.0)?);
    vars.text("border1", "#ccc");
    vars.text("border2", &khroma::rgba(255.0, 255.0, 255.0, 0.25));
    let arrowhead = invert(&vars.get("background"))?;
    vars.set("arrowheadColor", arrowhead);
    vars.text("fontFamily", "\"Recursive Variable\", arial, sans-serif");
    vars.text("fontSize", "14px");
    vars.text("labelBackground", "#111113");
    vars.text("textColor", "#ccc");
    vars.number("THEME_COLOR_LIMIT", 12.0);
    vars.number("radius", 12.0);
    vars.number("strokeWidth", 2.0);

    vars.nullish_text("noteBkgColor", "#FEF9C3");
    vars.nullish_text("noteTextColor", "#28253D");

    vars.number("THEME_COLOR_LIMIT", 12.0);
    // dark
    vars.text("fontFamily", "\"Recursive Variable\", arial, sans-serif");
    vars.text("fontSize", "14px");

    // Neo-specific
    vars.text("nodeBorder", "#FFFFFF");
    vars.text("stateBorder", "#FFFFFF");

    vars.flag("useGradient", false);
    vars.text("gradientStart", "#0042eb");
    vars.text("gradientStop", "#eb0042");
    vars.text("dropShadow", "url(#drop-shadow)");
    vars.flag("nodeShadow", true);

    /* Architecture Diagram variables */
    vars.text("archEdgeColor", "calculated");
    vars.text("archEdgeArrowColor", "calculated");
    vars.text("archEdgeWidth", "3");
    vars.set("archGroupBorderColor", vars.get("primaryBorderColor"));
    vars.text("archGroupBorderWidth", "2px");

    /* Class Diagram variables */
    vars.text("clusterBkg", "#1E1A2E");
    vars.text("clusterBorder", "#BDBCCC");
    vars.text("noteBorderColor", "#FACC15");

    /* Sequence Diagram variables */
    vars.number("noteFontWeight", 600.0);

    vars.set("borderColorArray", text_array(&BORDER_COLORS));
    vars.set("bkgColorArray", text_array(&[]));

    /* Usecase Diagram variables: borders only, the fills fall back to the rest of the theme. */
    vars.text("usecaseActorBorder", "#A78BFA");
    vars.text("usecaseBorder", "#2DD4BF");
    vars.text("usecaseBoundaryBorder", "#BDBCCC");
    vars.text("usecaseIncludeLine", "#38BDF8");
    vars.text("usecaseExtendLine", "#FB923C");

    vars.text("filterColor", "#FFFFFF");
    Ok(vars)
}

/// `updateColors` of the reference: every derived value, each of them only when it is not set
/// already, except the ones the reference assigns outright.
fn update_colors(t: &mut Vars) -> Res<()> {
    // The || is to make sure that if the variable has been defined by a user override that value
    // is to be used.

    /* Main */
    t.or("primaryTextColor", |t| {
        Ok(text(if t.truthy("darkMode") {
            "#eee"
        } else {
            "#FFFFFF"
        }))
    })?;
    t.or("secondaryColor", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, -120.0)])
    })?;
    t.or("tertiaryColor", |t| {
        adjust(
            &t.get("primaryColor"),
            &[(Channel::H, 180.0), (Channel::L, 5.0)],
        )
    })?;

    t.or("primaryBorderColor", |t| {
        mk_border(&t.get("primaryColor"), t.truthy("darkMode"))
    })?;
    t.or("secondaryBorderColor", |t| {
        mk_border(&t.get("secondaryColor"), t.truthy("darkMode"))
    })?;
    t.or("tertiaryBorderColor", |t| {
        mk_border(&t.get("tertiaryColor"), t.truthy("darkMode"))
    })?;
    t.or("noteBorderColor", |t| {
        mk_border(&t.get("noteBkgColor"), t.truthy("darkMode"))
    })?;
    t.or_text("noteBkgColor", "#fff5ad");
    t.or_text("noteTextColor", "#FFFFFF");

    t.or("secondaryTextColor", |t| invert(&t.get("secondaryColor")))?;
    t.or("tertiaryTextColor", |t| invert(&t.get("tertiaryColor")))?;
    t.or("lineColor", |t| invert(&t.get("background")))?;
    t.or("arrowheadColor", |t| invert(&t.get("background")))?;
    t.or_key("textColor", "primaryTextColor");

    // TODO: should this instead default to secondaryBorderColor?
    t.or_key("border2", "tertiaryBorderColor");

    /* Flowchart variables */
    t.or_key("nodeBkg", "primaryColor");
    t.or_key("mainBkg", "primaryColor");
    // Resolve the constructor's 'calculated' placeholder, which this theme inherited from
    // theme-dark.js without the line that computes it.
    if t.get("secondBkg") == Some(Value::String("calculated".to_owned())) {
        let shade = lighten(&t.get("mainBkg"), 16.0)?;
        t.set("secondBkg", shade);
    }
    t.or_key("nodeBorder", "border1");
    t.or_key("clusterBkg", "tertiaryColor");
    t.or_key("clusterBorder", "tertiaryBorderColor");
    t.or_key("defaultLinkColor", "lineColor");
    t.or_key("titleColor", "tertiaryTextColor");
    t.or("edgeLabelBackground", |t| {
        if t.truthy("darkMode") {
            darken(&t.get("secondaryColor"), 30.0)
        } else {
            Ok(t.get("secondaryColor"))
        }
    })?;
    t.or_key("nodeTextColor", "primaryTextColor");
    /* Agentflow variables */
    t.or_key("flowContainerStroke", "secondaryBorderColor");

    /* Sequence Diagram variables */
    t.text("actorBorder", "#FFFFFF");
    t.text("signalColor", "#FFFFFF");
    t.text("labelBoxBorderColor", "#BDBCCC");
    t.or_key("actorBorder", "primaryBorderColor");
    t.or_key("actorBkg", "mainBkg");
    t.or_key("actorTextColor", "primaryTextColor");
    t.or_key("actorLineColor", "actorBorder");
    t.or_key("labelBoxBkgColor", "actorBkg");
    t.or_key("signalColor", "textColor");
    t.or_key("signalTextColor", "textColor");
    t.or_key("labelBoxBorderColor", "actorBorder");
    t.or_key("labelTextColor", "actorTextColor");
    t.or_key("loopTextColor", "actorTextColor");
    t.or("activationBorderColor", |t| {
        darken(&t.get("secondaryColor"), 10.0)
    })?;
    t.or_key("activationBkgColor", "secondaryColor");
    t.or("sequenceNumberColor", |t| invert(&t.get("lineColor")))?;
    t.or_key("rectBkgColor", "tertiaryColor");

    /* Mindmap Diagram variables */
    t.text("rootLabelColor", "#FFFFFF");

    /* Gantt chart variables */
    t.or_text("excludeBkgColor", "#eeeeee");
    t.or_key("taskBorderColor", "primaryBorderColor");
    t.or_key("taskBkgColor", "primaryColor");
    t.or_key("activeTaskBorderColor", "primaryColor");
    t.or("activeTaskBkgColor", |t| {
        lighten(&t.get("primaryColor"), 23.0)
    })?;
    t.or_text("gridColor", "lightgrey");
    // `doneTaskBkgColor` was 'lightgrey' in the light theme: a light fill under this theme's light
    // `taskTextDarkColor`. The active-task fill is dark here, so the fill has to move.
    t.or_key("doneTaskBkgColor", "secondBkg");
    t.or_text("doneTaskBorderColor", "grey");
    t.or_text("critBorderColor", "#ff8888");
    t.or_text("critBkgColor", "red");
    t.or_text("todayLineColor", "red");
    t.or_key("taskTextColor", "textColor");
    t.or_key("vertLineColor", "primaryBorderColor");
    t.or_key("taskTextOutsideColor", "textColor");
    t.or_key("taskTextLightColor", "textColor");
    t.or_key("taskTextColor", "primaryTextColor");
    t.or_key("taskTextDarkColor", "textColor");
    t.or_text("taskTextClickableColor", "#003163");

    /* Architecture Diagram variables */
    let line = t.get("lineColor");
    t.set("archEdgeColor", line.clone());
    t.set("archEdgeArrowColor", line);

    /* Sequence Diagram variables */
    t.or_key("personBorder", "primaryBorderColor");
    t.or_key("personBkg", "mainBkg");

    /* state colors */
    t.or_key("transitionColor", "lineColor");
    t.or_key("transitionLabelColor", "textColor");
    /* The color of the text tables of the states*/
    t.or("stateLabelColor", |t| {
        Ok(first_truthy(t, &["stateBkg", "primaryTextColor"]))
    })?;
    t.or_text("stateEdgeLabelBackground", "#16141F");
    t.or_key("stateBkg", "mainBkg");
    t.or_key("labelBackgroundColor", "stateBkg");
    t.or_text("compositeBackground", "#16141F");
    t.or_text("altBackground", "#16141F");
    t.or_text("compositeTitleBackground", "#16141F");
    t.or_key("compositeBorder", "nodeBorder");
    t.set("innerEndBackground", t.get("nodeBorder"));
    t.or_key("errorBkgColor", "tertiaryColor");
    t.or_key("errorTextColor", "tertiaryTextColor");
    t.or_key("transitionColor", "lineColor");
    t.set("specialStateColor", t.get("lineColor"));

    /* Color Scale */
    /* Each color-set will have a background, a foreground and a border color */
    for (index, colour) in SCALE.iter().enumerate() {
        t.or_text(&format!("cScale{index}"), colour);
    }

    /* Gantt chart section banding. altSectionBkgColor is the canvas colour, not the inherited
     * 'white': at 20% over the #333 canvas white composites to a grey brighter than both tuned
     * hues. The canvas colour makes those bands read as absent instead. */
    t.or_key("sectionBkgColor", "cScale0");
    t.or_key("sectionBkgColor2", "cScale1");
    t.or_key("altSectionBkgColor", "background");

    let limit = to_number(&t.get("THEME_COLOR_LIMIT"));

    // Setup the inverted color for the set
    for index in scale_indices(limit) {
        let scale = format!("cScale{index}");
        t.or(&format!("cScaleInv{index}"), |t| invert(&t.get(&scale)))?;
    }

    // Setup the peer color for the set, useful for borders
    for index in scale_indices(limit) {
        let scale = format!("cScale{index}");
        let peer = format!("cScalePeer{index}");
        if t.truthy("darkMode") {
            t.or(&peer, |t| lighten(&t.get(&scale), 10.0))?;
        } else {
            t.or(&peer, |t| darken(&t.get(&scale), 10.0))?;
        }
    }

    // Setup the label color for the set
    t.or_key("scaleLabelColor", "labelTextColor");

    // The label of each colour is darkened outright; the reference does not keep a set one.
    for index in scale_indices(limit) {
        let scale = format!("cScale{index}");
        let label = darken(&t.get(&scale), 75.0)?;
        t.set(&format!("cScaleLabel{index}"), label);
    }

    let multiplier = if t.truthy("darkMode") { -4.0 } else { -1.0 };
    for index in 0..5_usize {
        let offset = index as f64 * 3.0;
        t.or(&format!("surface{index}"), |t| {
            adjust(
                &t.get("mainBkg"),
                &[
                    (Channel::H, 180.0),
                    (Channel::S, -15.0),
                    (Channel::L, multiplier * (5.0 + offset)),
                ],
            )
        })?;
        t.or(&format!("surfacePeer{index}"), |t| {
            adjust(
                &t.get("mainBkg"),
                &[
                    (Channel::H, 180.0),
                    (Channel::S, -15.0),
                    (Channel::L, multiplier * (8.0 + offset)),
                ],
            )
        })?;
    }

    /* class */
    t.or_key("classText", "textColor");

    /* user-journey */
    // Journey paints the section band and the task rects inside it with the same `fillTypeN`, and
    // task labels use the light `textColor`, so these fills have to stay dark.
    for (index, fill) in JOURNEY_FILLS.iter().enumerate() {
        t.or_text(&format!("fillType{index}"), fill);
    }

    /* pie */
    // Slices reuse the theme's categorical scale so a pie reads like a mindmap or treemap in the
    // same theme.
    for index in scale_indices(limit) {
        t.or_key(&format!("pie{}", index + 1), &format!("cScale{index}"));
    }
    t.or_text("pieTitleTextSize", "25px");
    t.or_key("pieTitleTextColor", "taskTextDarkColor");
    t.or_text("pieSectionTextSize", "17px");
    // Slice fills are now the bright categorical scale, so the in-slice label needs a dark ink.
    t.or_text("pieSectionTextColor", "#28253D");
    t.or_text("pieLegendTextSize", "17px");
    t.or_key("pieLegendTextColor", "taskTextDarkColor");
    t.or_text("pieStrokeColor", "black");
    t.or_text("pieStrokeWidth", "2px");
    t.or_text("pieOuterStrokeWidth", "2px");
    t.or_text("pieOuterStrokeColor", "black");
    t.or_text("pieOpacity", "0.7");

    /* venn */
    /* `borderColorArray` and not the lighter `cScale`: the fill is painted at 0.1 opacity, so the
     * stroke and the label carry the circle. */
    for index in 0..8_usize {
        t.nullish(&format!("venn{}", index + 1), |t| {
            cycle(&t.get("borderColorArray"), index)
        })?;
    }
    t.nullish_key("vennTitleTextColor", "titleColor");
    t.nullish_key("vennSetTextColor", "textColor");

    /* quadrant-graph */
    t.or_key("quadrant1Fill", "primaryColor");
    t.or("quadrant2Fill", |t| {
        adjust(
            &t.get("primaryColor"),
            &[(Channel::R, 5.0), (Channel::G, 5.0), (Channel::B, 5.0)],
        )
    })?;
    t.or("quadrant3Fill", |t| {
        adjust(
            &t.get("primaryColor"),
            &[(Channel::R, 10.0), (Channel::G, 10.0), (Channel::B, 10.0)],
        )
    })?;
    t.or("quadrant4Fill", |t| {
        adjust(
            &t.get("primaryColor"),
            &[(Channel::R, 15.0), (Channel::G, 15.0), (Channel::B, 15.0)],
        )
    })?;
    t.or_key("quadrant1TextFill", "primaryTextColor");
    t.or("quadrant2TextFill", |t| {
        adjust(
            &t.get("primaryTextColor"),
            &[(Channel::R, -5.0), (Channel::G, -5.0), (Channel::B, -5.0)],
        )
    })?;
    t.or("quadrant3TextFill", |t| {
        adjust(
            &t.get("primaryTextColor"),
            &[
                (Channel::R, -10.0),
                (Channel::G, -10.0),
                (Channel::B, -10.0),
            ],
        )
    })?;
    t.or("quadrant4TextFill", |t| {
        adjust(
            &t.get("primaryTextColor"),
            &[
                (Channel::R, -15.0),
                (Channel::G, -15.0),
                (Channel::B, -15.0),
            ],
        )
    })?;
    // `this.quadrantPointFill || isDark(...) ? lighten(...) : darken(...)` is assigned whatever the
    // property held. The reference calls `lighten` and `darken` with no amount, so the lightness
    // is NaN, and the override, applied afterwards, is what the property ends up as.
    let quadrant1 = t.get("quadrant1Fill");
    let point = if t.truthy("quadrantPointFill") || is_dark(&quadrant1)? {
        lighten(&quadrant1, f64::NAN)?
    } else {
        darken(&quadrant1, f64::NAN)?
    };
    t.set("quadrantPointFill", point);
    t.or_key("quadrantPointTextFill", "primaryTextColor");
    t.or_key("quadrantXAxisTextFill", "primaryTextColor");
    t.or_key("quadrantYAxisTextFill", "primaryTextColor");
    t.or_key("quadrantInternalBorderStrokeFill", "primaryBorderColor");
    t.or_key("quadrantExternalBorderStrokeFill", "primaryBorderColor");
    t.or_key("quadrantTitleFill", "primaryTextColor");

    /* xychart */
    set_xy_chart(t);

    /* requirement-diagram */
    t.or_key("requirementBackground", "primaryColor");
    t.or_key("requirementBorderColor", "primaryBorderColor");
    t.or_text("requirementBorderSize", "1");
    t.or_key("requirementTextColor", "primaryTextColor");
    t.or_key("relationColor", "lineColor");
    t.or("relationLabelBackground", |t| {
        if t.truthy("darkMode") {
            darken(&t.get("secondaryColor"), 30.0)
        } else {
            Ok(t.get("secondaryColor"))
        }
    })?;
    t.or_key("relationLabelColor", "actorTextColor");
    t.text("requirementEdgeLabelBackground", "#16141F");

    /* git */
    t.or_key("git0", "primaryColor");
    t.or_key("git1", "secondaryColor");
    t.or_key("git2", "tertiaryColor");
    t.or("git3", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, -30.0)])
    })?;
    t.or("git4", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, -60.0)])
    })?;
    t.or("git5", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, -90.0)])
    })?;
    t.or("git6", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, 60.0)])
    })?;
    t.or("git7", |t| {
        adjust(&t.get("primaryColor"), &[(Channel::H, 120.0)])
    })?;
    for index in 0..8_usize {
        let key = format!("git{index}");
        let current = t.get(&key);
        let shade = if t.truthy("darkMode") {
            lighten(&current, 25.0)?
        } else {
            darken(&current, 25.0)?
        };
        t.set(&key, shade);
    }
    for index in 0..8_usize {
        t.or(&format!("gitInv{index}"), |t| {
            invert(&t.get(&format!("git{index}")))
        })?;
    }
    t.or("branchLabelColor", |t| {
        Ok(if t.truthy("darkMode") {
            text("black")
        } else {
            t.get("labelTextColor")
        })
    })?;
    for index in 0..8_usize {
        t.or_key(&format!("gitBranchLabel{index}"), "branchLabelColor");
    }

    t.or_key("tagLabelColor", "primaryTextColor");
    t.or_key("tagLabelBackground", "primaryColor");
    // The reference assigns this from `tagBorder`, not from `tagLabelBorder` itself.
    let tag_border = first_truthy(t, &["tagBorder", "primaryBorderColor"]);
    t.set("tagLabelBorder", tag_border);
    t.or_text("tagLabelFontSize", "10px");
    t.or_key("commitLabelColor", "secondaryTextColor");
    t.or_key("commitLabelBackground", "secondaryColor");
    t.nullish_text("commitLineColor", "#BDBCCC");
    t.or_text("commitLabelFontSize", "10px");
    t.number("fontWeight", 600.0);

    /* EntityRelationship diagrams */
    t.text("erEdgeLabelBackground", "#16141F");

    Ok(())
}
