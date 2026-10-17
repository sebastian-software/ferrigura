//! The stylesheet of the architecture diagram: `architectureStyles.ts`, with the colours of the theme
//! (`archEdgeColor`, `archEdgeArrowColor`, `archEdgeWidth`, `archGroupBorderColor` and
//! `archGroupBorderWidth`), each rule under the diagram's id.

use crate::theme::Themes;
use crate::variables::text_of;

/// The rules of the diagram, selected under `#id` as the reference's stylis does (`styles.ts` 34-38).
#[must_use]
pub fn own_rules(id: &str, themes: Themes<'_>) -> String {
    let variables = themes.primary();
    let edge_color = text_of(variables, "archEdgeColor").unwrap_or_else(|| "#333333".to_owned());
    let arrow_color =
        text_of(variables, "archEdgeArrowColor").unwrap_or_else(|| edge_color.clone());
    let edge_width = text_of(variables, "archEdgeWidth").unwrap_or_else(|| "3".to_owned());
    let group_color =
        text_of(variables, "archGroupBorderColor").unwrap_or_else(|| "#333333".to_owned());
    let group_width =
        text_of(variables, "archGroupBorderWidth").unwrap_or_else(|| "2px".to_owned());
    format!(
        "#{id} .edge{{stroke-width: {edge_width};stroke: {edge_color};fill: none;}}\
#{id} .arrow{{fill: {arrow_color};}}\
#{id} .node-bkg{{fill: none;stroke: {group_color};stroke-width: {group_width};stroke-dasharray: 8;}}\
#{id} .node-icon-text{{display: flex;align-items: center;}}\
#{id} .node-icon-text > div{{color: #fff;margin: 1px;height: fit-content;text-align: center;overflow: hidden;display: -webkit-box;-webkit-box-orient: vertical;}}"
    )
}
