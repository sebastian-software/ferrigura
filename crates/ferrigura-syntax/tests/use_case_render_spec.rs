//! The reference's renderer and colour specs of the use case diagram. They test the renderer
//! (`usecaseRenderer.spec.ts`: the markers, the accessible names, the fonts measured), the role
//! colours of the stylesheet (`usecaseRoleColors.spec.ts`), and the colour slots that `getData`
//! assigns (`usecase-colorIndex.spec.ts`). None of them is in this port: they belong to the renderer
//! stage (`docs-port/P3-PLAN.md`), so each stays an ignored test that names its reason.

#[test]
#[ignore = "the renderer of the use case diagram is not ported (usecaseRenderer.spec.ts, the renderer stage)"]
fn renderer_spec_is_not_ported() {
    unimplemented!("usecaseRenderer.spec.ts: 6 tests and 1 table, with the renderer stage");
}

#[test]
#[ignore = "the stylesheet of the use case diagram is not ported (usecaseRoleColors.spec.ts, the renderer stage)"]
fn role_colors_spec_is_not_ported() {
    unimplemented!(
        "usecaseRoleColors.spec.ts: 12 tests and 3 tables, with the stylesheet of the renderer stage"
    );
}

#[test]
#[ignore = "the colour slots of getData are not ported (usecase-colorIndex.spec.ts, the renderer stage)"]
fn color_index_spec_is_not_ported() {
    unimplemented!("usecase-colorIndex.spec.ts: 6 tests on getData, with the renderer stage");
}
