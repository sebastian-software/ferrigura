//! The stylesheet of the architecture diagram: `architectureStyles.ts`, with the colours of the theme
//! (`archEdgeColor`, `archEdgeArrowColor`, `archEdgeWidth`, `archGroupBorderColor` and
//! `archGroupBorderWidth`), each rule under the diagram's id.

use crate::context::ThemeView;

/// The rules of the diagram, selected under `#id` as the reference's stylis does (`styles.ts` 34-38).
#[must_use]
pub fn own_rules(id: &str, theme: ThemeView<'_>) -> String {
    let tokens = theme.light;
    let edge_color = tokens
        .arch_edge_color
        .text()
        .unwrap_or_else(|| "#333333".to_owned());
    let arrow_color = tokens
        .arch_edge_arrow_color
        .text()
        .unwrap_or_else(|| edge_color.clone());
    let edge_width = tokens
        .arch_edge_width
        .text()
        .unwrap_or_else(|| "3".to_owned());
    let group_color = tokens
        .arch_group_border_color
        .text()
        .unwrap_or_else(|| "#333333".to_owned());
    let group_width = tokens
        .arch_group_border_width
        .text()
        .unwrap_or_else(|| "2px".to_owned());
    format!(
        "#{id} .edge{{stroke-width: {edge_width};stroke: {edge_color};fill: none;}}\
#{id} .arrow{{fill: {arrow_color};}}\
#{id} .node-bkg{{fill: none;stroke: {group_color};stroke-width: {group_width};stroke-dasharray: 8;}}\
#{id} .node-icon-text{{display: flex;align-items: center;}}\
#{id} .node-icon-text > div{{color: #fff;margin: 1px;height: fit-content;text-align: center;overflow: hidden;display: -webkit-box;-webkit-box-orient: vertical;}}"
    )
}
