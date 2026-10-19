//! The style sheet of a mindmap: `genSections` (`styles.ts` 5-74) and `getStyles` (92-145) for the redux
//! themes, which are the only ones that ship. Every selector starts with the diagram's id (decision
//! 0007), and the colours are the theme's `cScale<i>`, `cScaleLabel<i>`, `cScaleInv<i>`, `git0`,
//! `gitBranchLabel0`, `nodeBorder`, `mainBkg` and `strokeWidth`, read by name.
//!
//! The rules that match no element of a mindmap are left out, each for its reason: `.node-icon-N` (the
//! icon is never drawn, `mindmap/NOTES.md` 8, question 3), `.section-N line` (no line in a mindmap),
//! `.icon-container` (no icon), `.mindmap-node-label` (no element has the class: the labels take
//! `markdown-node-label`), `.edge` and `.edge`'s fill for the edge elements only where they apply.
//! The gradient (`genGradient`) is inactive: the redux themes have `useGradient` false
//! (`theme-redux-color.js:32`).

use ferrigura_model::theme::{ThemeTokens, Token};

/// `THEME_COLOR_LIMIT` (`theme-base.js` 18): twelve slots, the section `-1` (the root) through `10`.
const COLOUR_LIMIT: usize = 12;

/// The rules of the mindmap, to append to the base style sheet of the diagram.
#[must_use]
pub fn own_rules(id: &str, tokens: &ThemeTokens, look: &str) -> String {
    let neo = look == "neo";
    let text = |token: &Token| token.text().unwrap_or_default();
    let node_border = text(&tokens.node_border);
    let main_bkg = text(&tokens.main_bkg);
    let stroke_width = tokens.stroke_width.text().unwrap_or_else(|| "2".to_owned());
    let drop_shadow = tokens.drop_shadow.text().map_or_else(
        || "none".to_owned(),
        |shadow| shadow.replace("url(#drop-shadow)", &format!("url(#{id}-drop-shadow)")),
    );

    let mut css = String::new();
    // `.edge` (styles.ts 104-106, 123-125): the stroke width and no fill.
    css.push_str(&format!("#{id} .edge{{stroke-width:3;fill:none;}}"));

    // `genSections` (styles.ts 19-72): the rules of section `i - 1` for each colour slot `i`.
    for index in 0..COLOUR_LIMIT {
        let section = index as i64 - 1;
        let fill = text(tokens.c_scale.get(index));
        let label = text(tokens.c_scale_label.get(index));
        // The depth stroke: `17 - 3i` in the classic look, `max(10 - (i - 1) * 2, 2)` in the neo look
        // (styles.ts 20).
        let edge_width = if neo {
            (10 - (index as i64 - 1) * 2).max(2)
        } else {
            17 - 3 * index as i64
        };
        css.push_str(&format!(
            "#{id} .section-{section} rect,#{id} .section-{section} path,#{id} .section-{section} circle,#{id} .section-{section} polygon,#{id} .section-{section} path{{fill:{fill};}}\
             #{id} .section-{section} text{{fill:{label};}}\
             #{id} .section-{section} span{{color:{label};}}\
             #{id} .section-edge-{section}{{stroke:{fill};}}\
             #{id} .edge-depth-{section}{{stroke-width:{edge_width};}}"
        ));
        // The neo look swaps the fill of a node: the main background, with the node border as stroke.
        if neo {
            css.push_str(&format!(
                "#{id} [data-look=\"neo\"].mindmap-node.section-{section} rect,#{id} [data-look=\"neo\"].mindmap-node.section-{section} path,#{id} [data-look=\"neo\"].mindmap-node.section-{section} circle,#{id} [data-look=\"neo\"].mindmap-node.section-{section} polygon{{fill:{main_bkg};stroke:{node_border};stroke-width:{stroke_width}px;}}\
                 #{id} [data-look=\"neo\"].section-edge-{section}{{stroke:{node_border};}}\
                 #{id} [data-look=\"neo\"].mindmap-node.section-{section} text,#{id} [data-look=\"neo\"].mindmap-node.section-{section} span{{fill:{node_border};color:{node_border};}}"
            ));
        }
    }

    // The disabled nodes (styles.ts 53-58): lightgray, which no mindmap element takes.
    // `.section-root` (styles.ts 108-116): the root's fill and label, from the git colours.
    css.push_str(&format!(
        "#{id} .section-root rect,#{id} .section-root path,#{id} .section-root circle,#{id} .section-root polygon{{fill:{};}}\
         #{id} .section-root text{{fill:{};}}\
         #{id} .section-root span{{color:{};}}",
        text(tokens.git.get(0)),
        text(tokens.git_branch_label.get(0)),
        node_border,
    ));

    // The neo shadow and the root's neo fill and label (styles.ts 133-142).
    if neo {
        css.push_str(&format!(
            "#{id} [data-look=\"neo\"].mindmap-node{{filter:{drop_shadow};}}\
             #{id} [data-look=\"neo\"].mindmap-node.section-root rect,#{id} [data-look=\"neo\"].mindmap-node.section-root path,#{id} [data-look=\"neo\"].mindmap-node.section-root circle,#{id} [data-look=\"neo\"].mindmap-node.section-root polygon{{fill:{main_bkg};}}\
             #{id} [data-look=\"neo\"].mindmap-node.section-root .text-inner-tspan,#{id} [data-look=\"neo\"].mindmap-node.section-root span{{fill:{node_border};color:{node_border};}}"
        ));
    }
    css
}
