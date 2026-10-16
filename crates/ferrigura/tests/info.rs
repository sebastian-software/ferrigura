//! The info diagram through the public entry point: the version text, the root and the style sheet.
//! The geometry is checked against the reference by `pnpm port:geometry info`.

use ferrigura::render;
use ferrigura_model::value::Value;

fn svg(text: &str) -> String {
    render(text, &Value::empty_object()).expect("the info diagram renders")
}

#[test]
fn should_draw_the_version_centred_at_the_top_of_a_400_pixel_root() {
    let svg = svg("info");
    assert!(
        svg.contains(
            "<g><text x=\"100\" y=\"40\" class=\"version\" font-size=\"32\" style=\"text-anchor: middle;\">v12.2.0</text></g>"
        ),
        "{svg}"
    );
    assert!(svg.contains("width=\"100%\""), "{svg}");
    assert!(svg.contains("style=\"max-width: 400px;\""), "{svg}");
    assert!(!svg.contains("viewBox"), "{svg}");
}

#[test]
fn should_render_the_showinfo_form_the_same_way() {
    // The id of the root is derived from the text, so only the version element is compared.
    let version = "<g><text x=\"100\" y=\"40\" class=\"version\" font-size=\"32\" style=\"text-anchor: middle;\">v12.2.0</text></g>";
    assert!(svg("info showInfo").contains(version));
}

#[test]
fn should_write_the_accessibility_roles_and_no_title() {
    let svg = svg("info");
    assert!(svg.contains("role=\"graphics-document document\""), "{svg}");
    assert!(svg.contains("aria-roledescription=\"info\""), "{svg}");
    assert!(!svg.contains("<title"), "{svg}");
    assert!(!svg.contains("aria-labelledby"), "{svg}");
}

#[test]
fn should_scope_every_style_rule_to_the_diagram_id() {
    let svg = svg("info");
    let id = svg
        .split("id=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the root has an id");
    assert!(svg.contains(&format!("#{id}{{font-family:")), "{svg}");
    assert!(svg.contains(&format!("#{id} .marker{{")), "{svg}");
}
