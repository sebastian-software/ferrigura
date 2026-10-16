//! `insertTitle` of `packages/mermaid/src/utils.ts`: the title of a diagram, centred over a box and
//! placed `title_top_margin` above its top edge (`y = -titleTopMargin`).

use crate::svg::number;

use super::Element;
use super::viewbox::BBox;

/// The `<text>` the reference appends to the parent group, or `None` for an empty title (the
/// reference returns before it measures anything).
#[must_use]
pub fn insert_title(
    parent: BBox,
    css_class: &str,
    title_top_margin: f64,
    title: &str,
) -> Option<Element> {
    if title.is_empty() {
        return None;
    }
    Some(
        Element::new("text")
            .with("text-anchor", "middle")
            .with("x", number(parent.x + parent.width / 2.0))
            .with("y", number(-title_top_margin))
            .with("class", css_class)
            .text(title),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The probe's case: a group whose box is (10, 5, 100, 40), with the margin 25.
    #[test]
    fn the_title_is_centred_over_the_box_and_placed_above_it() {
        let parent = BBox {
            x: 10.0,
            y: 5.0,
            width: 100.0,
            height: 40.0,
        };
        let title = insert_title(parent, "gitTitleText", 25.0, "My title").expect("a title");
        assert_eq!(
            title.render(),
            "<text text-anchor=\"middle\" x=\"60\" y=\"-25\" class=\"gitTitleText\">My title</text>"
        );
    }

    #[test]
    fn markup_in_the_title_is_escaped_and_an_empty_title_is_absent() {
        let parent = BBox {
            x: 10.0,
            y: 5.0,
            width: 100.0,
            height: 40.0,
        };
        let title =
            insert_title(parent, "title", 0.0, "Title with <b> & \"quotes\"").expect("a title");
        assert_eq!(
            title.render(),
            "<text text-anchor=\"middle\" x=\"60\" y=\"0\" class=\"title\">Title with &lt;b&gt; &amp; &quot;quotes&quot;</text>"
        );
        assert!(insert_title(parent, "title", 0.0, "").is_none());
    }
}
