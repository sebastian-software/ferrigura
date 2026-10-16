//! Port of `rendering-elements/markers.spec.ts`: every marker carries `markerUnits="userSpaceOnUse"`,
//! so its size does not scale with the stroke width.
//!
//! The reference checks the class-diagram markers, which the flowchart port does not draw. The
//! flowchart's twelve markers are the ones that carry the attribute here, so the test checks them.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::flowchart::markers::markers;

#[test]
fn sets_marker_units_user_space_on_use_on_every_marker_so_size_is_independent_of_stroke_width() {
    let svg = markers("test", &MermaidConfig::default());
    // Each `<marker ` opens one definition, and its opening tag carries the attribute.
    let definitions: Vec<&str> = svg.split("<marker ").skip(1).collect();
    assert_eq!(definitions.len(), 12);
    let offenders: Vec<&str> = definitions
        .iter()
        .copied()
        .filter(|definition| !definition.contains("markerUnits=\"userSpaceOnUse\""))
        .collect();
    assert!(offenders.is_empty(), "{offenders:?}");
}
