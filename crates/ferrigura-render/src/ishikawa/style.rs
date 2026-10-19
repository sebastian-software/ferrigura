//! The style sheet of the ishikawa diagram: `ishikawaStyles.ts` for the redux themes, with the
//! colours as custom properties on the diagram's id (`docs-port/decisions/0007`). The line and text
//! colours are the base sheet's own tokens; the background of the head and of the labels is the
//! `--mainBkg` token of this sheet.
//!
//! Every rule starts with the diagram's id. The rules are the reference's, selector for selector,
//! except the `dominant-baseline: baseline` of the `up` labels, which the reference writes and which
//! is not a `dominant-baseline` keyword: Chromium drops that declaration, so the baseline is the
//! alphabetic one (`text.rs`, `Baseline::Alphabetic`).

use ferrigura_text::EmbeddedFamily;

use crate::common::style::base_style_sheet;
use crate::context::ThemeView;

/// The background of the head and of the labels, in each scheme. Light only when the theme has no
/// dark set.
fn background_token(id: &str, theme: ThemeView<'_>) -> String {
    let light = theme
        .light
        .main_bkg
        .text()
        .unwrap_or_else(|| "#ffffff".to_owned());
    match theme.dark {
        None => format!("#{id}{{--mainBkg:{light};}}"),
        Some(dark) => {
            let dark = dark.main_bkg.text().unwrap_or_else(|| "#ffffff".to_owned());
            format!(
                "#{id}{{--mainBkg:{light};}}@media (prefers-color-scheme: dark){{#{id}{{--mainBkg:{dark};}}}}"
            )
        }
    }
}

/// The rules of the diagram, after the base rules. `family` is the font family as CSS reads it, and
/// `label_size` the theme's `fontSize`, which the stylesheet gives the labels.
fn rules(id: &str, family: &str, label_size: &str) -> String {
    format!(
        "#{id} .ishikawa .ishikawa-spine,#{id} .ishikawa .ishikawa-branch,#{id} .ishikawa .ishikawa-sub-branch{{stroke:var(--lineColor);stroke-width:2;fill:none;}}\
#{id} .ishikawa .ishikawa-sub-branch{{stroke-width:1;}}\
#{id} .ishikawa .ishikawa-arrow{{fill:var(--lineColor);}}\
#{id} .ishikawa .ishikawa-head{{fill:var(--mainBkg);stroke:var(--lineColor);stroke-width:2;}}\
#{id} .ishikawa .ishikawa-label-box{{fill:var(--mainBkg);stroke:var(--lineColor);stroke-width:2;}}\
#{id} .ishikawa text{{font-family:{family};font-size:{label_size};fill:var(--textColor);}}\
#{id} .ishikawa .ishikawa-head-label{{font-weight:600;text-anchor:middle;dominant-baseline:middle;font-size:14px;}}\
#{id} .ishikawa .ishikawa-label{{text-anchor:end;}}\
#{id} .ishikawa .ishikawa-label.cause{{text-anchor:middle;dominant-baseline:middle;}}\
#{id} .ishikawa .ishikawa-label.align{{text-anchor:end;dominant-baseline:middle;}}\
#{id} .ishikawa .ishikawa-label.up{{dominant-baseline:baseline;}}\
#{id} .ishikawa .ishikawa-label.down{{dominant-baseline:hanging;}}"
    )
}

/// The `<style>` text of the diagram: the base rules with the embedded faces, then the colour of the
/// background, then the ishikawa rules. The whole sheet is escaped once, for the XML element.
#[must_use]
pub fn sheet(
    id: &str,
    theme: ThemeView<'_>,
    family: &str,
    label_size: &str,
    families: &[EmbeddedFamily],
) -> String {
    let own = format!(
        "{}{}",
        background_token(id, theme),
        rules(id, family, label_size)
    );
    base_style_sheet(id, theme, family, label_size, families, &own)
}
