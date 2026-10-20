//! Port of `packages/mermaid/src/diagrams/swimlanes/lanePalette.spec.ts`: the lane rules of the swimlane
//! style sheet, for each colour theme's palette, and for the themes without one. The tests read the rules of
//! the style sheet, rather than its text, since the port writes the rules compactly.
//!
//! The seven tests of the placeholder spec are kept under their names: `paints_both_halves…`,
//! `keeps_the_generic…`, `exempts_palette_lanes…`, `never_marks…`, `emits_no_empty…`,
//! `emits_the_handdrawn…`, and `emits_no_lane_palette_rules_for_the_plain_themes`.

#[path = "support/swimlanes.rs"]
#[allow(dead_code)]
mod support;

use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Globals, ThemeView};
use ferrigura_render::swimlanes::style::style_sheet;
use support::{Setup, config_with};

/// The colour themes, which carry a palette (`COLOR_THEMES` in `colorThemeGate.ts`).
const COLOUR_THEMES: [&str; 2] = ["redux-color", "redux-dark-color"];

/// The themes without a palette: the reference's plain themes.
const PLAIN_THEMES: [&str; 5] = ["default", "dark", "neutral", "forest", "base"];

/// The tokens of a colour theme, from its variables.
fn colour_tokens(name: &str) -> ThemeTokens {
    let theme = match name {
        "redux-color" => Theme::ReduxColor,
        "redux-dark-color" => Theme::ReduxDarkColor,
        other => panic!("no colour theme {other}"),
    };
    ThemeTokens::from_value(&theme.variables(None))
}

/// The style sheet of a swimlane diagram with the given theme name and tokens.
fn sheet(theme: &str, tokens: &ThemeTokens) -> String {
    let globals: Globals = Globals::of(&config_with(serde_json::json!({ "theme": theme })));
    style_sheet(
        "p0",
        ThemeView {
            light: tokens,
            dark: None,
        },
        &globals,
        &[],
        &[],
    )
}

/// The style sheet of a colour theme.
fn colour_sheet(theme: &str) -> String {
    sheet(theme, &colour_tokens(theme))
}

/// The style sheet of a plain theme: its variables carry the lane colours, and no palette.
fn plain_sheet(theme: &str) -> String {
    let tokens = ThemeTokens::from_value(&Value::from(serde_json::json!({
        "clusterBkg": "#ECECFF",
        "clusterBorder": "#9370DB",
    })));
    sheet(theme, &tokens)
}

/// The rules of a style sheet: each selector list with its declarations, in order.
fn rules(css: &str) -> Vec<(String, String)> {
    css.split('}')
        .filter_map(|chunk| {
            let (selector, body) = chunk.split_once('{')?;
            Some((selector.trim().to_owned(), body.trim().to_owned()))
        })
        .collect()
}

/// The declarations of every rule whose selector list contains `needle`.
fn bodies_with(css: &str, needle: &str) -> Vec<String> {
    rules(css)
        .into_iter()
        .filter(|(selector, _)| selector.contains(needle))
        .map(|(_, body)| body)
        .collect()
}

/// Whether a rule declares `declaration` (a `property: value;` text) and its selector contains `needle`.
fn declares(css: &str, needle: &str, declaration: &str) -> bool {
    bodies_with(css, needle)
        .iter()
        .any(|body| body.contains(declaration))
}

#[test]
fn paints_both_halves_of_the_lane_in_every_palette_slot_of_every_colour_theme() {
    for theme in COLOUR_THEMES {
        let tokens = colour_tokens(theme);
        let css = colour_sheet(theme);
        let border = tokens.border_color_array.texts();
        let background = tokens.bkg_color_array.texts();
        for (slot, slot_border) in border.iter().enumerate() {
            // One rule names both halves, each with the full prefix (`laneRule` of `genColor`).
            let title =
                format!("[data-color-id=\"color-{slot}\"].swimlane.cluster rect.swimlane-title");
            let body_selector =
                format!("[data-color-id=\"color-{slot}\"].swimlane.cluster rect.swimlane-body");
            let fill = format!(
                "fill: {};",
                background
                    .get(slot % background.len().max(1))
                    .map_or("", String::as_str)
            );
            assert!(
                rules(&css).iter().any(|(selector, body)| {
                    selector.contains(&title)
                        && selector.contains(&body_selector)
                        && body.contains(&format!("stroke: {slot_border};"))
                        && (background.is_empty() || body.contains(&fill))
                }),
                "{theme}: no lane rect rule for slot {slot}"
            );
            // roughjs draws the hachure fill first, so the outline is the second path.
            assert!(
                declares(
                    &css,
                    &format!(
                        "[data-color-id=\"color-{slot}\"].swimlane.cluster .swimlane-title path:nth-of-type(2)"
                    ),
                    &format!("stroke: {slot_border};"),
                ),
                "{theme}: no lane outline rule for slot {slot}"
            );
        }
    }
}

#[test]
fn keeps_the_generic_cluster_palette_rules_away_from_lanes() {
    for theme in COLOUR_THEMES {
        let css = colour_sheet(theme);
        // Every palette rule of a cluster that is not a lane is `:not(.swimlane)`: the lanes have their own.
        let tails: Vec<String> = rules(&css)
            .into_iter()
            .filter_map(|(selector, _)| {
                let at = selector.find("].cluster")?;
                let tail = &selector[at + "].cluster".len()..];
                (selector.contains("[data-color-id="))
                    .then(|| tail.split(',').next().unwrap_or_default().trim().to_owned())
            })
            .collect();
        assert!(!tails.is_empty(), "{theme}: no cluster palette rule");
        assert!(
            tails.iter().all(|tail| tail.starts_with(":not(.swimlane)")),
            "{theme}: a palette rule reaches a lane: {tails:?}"
        );
    }
}

#[test]
fn exempts_palette_lanes_from_the_important_lane_border() {
    for theme in COLOUR_THEMES {
        let tokens = colour_tokens(theme);
        let css = colour_sheet(theme);
        let border = tokens
            .cluster_border
            .text()
            .expect("the theme gives a cluster border");
        // A lane with a palette slot is outside the `!important` border, which would outrank its palette.
        assert!(
            declares(
                &css,
                ".swimlane.cluster:not([data-color-id]) rect",
                &format!("stroke: {border} !important;"),
            ),
            "{theme}: the lane border rule is missing"
        );
        // An unscoped form would outrank every lane palette rule.
        assert!(
            !css.contains(".swimlane.cluster rect {"),
            "{theme}: the border rule is unscoped"
        );
    }
}

#[test]
fn never_marks_a_lane_palette_rule_important() {
    for theme in COLOUR_THEMES {
        let css = colour_sheet(theme);
        let palette_bodies: Vec<String> = rules(&css)
            .into_iter()
            .filter(|(selector, _)| {
                selector.contains("[data-color-id=\"color-") && selector.contains(".swimlane")
            })
            .map(|(_, body)| body)
            .collect();
        assert!(!palette_bodies.is_empty(), "{theme}: no lane palette rule");
        // A user's `style` reaches the lane inline and has to keep winning over the theme.
        assert!(
            palette_bodies
                .iter()
                .all(|body| !body.contains("!important")),
            "{theme}: a lane palette rule is !important"
        );
    }
}

#[test]
fn emits_no_empty_or_undefined_declaration() {
    for theme in COLOUR_THEMES {
        let css = colour_sheet(theme);
        let bodies = bodies_with(&css, "data-color-id=\"color-");
        assert!(!bodies.is_empty(), "{theme}: no palette rule");
        for body in bodies {
            assert!(
                !body.contains("undefined") && !body.contains("NaN"),
                "{theme}: {body}"
            );
            assert!(
                !body.contains(": ;") && !body.contains(":;"),
                "{theme}: {body}"
            );
        }
    }
}

#[test]
fn emits_the_handdrawn_title_fill_rule_only_where_a_background_palette_exists() {
    // `redux-dark-color` ships no background palette, so the rule must be absent there.
    assert!(
        !colour_tokens("redux-color")
            .bkg_color_array
            .texts()
            .is_empty()
    );
    assert!(
        colour_tokens("redux-dark-color")
            .bkg_color_array
            .texts()
            .is_empty()
    );
    assert!(colour_sheet("redux-color").contains(".swimlane-title path:first-of-type"));
    assert!(!colour_sheet("redux-dark-color").contains("path:first-of-type"));
}

#[test]
fn emits_no_lane_palette_rules_for_the_plain_themes() {
    for theme in PLAIN_THEMES {
        let css = plain_sheet(theme);
        assert!(
            !css.contains("data-color-id=\"color-"),
            "{theme}: a palette rule"
        );
        // The override still ships: nothing stamps the attribute its exemption keys on.
        assert!(
            css.contains(".swimlane.cluster:not([data-color-id]) rect"),
            "{theme}: no border rule"
        );
    }
}

#[test]
fn the_lane_title_carries_the_cluster_background_as_its_fill() {
    // The lane's own presentation attributes carry the theme's colours, which the palette rules repeat.
    let setup = Setup::new(config_with(serde_json::json!({})), None);
    let svg = setup.svg("swimlane-beta LR\n  subgraph A\n  X\n  end", "p0");
    assert!(
        svg.contains("fill=\"#F9F9FB\""),
        "the lane title's fill is the cluster background"
    );
}
