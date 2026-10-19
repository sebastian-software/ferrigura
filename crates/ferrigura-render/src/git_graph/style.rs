//! The stylesheet of a gitGraph: the rules `styles.js` writes for a colour theme (the redux themes,
//! which are the only ones ported), with every colour as a custom property of the diagram.
//!
//! The reference writes its colours into the rules; here each colour is `var(--name)`, and the
//! custom properties are declared on the diagram's id, for the light scheme and, when both
//! schemes are written, for the dark one under `prefers-color-scheme: dark` (decision 0007). Every
//! selector starts with the id.

use ferrigura_text::EmbeddedFamily;

use super::palette::Palette;
use crate::context::ThemeView;
use crate::svg::{css_text, font_faces, number};

/// The `<style>` element of the diagram: the embedded font faces, the colour properties of the
/// themes and the rules.
#[must_use]
pub fn style_element(
    id: &str,
    theme: ThemeView<'_>,
    light: &Palette,
    dark: &Palette,
    family: &str,
    families: &[EmbeddedFamily],
) -> String {
    let tokens = match theme.dark {
        None => format!("#{id}{{{}}}", light.declarations()),
        Some(_) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            light.declarations(),
            dark.declarations()
        ),
    };
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    format!(
        "<style>{}{tokens}{}</style>",
        css_text(&faces),
        rules(id, light, family)
    )
}

/// The rules of the diagram, from `styles.js` (`getStyles`, with `genColor`'s colour-theme
/// branch). Sizes and weights are the theme's own values, as the reference writes them.
fn rules(id: &str, palette: &Palette, family: &str) -> String {
    let stroke = &palette.stroke_width;
    let weight = &palette.note_font_weight;
    let mut css = format!(
        "#{id}{{font-family:{family};font-size:{}px;fill:var(--textColor);}}\
#{id} .commit-id,#{id} .commit-msg,#{id} .branch-label{{fill:lightgrey;color:lightgrey;font-family:{family};}}",
        number(palette.font_size)
    );
    for index in 0..palette.colour_limit {
        if index == 0 {
            css.push_str(&format!(
                "#{id} .branch-label0{{fill:var(--nodeBorder);font-weight:{weight};}}\
#{id} .commit0{{stroke:var(--nodeBorder);}}\
#{id} .commit-highlight0{{stroke:var(--nodeBorder);fill:var(--mainBkg);}}\
#{id} .label0{{fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke};font-weight:{weight};}}\
#{id} .arrow0{{stroke:var(--nodeBorder);}}\
#{id} .commit-bullets{{fill:var(--nodeBorder);}}"
            ));
        } else {
            css.push_str(&format!(
                "#{id} .branch-label{index}{{fill:var(--nodeBorder);font-weight:{weight};}}\
#{id} .commit{index}{{stroke:var(--gitBorder{index});fill:var(--gitBorder{index});}}\
#{id} .commit-highlight{index}{{stroke:var(--gitBorder{index});fill:var(--gitBorder{index});}}\
#{id} .label{index}{{fill:var(--gitLabelFill{index});stroke:var(--gitBorder{index});stroke-width:{stroke};}}\
#{id} .arrow{index}{{stroke:var(--gitBorder{index});}}"
            ));
        }
    }
    css.push_str(&format!(
        "#{id} .branch{{stroke-width:{stroke};stroke:var(--gitLine);stroke-dasharray:4 2;}}\
#{id} .commit-label{{font-size:{}px;fill:var(--nodeBorder);font-weight:{weight};}}\
#{id} .commit-label-bkg{{font-size:{}px;fill:transparent;}}\
#{id} .tag-label{{font-size:{}px;fill:var(--tagLabelColor);}}\
#{id} .tag-label-bkg{{fill:var(--mainBkg);stroke:var(--nodeBorder);filter:{};}}\
#{id} .tag-hole{{fill:var(--textColor);}}\
#{id} .commit-merge{{stroke:var(--mainBkg);fill:var(--mainBkg);}}\
#{id} .commit-reverse{{stroke:var(--mainBkg);fill:var(--mainBkg);stroke-width:{stroke};}}\
#{id} .commit-highlight-inner{{stroke:var(--mainBkg);fill:var(--mainBkg);}}\
#{id} .commit-cherry-pick[fill]{{fill:var(--gitEye);}}\
#{id} line.commit-cherry-pick{{stroke:var(--gitEye);}}\
#{id} .arrow{{stroke-width:{stroke};stroke-linecap:round;fill:none;}}\
#{id} .gitTitleText{{text-anchor:middle;font-size:18px;fill:var(--textColor);}}",
        number(palette.commit_label_font_size),
        number(palette.commit_label_font_size),
        number(palette.tag_label_font_size),
        css_text(&palette.drop_shadow),
    ));
    css
}
