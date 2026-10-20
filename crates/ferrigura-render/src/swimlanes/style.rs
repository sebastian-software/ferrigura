//! The style sheet of a swimlane diagram: the flowchart's, and the rules of its lanes. The reference
//! makes it the flowchart's style sheet with the swimlane rules appended (`swimlanes/styles.ts`), and the
//! lane rules of the flowchart's palette (`genColor` in `flowchart/styles.ts`, the `Lane` sections).
//!
//! The flowchart's palette rules of this port leave the lanes out (`:not(.swimlane)`), so the lanes' palette
//! rules come here, in one block after the flowchart's rules. No rule of either block matches an element
//! of the other, so the order of the two does not change a render.

use ferrigura_model::flowchart::FlowClass;
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::flowchart::style::{rule, style_sheet_with_fonts};
use crate::svg::css_text;
use crate::unified::palette::{Palette, palette};

/// The style sheet of a swimlane diagram: the flowchart's, then the lanes' rules.
#[must_use]
pub fn style_sheet(
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    classes: &[FlowClass],
    families: &[EmbeddedFamily],
) -> String {
    let mut css = style_sheet_with_fonts(id, theme, globals, classes, families);
    css.push_str(&css_text(&lane_rules(id, theme, globals)));
    css
}

/// The lanes' rules: the border of a lane that has no palette slot, the lanes' `neo` filter, and the palette
/// of the lanes (`swimlanes/styles.ts`, `flowchart/styles.ts` 49-127).
fn lane_rules(id: &str, theme: ThemeView<'_>, globals: &Globals) -> String {
    let border = theme
        .light
        .cluster_border
        .text()
        .unwrap_or_else(|| "undefined".to_owned());
    // The lane's own border is `!important`, which outranks the generic cluster rule. A lane with a palette
    // slot is exempt, as the palette's rules would lose to it (`swimlanes/styles.ts`).
    let mut css = rule(
        &format!("#{id} .swimlane.cluster:not([data-color-id]) rect"),
        &format!("stroke: {border} !important;"),
    );
    css.push_str(&rule(
        &format!("#{id} [data-look=\"neo\"].cluster rect"),
        "filter: none;",
    ));
    if let Some(palette) = palette(globals, theme.light) {
        css.push_str(&lane_palette(id, safe_look(&globals.look), &palette));
    }
    css
}

/// The palette's rules for the lanes: each slot of the palette paints both halves of a lane, its outline,
/// and, when the palette has background colours, the lane's fill (`genColor`'s lane sections). A lane is two
/// rectangles, so the rules name both halves. A rule carries no `!important`, so a user's inline style on the
/// lane still wins.
fn lane_palette(id: &str, look: &str, palette: &Palette) -> String {
    let mut css = String::new();
    for index in 0..palette.border.len() {
        let border = &palette.border[index];
        let slot = format!("#{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"]");
        let fill = if palette.background.is_empty() {
            String::new()
        } else {
            let background = &palette.background[index % palette.background.len()];
            format!(" fill: {background};")
        };
        css.push_str(&rule(
            &format!(
                "{slot}.swimlane.cluster rect.swimlane-title, {slot}.swimlane.cluster rect.swimlane-body"
            ),
            &format!("stroke: {border};{fill}"),
        ));
        // roughjs draws a lane's hachure fill first and its outline second, so the outline is the second
        // path, and the fill (drawn as lines) is the first (`genColor`).
        css.push_str(&rule(
            &lane_path_selector(&slot, "path:nth-of-type(2)"),
            &format!("stroke: {border};"),
        ));
        if !palette.background.is_empty() {
            let background = &palette.background[index % palette.background.len()];
            css.push_str(&rule(
                &lane_path_selector(&slot, "path:first-of-type"),
                &format!("stroke: {background};"),
            ));
        }
    }
    css
}

/// The selector of both halves of a lane, down to `suffix` (`laneRule` in `genColor`).
fn lane_path_selector(slot: &str, suffix: &str) -> String {
    format!(
        "{slot}.swimlane.cluster .swimlane-title {suffix}, {slot}.swimlane.cluster .swimlane-body {suffix}"
    )
}

/// The look a stylesheet is written for: a bare word, else `classic` (`safeLook` in `colorThemeGate.ts`). The
/// flowchart's stylesheet has the same rule (`flowchart/style.rs`), which is private to it.
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}
