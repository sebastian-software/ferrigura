//! `redux-color`, the light theme: `packages/mermaid/src/themes/theme-redux-color.js`.
//!
//! The constructor sets the base values, `update_colors` derives the rest in the order of the
//! reference's `updateColors`, and [`calculate`] applies the user's overrides around them. The
//! comments of the reference are kept where they explain a value.

use crate::value::Value;

use super::Res;
use super::helpers::{
    Vars, adjust, calculate, cycle, darken, element, first_truthy, invert, is_dark, lighten,
    mk_border, scale_indices, set_xy_chart, text, text_array, to_number,
};
use super::khroma::Channel;

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

/// The pale 50 shades of the background palette, `bkgColorArray`.
const BACKGROUND_COLORS: [&str; 12] = [
    "#FDF4FF", "#F0FDFA", "#FFF7ED", "#ECFEFF", "#F0FDF4", "#F5F3FF", "#FEF2F2", "#FEFCE8",
    "#EEF2FF", "#F7FEE7", "#F0F9FF", "#FFF1F2",
];

/// The `primaryColor` of the gantt, quadrant and git sections: a local `const` of the reference,
/// not `this.primaryColor`.
const GANTT_PRIMARY: &str = "#ECECFE";

/// The `secondaryColor` of the git section: a local `const` of the reference.
const GANTT_SECONDARY: &str = "#E9E9F1";

/// The variables of the light theme, with the user's overrides applied.
pub(crate) fn variables(overrides: Option<&Value>) -> Res<Value> {
    let mut vars = constructor()?;
    calculate(&mut vars, overrides, update_colors)?;
    Ok(Value::Object(vars.into_properties()))
}

/// The constructor of the reference's class. The border of the primary colour is computed here
/// with `darkMode` unset, because the overrides are only copied in after the constructor runs.
fn constructor() -> Res<Vars> {
    let mut vars = Vars::default();
    vars.text("background", "#ffffff");
    vars.text("primaryColor", "#cccccc");
    vars.text("mainBkg", "#ffffff");

    vars.text("noteBkgColor", "#fff5ad");
    vars.text("noteTextColor", "#28253D");

    vars.number("THEME_COLOR_LIMIT", 12.0);
    vars.number("radius", 12.0);
    vars.number("strokeWidth", 2.0);

    let border = mk_border(&text("#28253D"), vars.truthy("darkMode"))?;
    vars.set("primaryBorderColor", border);

    vars.text("fontFamily", "\"Recursive Variable\", arial, sans-serif");
    vars.text("fontSize", "14px");

    // Neo-specific
    vars.text("nodeBorder", "#28253D");
    vars.text("stateBorder", "#28253D");
    vars.flag("useGradient", false);
    vars.text("gradientStart", "#0042eb");
    vars.text("gradientStop", "#eb0042");
    vars.text("dropShadow", "url(#drop-shadow)");
    vars.flag("nodeShadow", true);
    vars.text("tertiaryColor", "#ffffff");

    // Class diagram variables
    vars.text("clusterBkg", "#F9F9FB");
    vars.text("clusterBorder", "#BDBCCC");

    // Architecture diagram variables
    vars.text("archEdgeColor", "calculated");
    vars.text("archEdgeArrowColor", "calculated");
    vars.text("archEdgeWidth", "3");
    vars.set("archGroupBorderColor", vars.get("primaryBorderColor"));
    vars.text("archGroupBorderWidth", "2px");

    // Sequence diagram variables
    vars.text("actorBorder", "#28253D");
    vars.text("noteBorderColor", "#FACC15");
    vars.number("noteFontWeight", 600.0);

    vars.set("borderColorArray", text_array(&BORDER_COLORS));
    vars.set("bkgColorArray", text_array(&BACKGROUND_COLORS));

    // Usecase diagram variables: one colour per kind of element.
    vars.text("usecaseActorBorder", "#A78BFA");
    vars.text("usecaseActorBkg", "#F5F3FF");
    vars.text("usecaseBorder", "#2DD4BF");
    vars.text("usecaseBkg", "#F0FDFA");
    vars.text("usecaseBoundaryBorder", "#BDBCCC");
    vars.text("usecaseBoundaryBkg", "#FAFAFC");
    vars.text("usecaseIncludeLine", "#38BDF8");
    vars.text("usecaseExtendLine", "#FB923C");

    vars.text("filterColor", "#000000");
    Ok(vars)
}

/// `updateColors` of the reference: every derived value, each of them only when it is not set
/// already, which is how the user's values come through.
fn update_colors(t: &mut Vars) -> Res<()> {
    // The || is to make sure that if the variable has been defined by a user override that value
    // is to be used.

    /* Main */
    t.or("primaryTextColor", |t| {
        Ok(text(if t.truthy("darkMode") {
            "#eee"
        } else {
            "#28253D"
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
    t.or_text("noteTextColor", "#28253D");

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
    t.or_key("nodeBorder", "primaryBorderColor");
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
    // Not tertiaryColor here. This theme pins tertiaryColor to its background, so deriving the
    // `rect` section band from it draws white on white -- present in the DOM, invisible on screen.
    // Keying it to the background instead keeps the band a shade of whatever the background is,
    // including when the background is overridden through themeVariables. Direction-aware because
    // darken() is a no-op at pure black: an override to #000000 needs the shade to go the other
    // way or the band vanishes exactly as it did on white.
    t.or("rectBkgColor", |t| {
        if is_dark(&t.get("background"))? {
            lighten(&t.get("background"), 4.0)
        } else {
            darken(&t.get("background"), 4.0)
        }
    })?;

    /* Gantt chart variables */
    let gantt_primary = text(GANTT_PRIMARY);
    let gantt_secondary = text(GANTT_SECONDARY);
    let gantt_tertiary = adjust(&gantt_primary, &[(Channel::H, 180.0), (Channel::L, 5.0)])?;
    t.or_text("excludeBkgColor", "#eeeeee");
    t.or_key("taskBorderColor", "primaryBorderColor");
    t.or("taskBkgColor", |_| Ok(gantt_primary.clone()))?;
    t.or("activeTaskBorderColor", |_| Ok(gantt_primary.clone()))?;
    t.or("activeTaskBkgColor", |_| lighten(&gantt_primary, 23.0))?;
    t.or_text("gridColor", "lightgrey");
    t.or_text("doneTaskBkgColor", "lightgrey");
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
    t.or_text("stateEdgeLabelBackground", "#FFFFFF");
    t.or_key("stateBkg", "mainBkg");
    t.or_key("labelBackgroundColor", "stateBkg");
    t.or("compositeBackground", |t| {
        Ok(first_truthy(t, &["background", "tertiaryColor"]))
    })?;
    t.or_text("altBackground", "#F9F9FB");
    t.or_text("compositeTitleBackground", "#F9F9FB");
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

    /* Gantt chart section banding. Assigned here rather than in the gantt block above so the two
     * visible bands come off the categorical scale and follow it if the palette changes.
     *
     * altSectionBkgColor keeps the base theme's 'white' verbatim rather than being restated as
     * `this.background`. It is the same colour on this canvas. The dark theme is where 'white' is
     * wrong and has to change. */
    t.or_key("sectionBkgColor", "cScale0");
    t.or_key("sectionBkgColor2", "cScale1");
    t.or_text("altSectionBkgColor", "white");

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

    for index in scale_indices(limit) {
        t.or_key(&format!("cScaleLabel{index}"), "scaleLabelColor");
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
    // Journey task labels resolve to the theme's `textColor` (#28253D here), so these fills have to
    // stay light. The pale background array keeps the fills hue-distinct where the old tints of
    // `primaryColor` were eight shades of the same lavender.
    for index in 0..8_usize {
        t.or(&format!("fillType{index}"), |t| {
            element(&t.get("bkgColorArray"), index as f64)
        })?;
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
    t.or_key("pieSectionTextColor", "textColor");
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
    t.or("quadrant1Fill", |_| Ok(gantt_primary.clone()))?;
    t.or("quadrant2Fill", |_| {
        adjust(
            &gantt_primary,
            &[(Channel::R, 5.0), (Channel::G, 5.0), (Channel::B, 5.0)],
        )
    })?;
    t.or("quadrant3Fill", |_| {
        adjust(
            &gantt_primary,
            &[(Channel::R, 10.0), (Channel::G, 10.0), (Channel::B, 10.0)],
        )
    })?;
    t.or("quadrant4Fill", |_| {
        adjust(
            &gantt_primary,
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
    t.or("requirementBackground", |_| Ok(gantt_primary.clone()))?;
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
    t.text("requirementEdgeLabelBackground", "#FFFFFF");

    /* git */
    t.or("git0", |_| Ok(gantt_primary.clone()))?;
    t.or("git1", |_| Ok(gantt_secondary.clone()))?;
    t.or("git2", |_| Ok(gantt_tertiary.clone()))?;
    t.or("git3", |_| adjust(&gantt_primary, &[(Channel::H, -30.0)]))?;
    t.or("git4", |_| adjust(&gantt_primary, &[(Channel::H, -60.0)]))?;
    t.or("git5", |_| adjust(&gantt_primary, &[(Channel::H, -90.0)]))?;
    t.or("git6", |_| adjust(&gantt_primary, &[(Channel::H, 60.0)]))?;
    t.or("git7", |_| adjust(&gantt_primary, &[(Channel::H, 120.0)]))?;
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
    t.or_text("commitLabelFontSize", "10px");
    t.nullish_text("commitLineColor", "#BDBCCC");
    t.number("fontWeight", 600.0);

    /* EntityRelationship diagrams */
    t.text("erEdgeLabelBackground", "#FFFFFF");

    Ok(())
}
