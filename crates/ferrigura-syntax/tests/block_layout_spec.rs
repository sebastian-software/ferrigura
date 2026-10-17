//! Port of packages/mermaid/src/diagrams/block/layout.spec.ts. The two tests check the layout of
//! the blocks (`calculateBlockPosition`, and `layout` reading `block.padding` at call time), which is
//! the renderer stage's (`docs-port/block/PLAN.md`, section 1). They are ignored here and move to
//! the renderer's tests with the layout.

#[test]
#[ignore = "port: the layout is the renderer stage's (calculateBlockPosition, layout.ts)"]
fn should_calculate_position_correctly() {
    todo!("the renderer stage ports calculateBlockPosition")
}

#[test]
#[ignore = "port: the layout is the renderer stage's (layout.ts reads block.padding at call time)"]
fn should_read_block_padding_from_config_at_call_time_not_at_import_time() {
    todo!("the renderer stage ports layout")
}
