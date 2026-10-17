//! Port of packages/mermaid/src/diagrams/block/layout.spec.ts: the position of a block in its grid
//! (`calculateBlockPosition`), and that the layout reads the padding at call time, not at import time.

use ferrigura_render::block::Block;
use ferrigura_render::block::Size;
use ferrigura_render::block::layout::{calculate_block_position, layout};

#[test]
fn should_calculate_position_correctly() {
    assert_eq!(calculate_block_position(2.0, 0.0).unwrap(), (0.0, 0.0));
    assert_eq!(calculate_block_position(2.0, 1.0).unwrap(), (1.0, 0.0));
    assert_eq!(calculate_block_position(2.0, 2.0).unwrap(), (0.0, 1.0));
    assert_eq!(calculate_block_position(2.0, 3.0).unwrap(), (1.0, 1.0));
    assert_eq!(calculate_block_position(2.0, 4.0).unwrap(), (0.0, 2.0));
    assert_eq!(calculate_block_position(1.0, 3.0).unwrap(), (0.0, 3.0));
}

#[test]
fn a_grid_of_zero_columns_is_an_error() {
    assert!(calculate_block_position(0.0, 0.0).is_err());
}

/// A composite with two blocks of 100 by 50, in two columns.
fn two_blocks() -> Block {
    let leaf = |id: &str| Block {
        id: id.to_owned(),
        block_type: Some("square".to_owned()),
        label: id.to_owned(),
        columns: None,
        width_in_columns: None,
        directions: Vec::new(),
        classes: Vec::new(),
        styles: Vec::new(),
        color_index: None,
        children: Vec::new(),
        size: Some(Size {
            width: 100.0,
            height: 50.0,
            x: 0.0,
            y: 0.0,
        }),
    };
    Block {
        id: "root".to_owned(),
        block_type: Some("composite".to_owned()),
        label: String::new(),
        columns: Some(2.0),
        width_in_columns: None,
        directions: Vec::new(),
        classes: Vec::new(),
        styles: Vec::new(),
        color_index: None,
        children: vec![leaf("b1"), leaf("b2")],
        size: None,
    }
}

#[test]
fn should_read_block_padding_from_config_at_call_time_not_at_import_time() {
    // padding=4:  width = 2*(100+4)+4 = 212; padding=20: width = 2*(100+20)+20 = 260. Were the padding
    // read once, both layouts would be the same.
    let small = layout(&mut two_blocks(), 4.0).expect("laid out");
    let large = layout(&mut two_blocks(), 20.0).expect("laid out");

    assert_ne!(small, large);
    assert!(small.width < large.width);
}
