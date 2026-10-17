//! Port of `packages/mermaid/src/rendering-util/rendering-elements/shapes/c4LabelHelper.spec.ts`:
//! its two cases, as ignored tests (2 tests). The spec measures the label layout of the renderer,
//! which is not ported yet, so there is no call to make here. The plan is `docs-port/c4/PLAN.md`.

// c4LabelHelper.spec.ts:47
#[test]
#[ignore = "the element label layout of the renderer is not ported yet"]
fn wraps_element_labels_by_default_so_the_label_stays_within_c4_width() {
    // The reference wraps the long description, so the element has more than the three outer
    // tspan elements of its name, stereotype and description (spec line 52).
    unimplemented!("the renderer's element label layout is not ported yet");
}

// c4LabelHelper.spec.ts:55
#[test]
#[ignore = "the element label layout of the renderer is not ported yet"]
fn does_not_wrap_when_c4_wrap_is_disabled() {
    // With c4.wrap disabled, the reference draws one line per section: three outer tspan
    // elements (spec line 60).
    unimplemented!("the renderer's element label layout is not ported yet");
}
