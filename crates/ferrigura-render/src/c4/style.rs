//! The style sheet of the C4 diagram: `styles.js`. The diagram's rules come between the base rules and
//! the look rules, as the reference places a diagram's own rules (`common/style.rs`). Every selector
//! starts with `#id` (decision 0007). The element label's font is the embedded family of the diagram's
//! text, not the configured `"Open Sans"`: decision 0005 has every label drawn in the face it was
//! measured in, and the reference's `"Open Sans"` is mapped onto the bundled face by the harness.

use super::config::Conf;
use crate::svg::number;

/// The element types, each followed by its external kind, in the order of `C4_ELEMENT_TYPES`
/// (`c4ShapeAdapter.ts:117-130`).
const ELEMENT_BASES: [&str; 10] = [
    "person",
    "system",
    "system_db",
    "system_queue",
    "container",
    "container_db",
    "container_queue",
    "component",
    "component_db",
    "component_queue",
];

/// The rules of `styles.js` for the diagram with the id `id`. `family` is the CSS family of the
/// diagram's text (the embedded subset's name), and `person_border` and `person_background` are the
/// theme's colours of the person (`styles.js:37-40`).
#[must_use]
pub fn own_rules(
    id: &str,
    conf: &Conf,
    family: &str,
    person_border: &str,
    person_background: &str,
) -> String {
    let mut css = format!("#{id} .person{{stroke:{person_border};fill:{person_background};}}");
    for base in ELEMENT_BASES {
        for type_name in [base.to_owned(), format!("external_{base}")] {
            if let Some(element) = conf.element(&type_name) {
                css.push_str(&format!(
                    "#{id} .c4-shape.c4-{type_name} .label{{font-family:{family};font-size:{}px;font-weight:{};}}",
                    number(element.font.size),
                    element.font.weight,
                ));
            }
        }
    }
    css.push_str(&format!(
        "#{id} .c4-shape .label,#{id} .c4-shape .label text{{color:inherit;fill:currentColor;}}\
#{id} .c4-shape .label .c4-name{{font-weight:bold;}}\
#{id} .c4-shape .label .c4-type{{font-size:0.75em;}}\
#{id} .c4-shape .label .c4-descr{{font-size:0.82em;}}\
#{id} .c4-shape .basic,#{id} .c4-shape rect,#{id} .c4-shape path,#{id} .c4-shape circle,#{id} .c4-shape ellipse,#{id} .c4-shape line{{stroke-width:2px;}}"
    ));
    css
}
