//! The rules of a timeline: `styles.js` (`genReduxSections` and `getStyles`) for the redux themes,
//! with every colour a custom property on the diagram's id (`docs-port/decisions/0007`).
//!
//! The rules that match no element of a timeline are left out, each for its reason: `.edge` (a
//! timeline has no edges), `.node-icon-N` (no icons), `.section-edge-N` and `.edge-depth-N` (no
//! element carries them), `.section-N line` (a redux node has no line, the line is classic only),
//! `.disabled*` (no element is disabled), `.section-root` and `.icon-container` (no git root and no
//! icon). The `.lineWrapper line` rule is written once: the reference writes the same rule for every
//! colour, so the copies are equal.

use super::{Context, Palette};

/// The most colour slots the reference writes rules for: `colorSlotCount` (`colorThemeGate.ts`) caps
/// its loops at `MAX_COLOR_SLOTS`.
const MAX_RULE_SLOTS: usize = 64;

/// The rules the timeline adds to the base style sheet, as CSS text: the colour properties of the
/// schemes, then the section and line rules. The caller escapes the whole sheet once.
pub(super) fn own_rules(cx: &Context<'_>) -> String {
    let id = cx.svg_id;
    let slots = rule_slots(cx.limit);
    let mut css = String::new();

    // The colours: one custom property per slot and per colour, on the diagram's id, and the dark
    // scheme's values under `prefers-color-scheme: dark` when both schemes are written.
    match &cx.secondary {
        None => css.push_str(&format!("#{id}{{{}}}", tokens(&cx.primary, slots))),
        Some(dark) => css.push_str(&format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            tokens(&cx.primary, slots),
            tokens(dark, slots)
        )),
    }

    let weight = cx.font_weight.as_str();
    for slot in 0..slots {
        // `.section-(i-1)` is the class of colour `i`: the header and the tasks of section `i-1` take it.
        let class = slot as i64 - 1;
        css.push_str(&format!(
            "#{id} .section-{class} rect,#{id} .section-{class} path,#{id} .section-{class} circle{{fill:var(--sectionFill{slot});stroke:var(--sectionStroke{slot});stroke-width:2;filter:url(#{id}-drop-shadow);}}\
#{id} .section-{class} text{{fill:var(--nodeBorder);font-weight:{weight};}}"
        ));
    }
    css.push_str(&format!(
        "#{id} .lineWrapper line{{stroke:var(--nodeBorder);stroke-width:2;}}\
#{id} .eventWrapper{{filter:brightness(120%);}}"
    ));

    // The gradient of the neo look: the nodes are filled with `mainBkg` and stroked with the gradient.
    if cx.gradient {
        for slot in 0..slots {
            let class = slot as i64 - 1;
            css.push_str(&format!(
                "#{id} .section-{class}[data-look=\"neo\"] rect,#{id} .section-{class}[data-look=\"neo\"] path,#{id} .section-{class}[data-look=\"neo\"] circle{{fill:var(--mainBkg);stroke:url(#{id}-gradient);stroke-width:2;}}\
#{id} .section-{class}[data-look=\"neo\"] line{{stroke:url(#{id}-gradient);stroke-width:2;}}"
            ));
        }
    }
    css
}

/// The custom properties of one scheme: its main background, and the fill and the stroke of each slot.
fn tokens(palette: &Palette, slots: usize) -> String {
    let mut out = format!("--mainBkg:{};", palette.main_bkg);
    for slot in 0..slots {
        let colours = palette.colours(slot);
        out.push_str(&format!(
            "--sectionFill{slot}:{};--sectionStroke{slot}:{};",
            colours.fill, colours.stroke
        ));
    }
    out
}

/// The number of slots the rules are written for: `THEME_COLOR_LIMIT`, at most [`MAX_RULE_SLOTS`].
fn rule_slots(limit: usize) -> usize {
    limit.min(MAX_RULE_SLOTS)
}
