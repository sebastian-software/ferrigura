//! Accessibility of the SVG: the role, the accessible title and the description, as Mermaid's
//! `accessibility.ts` writes them. The title and the description are the first children of the root
//! element, the title first, and the root refers to them by id.

use crate::svg::escape;

/// The role of every diagram's SVG: `graphics-document` per the SVG standard, and `document` for the
/// browsers that only know ARIA 1.0.
pub const SVG_ROLE: &str = "graphics-document document";

/// The id of the title element of a diagram with the given base id.
#[must_use]
pub fn title_id(base_id: &str) -> String {
    format!("chart-title-{base_id}")
}

/// The id of the description element of a diagram with the given base id.
#[must_use]
pub fn description_id(base_id: &str) -> String {
    format!("chart-desc-{base_id}")
}

/// The attributes of the root `<svg>` element for the role and the names: `role`, the diagram type
/// as `aria-roledescription` (not when it is empty), and `aria-labelledby` and `aria-describedby` for
/// the title and the description when they are not empty. Each attribute is written with a leading
/// space, so the result can be placed after the other attributes.
#[must_use]
pub fn root_attributes(
    base_id: &str,
    diagram_type: &str,
    title: &str,
    description: &str,
) -> String {
    let mut out = format!(" role=\"{SVG_ROLE}\"");
    if !diagram_type.is_empty() {
        out.push_str(&format!(
            " aria-roledescription=\"{}\"",
            escape(diagram_type)
        ));
    }
    if !description.is_empty() {
        out.push_str(&format!(
            " aria-describedby=\"{}\"",
            description_id(base_id)
        ));
    }
    if !title.is_empty() {
        out.push_str(&format!(" aria-labelledby=\"{}\"", title_id(base_id)));
    }
    out
}

/// The `<title>` and `<desc>` children of the root element, the title first, for the texts that are
/// not empty.
#[must_use]
pub fn children(base_id: &str, title: &str, description: &str) -> String {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!(
            "<title id=\"{}\">{}</title>",
            title_id(base_id),
            escape(title)
        ));
    }
    if !description.is_empty() {
        out.push_str(&format!(
            "<desc id=\"{}\">{}</desc>",
            description_id(base_id),
            escape(description)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // The cases of `packages/mermaid/src/accessibility.spec.ts`, under the same names.

    #[test]
    fn should_set_svg_element_role_to_graphics_document_document() {
        assert!(
            root_attributes("id", "flowchart", "", "")
                .contains("role=\"graphics-document document\"")
        );
    }

    #[test]
    fn should_set_aria_roledescription_to_the_diagram_type() {
        assert!(
            root_attributes("id", "flowchart", "", "")
                .contains("aria-roledescription=\"flowchart\"")
        );
    }

    #[test]
    fn should_not_set_aria_roledescription_if_the_diagram_type_is_empty() {
        assert!(!root_attributes("id", "", "", "").contains("aria-roledescription"));
    }

    #[test]
    fn should_set_aria_labelledby_to_the_title_id_inserted_as_a_child() {
        let attributes = root_attributes("theBaseId", "pie", "a11y title", "a11y description");
        assert!(attributes.contains("aria-labelledby=\"chart-title-theBaseId\""));
    }

    #[test]
    fn should_set_aria_describedby_to_the_description_id_inserted_as_a_child() {
        let attributes = root_attributes("theBaseId", "pie", "a11y title", "a11y description");
        assert!(attributes.contains("aria-describedby=\"chart-desc-theBaseId\""));
    }

    #[test]
    fn should_insert_title_tag_as_the_first_child_with_the_text_set_to_the_acc_title_given() {
        let children = children("theBaseId", "a11y title", "a11y description");
        assert!(children.starts_with("<title id=\"chart-title-theBaseId\">a11y title</title>"));
    }

    #[test]
    fn should_insert_desc_tag_as_the_2nd_child_with_the_text_set_to_acc_description_given() {
        let children = children("theBaseId", "a11y title", "a11y description");
        assert!(children.contains("<desc id=\"chart-desc-theBaseId\">a11y description</desc>"));
        assert!(
            children.find("<title").unwrap_or(usize::MAX) < children.find("<desc").unwrap_or(0)
        );
    }

    #[test]
    fn without_a_description_should_not_set_aria_describedby() {
        let attributes = root_attributes("theBaseId", "pie", "a11y title", "");
        assert!(!attributes.contains("aria-describedby"));
        assert!(attributes.contains("aria-labelledby=\"chart-title-theBaseId\""));
    }

    #[test]
    fn without_a_description_should_not_insert_desc_tag() {
        let children = children("theBaseId", "a11y title", "");
        assert!(children.contains("<title"));
        assert!(!children.contains("<desc"));
    }

    #[test]
    fn without_a_title_should_not_set_aria_labelledby() {
        let attributes = root_attributes("theBaseId", "pie", "", "a11y description");
        assert!(!attributes.contains("aria-labelledby"));
        assert!(attributes.contains("aria-describedby=\"chart-desc-theBaseId\""));
    }

    #[test]
    fn without_a_title_should_not_insert_title_tag() {
        let children = children("theBaseId", "", "a11y description");
        assert!(!children.contains("<title"));
        assert!(children.contains("<desc id=\"chart-desc-theBaseId\">a11y description</desc>"));
    }

    #[test]
    fn the_texts_are_escaped() {
        let children = children("id", "a < b", "x & y");
        assert!(children.contains("a &lt; b"));
        assert!(children.contains("x &amp; y"));
    }
}
