//! The layout and the drawing of a gitGraph, step by step as `gitGraphRenderer.ts` does them: the
//! positions of the branches and of the commits, the commits' bullets, labels and tags, the arrows,
//! and the branch lines and labels. The reference draws the commits twice (`drawCommits`, first
//! without the elements, to place them, then with them), so the functions here run in that order.
//!
//! The geometry is the redux geometry, the only one ported (M6-PLAN, themes): the commit radius is
//! 7, and the branch labels are padded by 12 (`REDUX_BRANCH_LABEL_PADDING_Y`).

use std::collections::HashMap;

use ferrigura_model::config::GitGraphDiagramConfig;
use ferrigura_model::git_graph::{
    CommitType, Direction, GitGraphBranch, GitGraphCommit, GitGraphDiagram,
};
use ferrigura_text::Face;

use super::palette::Palette;
use super::text;
use super::tree::{Element, PathBuilder, Transform};
use crate::RenderError;
use crate::unified::geom::{BBox, Point};

/// The offset of a commit from its lane (`LAYOUT_OFFSET`).
const LAYOUT_OFFSET: f64 = 10.0;
/// The distance between two commits along the time axis (`COMMIT_STEP`).
const COMMIT_STEP: f64 = 40.0;
/// Half the padding of a label box, horizontally and vertically (`PX`, `PY`).
const PX: f64 = 4.0;
const PY: f64 = 2.0;
/// The first position of a vertical graph, and the origin of the parallel commits (`defaultPos`).
const DEFAULT_POS: f64 = 30.0;
/// The number of colours of a commit, arrow and bullet (`THEME_COLOR_LIMIT` of the renderer).
const COMMIT_COLOUR_LIMIT: usize = 8;
/// The padding of a branch label, horizontally and vertically, in the redux geometry.
const LABEL_PADDING_X: f64 = 16.0;
const LABEL_PADDING_Y: f64 = 12.0;

/// The lane of a branch and its colour index.
#[derive(Debug, Clone, Copy)]
struct BranchPos {
    pos: f64,
    index: usize,
}

/// Where a commit's bullet and labels go: its position, and the position along the time axis
/// with the offset of the lane (`posWithOffset`).
#[derive(Debug, Clone, Copy)]
struct CommitOffset {
    x: f64,
    y: f64,
    pos_with_offset: f64,
}

/// The state of one drawing: the branches and commits, their positions, and the lanes the
/// arrows use.
pub(super) struct Layout<'a> {
    commits: &'a [GitGraphCommit],
    commit_index: HashMap<&'a str, usize>,
    branches: Vec<&'a GitGraphBranch>,
    dir: Direction,
    config: &'a GitGraphDiagramConfig,
    palette: &'a Palette,
    id: &'a str,
    neo: bool,
    branch_pos: HashMap<&'a str, BranchPos>,
    commit_pos: HashMap<&'a str, Point>,
    lanes: Vec<f64>,
    max_pos: f64,
}

impl<'a> Layout<'a> {
    /// The state of a drawing of `diagram`. The branches are in the order of their `order`, as the
    /// reference's `getBranchesAsObjArray` sorts them (a stable sort).
    pub(super) fn new(
        diagram: &'a GitGraphDiagram,
        config: &'a GitGraphDiagramConfig,
        palette: &'a Palette,
        id: &'a str,
        neo: bool,
    ) -> Self {
        let mut branches: Vec<&GitGraphBranch> = diagram.branches.iter().collect();
        branches.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let commit_index = diagram
            .commits
            .iter()
            .enumerate()
            .map(|(index, commit)| (commit.id.as_str(), index))
            .collect();
        Self {
            commits: &diagram.commits,
            commit_index,
            branches,
            dir: diagram.direction,
            config,
            palette,
            id,
            neo,
            branch_pos: HashMap::new(),
            commit_pos: HashMap::new(),
            lanes: Vec::new(),
            max_pos: 0.0,
        }
    }

    /// The branches, in drawing order.
    pub(super) fn branches(&self) -> &[&'a GitGraphBranch] {
        &self.branches
    }

    /// Whether time runs down the page (`TB` and `BT`).
    fn vertical(&self) -> bool {
        matches!(self.dir, Direction::TB | Direction::BT)
    }

    /// The lane of a named branch, or the reference's error when the branch is unknown.
    fn lane(&self, branch: &str) -> Result<BranchPos, RenderError> {
        self.branch_pos
            .get(branch)
            .copied()
            .ok_or_else(|| RenderError::Layout(format!("branch not found: {branch}")))
    }

    /// `setBranchPosition`: the lane of a branch, and the position of the next one. The step after
    /// a branch is its label's width (in a vertical graph) and the rotated label's room.
    pub(super) fn set_branch_position(
        &mut self,
        name: &'a str,
        pos: f64,
        index: usize,
        label_width: f64,
        rotate: bool,
    ) -> f64 {
        self.branch_pos.insert(name, BranchPos { pos, index });
        pos + 50.0
            + if rotate { 40.0 } else { 0.0 }
            + if self.vertical() {
                label_width / 2.0
            } else {
                0.0
            }
    }

    /// `drawCommits`: places every commit and, when `modify` is set, draws its bullet, label and
    /// tags. Returns the `commit-bullets` and `commit-labels` groups, which are empty when the
    /// commits are not drawn.
    pub(super) fn draw_commits(&mut self, modify: bool) -> Result<(Element, Element), RenderError> {
        let commits = self.commits;
        let mut bullets = Element::group().class("commit-bullets");
        let mut labels = Element::group().class("commit-labels");
        let parallel = self.config.parallel_commits;
        let mut pos = if self.vertical() { DEFAULT_POS } else { 0.0 };
        let mut sorted: Vec<usize> = (0..commits.len()).collect();
        sorted.sort_by_key(|&index| commits[index].seq);
        if self.dir == Direction::BT {
            if parallel {
                self.set_parallel_bt_pos(&sorted, pos)?;
            }
            sorted.reverse();
        }
        for &index in &sorted {
            let commit = &commits[index];
            if parallel {
                pos = self.calculate_position(index);
            }
            let at = self.commit_position(index, pos, parallel)?;
            if modify {
                let branch_index = self
                    .branch_pos
                    .get(commit.branch.as_str())
                    .map_or(0, |branch| branch.index);
                self.draw_bullet(&mut bullets, commit, at, branch_index);
                self.draw_label(&mut labels, commit, at, pos)?;
                self.draw_tags(&mut labels, commit, at, pos)?;
            }
            let point = if self.vertical() {
                Point::new(at.x, at.pos_with_offset)
            } else {
                Point::new(at.pos_with_offset, at.y)
            };
            self.commit_pos.insert(commit.id.as_str(), point);
            pos = if self.dir == Direction::BT && parallel {
                pos + COMMIT_STEP
            } else {
                pos + COMMIT_STEP + LAYOUT_OFFSET
            };
            if pos > self.max_pos {
                self.max_pos = pos;
            }
        }
        Ok((bullets, labels))
    }

    /// `getCommitPosition`: where a commit sits, given its position along the time axis.
    fn commit_position(
        &self,
        index: usize,
        pos: f64,
        parallel: bool,
    ) -> Result<CommitOffset, RenderError> {
        let commit = &self.commits[index];
        let pos_with_offset = if self.dir == Direction::BT && parallel {
            pos
        } else {
            pos + LAYOUT_OFFSET
        };
        let lane = self.lane(&commit.branch)?.pos;
        let (x, y) = if self.vertical() {
            (lane, pos_with_offset)
        } else {
            (pos_with_offset, lane + LABEL_PADDING_Y / 2.0 + 1.0)
        };
        Ok(CommitOffset {
            x,
            y,
            pos_with_offset,
        })
    }

    /// `findClosestParent`: the parent furthest along the time axis that has a position (the last
    /// one of equal positions); in `BT`, the parent nearest the top.
    fn closest_parent<'p>(&self, parents: &'p [String]) -> Option<&'p str> {
        let bottom_to_top = self.dir == Direction::BT;
        let mut target = if bottom_to_top { f64::INFINITY } else { 0.0 };
        let mut closest = None;
        for parent in parents {
            let Some(position) = self.commit_pos.get(parent.as_str()) else {
                continue;
            };
            let value = if self.vertical() {
                position.y
            } else {
                position.x
            };
            let wins = if bottom_to_top {
                value <= target
            } else {
                value >= target
            };
            if wins {
                closest = Some(parent.as_str());
                target = value;
            }
        }
        closest
    }

    /// `findClosestParentBT`: the parent with the smallest `y` (the last one of equal values), or
    /// an error for a parent without a position, which the reference reports as a TypeError.
    fn closest_parent_bt<'p>(&self, parents: &'p [String]) -> Result<Option<&'p str>, RenderError> {
        let mut closest = None;
        let mut max_position = f64::INFINITY;
        for parent in parents {
            let position = self
                .commit_pos
                .get(parent.as_str())
                .ok_or_else(|| RenderError::Layout(format!("no position for commit {parent}")))?;
            if position.y <= max_position {
                closest = Some(parent.as_str());
                max_position = position.y;
            }
        }
        Ok(closest)
    }

    /// `findClosestParentPos` with `calculateCommitPosition`: the time-axis position of the closest
    /// parent, plus one step.
    fn calculate_commit_position(&self, index: usize) -> Result<f64, RenderError> {
        let commit = &self.commits[index];
        let closest = self.closest_parent(&commit.parents).ok_or_else(|| {
            RenderError::Layout(format!("closest parent not found for commit {}", commit.id))
        })?;
        let position = self.commit_pos.get(closest).ok_or_else(|| {
            RenderError::Layout(format!(
                "closest parent position not found for commit {}",
                commit.id
            ))
        })?;
        Ok(position.y + COMMIT_STEP)
    }

    /// `setParallelBTPos`: the positions of the parallel commits of a `BT` graph, which the reference
    /// computes in two passes, the commits with parents first and the roots after them.
    fn set_parallel_bt_pos(
        &mut self,
        sorted: &[usize],
        default_pos: f64,
    ) -> Result<(), RenderError> {
        let commits = self.commits;
        let mut cur_pos = default_pos;
        let mut max_position = default_pos;
        let mut roots = Vec::new();
        for &index in sorted {
            let commit = &commits[index];
            if !commit.parents.is_empty() {
                cur_pos = self.calculate_commit_position(index)?;
                max_position = max_position.max(cur_pos);
            } else {
                roots.push(index);
            }
            self.set_commit_position(index, cur_pos)?;
        }
        cur_pos = max_position;
        for &index in &roots {
            self.set_root_position(index, cur_pos, default_pos)?;
        }
        for &index in sorted {
            let commit = &commits[index];
            if commit.parents.is_empty() {
                continue;
            }
            let closest = self.closest_parent_bt(&commit.parents)?.ok_or_else(|| {
                RenderError::Layout(format!("closest parent not found for commit {}", commit.id))
            })?;
            let position = self
                .commit_pos
                .get(closest)
                .ok_or_else(|| RenderError::Layout(format!("no position for commit {closest}")))?;
            cur_pos = position.y - COMMIT_STEP;
            if cur_pos <= max_position {
                max_position = cur_pos;
            }
            let x = self.lane(&commit.branch)?.pos;
            self.commit_pos
                .insert(commit.id.as_str(), Point::new(x, cur_pos - LAYOUT_OFFSET));
        }
        Ok(())
    }

    /// `setCommitPosition`: a commit at `cur_pos` along the time axis, in its lane.
    fn set_commit_position(&mut self, index: usize, cur_pos: f64) -> Result<(), RenderError> {
        let commits = self.commits;
        let commit = &commits[index];
        let x = self.lane(&commit.branch)?.pos;
        self.commit_pos
            .insert(commit.id.as_str(), Point::new(x, cur_pos + LAYOUT_OFFSET));
        Ok(())
    }

    /// `setRootPosition`: a root commit, placed below the last commit of the parallel layout.
    fn set_root_position(
        &mut self,
        index: usize,
        cur_pos: f64,
        default_pos: f64,
    ) -> Result<(), RenderError> {
        let commits = self.commits;
        let commit = &commits[index];
        let x = self.lane(&commit.branch)?.pos;
        self.commit_pos
            .insert(commit.id.as_str(), Point::new(x, cur_pos + default_pos));
        Ok(())
    }

    /// `calculatePosition`: the time-axis position of a commit in a graph with parallel commits.
    /// A commit with no position of its parent (or without a parent in `LR`) gets 0, as the
    /// reference's defaults give it.
    fn calculate_position(&self, index: usize) -> f64 {
        let commit = &self.commits[index];
        let own = self
            .commit_pos
            .get(commit.id.as_str())
            .copied()
            .unwrap_or(Point::new(0.0, 0.0));
        if !commit.parents.is_empty() {
            if let Some(closest) = self.closest_parent(&commit.parents) {
                let parent = self
                    .commit_pos
                    .get(closest)
                    .copied()
                    .unwrap_or(Point::new(0.0, 0.0));
                return match self.dir {
                    Direction::TB => parent.y + COMMIT_STEP,
                    Direction::BT => own.y - COMMIT_STEP,
                    Direction::LR => parent.x + COMMIT_STEP,
                };
            }
            0.0
        } else {
            match self.dir {
                Direction::TB => DEFAULT_POS,
                Direction::BT => own.y - COMMIT_STEP,
                Direction::LR => 0.0,
            }
        }
    }

    /// `calcColorIndex`: the colour of a branch index. With `avoid_default` (the colour themes),
    /// the branches after the first one cycle through the colours after the first.
    fn colour_index(raw: usize, limit: usize, avoid_default: bool) -> usize {
        if avoid_default && raw > 0 {
            ((raw - 1) % (limit - 1)) + 1
        } else {
            raw % limit
        }
    }

    /// `drawCommitBullet`: the bullet of a commit, by its symbol (its `customType`, else its type).
    fn draw_bullet(
        &self,
        group: &mut Element,
        commit: &GitGraphCommit,
        at: CommitOffset,
        branch_index: usize,
    ) {
        let symbol = commit.custom_type.unwrap_or(commit.kind);
        let type_class = type_class(symbol);
        let colour = Self::colour_index(branch_index, COMMIT_COLOUR_LIMIT, true);
        let id = commit.id.as_str();
        let border = self.palette.border(colour);
        let bkg = self.palette.main_bkg();
        match symbol {
            CommitType::Highlight => {
                let outer_fill = if colour == 0 {
                    bkg.to_owned()
                } else {
                    self.palette.border(colour)
                };
                group.push(
                    Element::rect(at.x - 10.0 + 3.0, at.y - 10.0 + 3.0, 14.0, 14.0)
                        .class(format!(
                            "commit {id} commit-highlight{colour} {type_class}-outer"
                        ))
                        .attr("stroke", border.clone())
                        .attr("fill", outer_fill),
                );
                group.push(
                    Element::rect(at.x - 6.0 + 2.0, at.y - 6.0 + 2.0, 8.0, 8.0)
                        .class(format!("commit {id} commit{colour} {type_class}-inner"))
                        .attr("stroke", bkg)
                        .attr("fill", bkg),
                );
            }
            CommitType::CherryPick => {
                let eye = self.palette.eye();
                let body = format!("commit {id} {type_class}");
                group.push(Element::circle(at.x, at.y, 7.0).class(body.clone()));
                group.push(
                    Element::circle(at.x - 3.0, at.y + 2.0, 2.5)
                        .class(body.clone())
                        .attr("fill", eye),
                );
                group.push(
                    Element::circle(at.x + 3.0, at.y + 2.0, 2.5)
                        .class(body.clone())
                        .attr("fill", eye),
                );
                group.push(
                    Element::line(at.x + 3.0, at.y + 1.0, at.x, at.y - 5.0)
                        .class(body.clone())
                        .attr("stroke", eye),
                );
                group.push(
                    Element::line(at.x - 3.0, at.y + 1.0, at.x, at.y - 5.0)
                        .class(body)
                        .attr("stroke", eye),
                );
            }
            _ => {
                group.push(
                    Element::circle(at.x, at.y, 7.0)
                        .class(format!("commit {id} commit{colour}"))
                        .attr("stroke", border.clone())
                        .attr("fill", border.clone()),
                );
                if symbol == CommitType::Merge {
                    group.push(
                        Element::circle(at.x, at.y, 5.0)
                            .class(format!("commit {type_class} {id} commit{colour}"))
                            .attr("stroke", bkg)
                            .attr("fill", bkg),
                    );
                }
                if symbol == CommitType::Reverse {
                    let c = 4.0;
                    let d = format!(
                        "M {},{}L{},{}M{},{}L{},{}",
                        crate::svg::number(at.x - c),
                        crate::svg::number(at.y - c),
                        crate::svg::number(at.x + c),
                        crate::svg::number(at.y + c),
                        crate::svg::number(at.x - c),
                        crate::svg::number(at.y + c),
                        crate::svg::number(at.x + c),
                        crate::svg::number(at.y - c),
                    );
                    let bounds = BBox::new(at.x - c, at.y - c, 2.0 * c, 2.0 * c);
                    group.push(
                        Element::path(d, bounds)
                            .class(format!("commit {type_class} {id} commit{colour}"))
                            .attr("stroke", bkg)
                            .attr("fill", bkg),
                    );
                }
            }
        }
    }

    /// `drawCommitLabel`: the commit's id, in a box, rotated when `rotateCommitLabel` is set.
    fn draw_label(
        &self,
        group: &mut Element,
        commit: &GitGraphCommit,
        at: CommitOffset,
        pos: f64,
    ) -> Result<(), RenderError> {
        let shown = commit.kind != CommitType::CherryPick
            && ((commit.custom_id == Some(true) && commit.kind == CommitType::Merge)
                || commit.kind != CommitType::Merge)
            && self.config.show_commit_label;
        if !shown {
            return Ok(());
        }
        let size = self.palette.commit_label_font_size;
        let width = text::width(text::bold(), &commit.id, size)?;
        let height = text::line_height(text::bold(), size)?;
        let vertical = self.vertical();
        let (bkg_x, bkg_y, text_x, text_y) = if vertical {
            (
                at.x - (width + 4.0 * PX + 5.0),
                at.y - 12.0,
                at.x - (width + 4.0 * PX),
                at.y + height - 12.0,
            )
        } else {
            (
                at.pos_with_offset - width / 2.0 - PY,
                at.y + 13.5,
                at.pos_with_offset - width / 2.0,
                at.y + 25.0,
            )
        };
        let rotate = self.config.rotate_commit_label;
        let rotation = Transform::rotate(-45.0, at.x, at.y);
        let mut bkg = Element::rect(bkg_x, bkg_y, width + 2.0 * PY, height + 2.0 * PY)
            .class("commit-label-bkg")
            .attr("fill", "transparent");
        let text_box = text::line_box(text::bold(), size, text_x, text_y, width)?;
        let mut label = Element::text(&commit.id, text_box)
            .attr("x", crate::svg::number(text_x))
            .attr("y", crate::svg::number(text_y))
            .class("commit-label")
            .attr("fill", self.palette.node_border());
        let mut wrapper = Element::group();
        if vertical {
            if rotate {
                bkg = bkg.transform(rotation.clone());
                label = label.transform(rotation);
            }
        } else if rotate {
            let r_x = -7.5 - ((width + 10.0) / 25.0) * 9.5;
            let r_y = 10.0 + (width / 25.0) * 8.5;
            wrapper = wrapper.transform(
                Transform::translate(r_x, r_y).then(Transform::rotate(-45.0, pos, at.y)),
            );
        }
        wrapper.push(bkg);
        wrapper.push(label);
        group.push(wrapper);
        Ok(())
    }

    /// `drawCommitTags`: the tags of a commit, stacked above it (or beside it in a vertical graph).
    /// The reference reverses the tags, so the last one is nearest the commit.
    fn draw_tags(
        &self,
        group: &mut Element,
        commit: &GitGraphCommit,
        at: CommitOffset,
        pos: f64,
    ) -> Result<(), RenderError> {
        if commit.tags.is_empty() {
            return Ok(());
        }
        let face: &'static Face = text::regular();
        let size = self.palette.tag_label_font_size;
        let mut widths = Vec::with_capacity(commit.tags.len());
        let mut max_width = 0.0_f64;
        let mut max_height = 0.0_f64;
        for tag in commit.tags.iter().rev() {
            let width = text::width(face, tag, size)?;
            max_width = max_width.max(width);
            max_height = max_height.max(text::line_height(face, size)?);
            widths.push(width);
        }
        let half_height = max_height / 2.0;
        let vertical = self.vertical();
        for (slot, (tag, width)) in commit.tags.iter().rev().zip(widths).enumerate() {
            let y_offset = 20.0 * slot as f64;
            let ly = at.y - 19.2 - y_offset;
            let text_baseline = at.y - 16.0 - y_offset;
            let (points, hole_x, hole_y, text_x, text_y) = if vertical {
                let origin = pos + y_offset;
                (
                    vec![
                        Point::new(at.x, origin + PY),
                        Point::new(at.x, origin - PY),
                        Point::new(at.x + LAYOUT_OFFSET, origin - half_height - PY),
                        Point::new(
                            at.x + LAYOUT_OFFSET + max_width + 4.0,
                            origin - half_height - PY,
                        ),
                        Point::new(
                            at.x + LAYOUT_OFFSET + max_width + 4.0,
                            origin + half_height + PY,
                        ),
                        Point::new(at.x + LAYOUT_OFFSET, origin + half_height + PY),
                    ],
                    at.x + PX / 2.0,
                    origin,
                    at.x + 5.0,
                    origin + 3.0,
                )
            } else {
                (
                    vec![
                        Point::new(pos - max_width / 2.0 - PX / 2.0, ly + PY),
                        Point::new(pos - max_width / 2.0 - PX / 2.0, ly - PY),
                        Point::new(
                            at.pos_with_offset - max_width / 2.0 - PX,
                            ly - half_height - PY,
                        ),
                        Point::new(
                            at.pos_with_offset + max_width / 2.0 + PX,
                            ly - half_height - PY,
                        ),
                        Point::new(
                            at.pos_with_offset + max_width / 2.0 + PX,
                            ly + half_height + PY,
                        ),
                        Point::new(
                            at.pos_with_offset - max_width / 2.0 - PX,
                            ly + half_height + PY,
                        ),
                    ],
                    pos - max_width / 2.0 + PX / 2.0,
                    ly,
                    at.pos_with_offset - width / 2.0,
                    text_baseline,
                )
            };
            let rotation =
                Transform::translate(12.0, 12.0).then(Transform::rotate(45.0, at.x, pos));
            let text_rotation =
                Transform::translate(14.0, 14.0).then(Transform::rotate(45.0, at.x, pos));
            let mut polygon = Element::polygon(&points)
                .class("tag-label-bkg")
                .attr("fill", self.palette.main_bkg())
                .attr("stroke", self.palette.node_border());
            let mut hole = Element::circle(hole_x, hole_y, 1.5)
                .class("tag-hole")
                .attr("fill", self.palette.text_color());
            let box_of_text = text::line_box(face, size, text_x, text_y, width)?;
            let mut label = Element::text(tag, box_of_text)
                .attr("x", crate::svg::number(text_x))
                .attr("y", crate::svg::number(text_y))
                .class("tag-label")
                .attr("fill", self.palette.tag_label_color());
            if vertical {
                polygon = polygon.transform(rotation.clone());
                hole = hole.transform(rotation);
                label = label.transform(text_rotation);
            }
            group.push(polygon);
            group.push(hole);
            group.push(label);
        }
        Ok(())
    }

    /// `drawArrows`: one arrow from each parent to each commit, in the order of the commits.
    pub(super) fn draw_arrows(&mut self) -> Result<Element, RenderError> {
        let commits = self.commits;
        let mut group = Element::group().class("commit-arrows");
        for (index, commit) in commits.iter().enumerate() {
            for parent in &commit.parents {
                let source = *self.commit_index.get(parent.as_str()).ok_or_else(|| {
                    RenderError::Layout(format!("parent {parent} of {} not found", commit.id))
                })?;
                self.draw_arrow(&mut group, source, index)?;
            }
        }
        Ok(group)
    }

    /// `drawArrow`: the path from commit `source` to commit `target`, rerouted through a lane when
    /// another commit lies between them on the curve's branch.
    fn draw_arrow(
        &mut self,
        group: &mut Element,
        source: usize,
        target: usize,
    ) -> Result<(), RenderError> {
        let commits = self.commits;
        let a = &commits[source];
        let b = &commits[target];
        let p1 = *self.commit_pos.get(a.id.as_str()).ok_or_else(|| {
            RenderError::Layout(format!("commit position not found for {}", a.id))
        })?;
        let p2 = *self.commit_pos.get(b.id.as_str()).ok_or_else(|| {
            RenderError::Layout(format!("commit position not found for {}", b.id))
        })?;
        let reroute = self.should_reroute(a, b, p1, p2);
        let merge_from_other = b.kind == CommitType::Merge && b.parents.first() != Some(&a.id);
        let mut colour_source = if merge_from_other { a } else { b };

        let dir = self.dir;
        let (d, bounds) = if reroute {
            let radius = 10.0;
            let offset = 10.0;
            let line_y = if p1.y < p2.y {
                self.find_lane(p1.y, p2.y, 0)
            } else {
                self.find_lane(p2.y, p1.y, 0)
            };
            let line_x = if p1.x < p2.x {
                self.find_lane(p1.x, p2.x, 0)
            } else {
                self.find_lane(p2.x, p1.x, 0)
            };
            let builder = match dir {
                Direction::TB => {
                    if p1.x < p2.x {
                        PathBuilder::new(p1)
                            .line(Point::new(line_x - radius, p1.y))
                            .arc(radius, true, Point::new(line_x, p1.y + offset))
                            .line(Point::new(line_x, p2.y - radius))
                            .arc(radius, false, Point::new(line_x + offset, p2.y))
                            .line(p2)
                    } else {
                        colour_source = a;
                        PathBuilder::new(p1)
                            .line(Point::new(line_x + radius, p1.y))
                            .arc(radius, false, Point::new(line_x, p1.y + offset))
                            .line(Point::new(line_x, p2.y - radius))
                            .arc(radius, true, Point::new(line_x - offset, p2.y))
                            .line(p2)
                    }
                }
                Direction::BT => {
                    if p1.x < p2.x {
                        PathBuilder::new(p1)
                            .line(Point::new(line_x - radius, p1.y))
                            .arc(radius, false, Point::new(line_x, p1.y - offset))
                            .line(Point::new(line_x, p2.y + radius))
                            .arc(radius, true, Point::new(line_x + offset, p2.y))
                            .line(p2)
                    } else {
                        colour_source = a;
                        PathBuilder::new(p1)
                            .line(Point::new(line_x + radius, p1.y))
                            .arc(radius, true, Point::new(line_x, p1.y - offset))
                            .line(Point::new(line_x, p2.y + radius))
                            .arc(radius, false, Point::new(line_x - offset, p2.y))
                            .line(p2)
                    }
                }
                Direction::LR => {
                    if p1.y < p2.y {
                        PathBuilder::new(p1)
                            .line(Point::new(p1.x, line_y - radius))
                            .arc(radius, false, Point::new(p1.x + offset, line_y))
                            .line(Point::new(p2.x - radius, line_y))
                            .arc(radius, true, Point::new(p2.x, line_y + offset))
                            .line(p2)
                    } else {
                        colour_source = a;
                        PathBuilder::new(p1)
                            .line(Point::new(p1.x, line_y + radius))
                            .arc(radius, true, Point::new(p1.x + offset, line_y))
                            .line(Point::new(p2.x - radius, line_y))
                            .arc(radius, false, Point::new(p2.x, line_y - offset))
                            .line(p2)
                    }
                }
            };
            builder.finish()
        } else {
            straight_arrow(self.dir, p1, p2, merge_from_other)
        };
        let colour = Self::colour_index(
            self.lane(&colour_source.branch)?.index,
            COMMIT_COLOUR_LIMIT,
            true,
        );
        group.push(
            Element::path(d, bounds)
                .class(format!("arrow arrow{colour}"))
                .attr("stroke", self.palette.border(colour))
                .attr("fill", "none"),
        );
        Ok(())
    }

    /// `shouldRerouteArrow`: an arrow is rerouted when a commit between the two (in creation order)
    /// is on the branch whose curve the arrow takes.
    fn should_reroute(&self, a: &GitGraphCommit, b: &GitGraphCommit, p1: Point, p2: Point) -> bool {
        let b_is_furthest = if self.vertical() {
            p1.x < p2.x
        } else {
            p1.y < p2.y
        };
        let curve_branch = if b_is_furthest { &b.branch } else { &a.branch };
        self.commits
            .iter()
            .any(|x| x.seq > a.seq && x.seq < b.seq && &x.branch == curve_branch)
    }

    /// `findLane`: a free lane between two positions, at the middle of their gap, moved towards the
    /// second one until it is at least 10 away from every lane taken (at most six tries).
    fn find_lane(&mut self, y1: f64, y2: f64, depth: u32) -> f64 {
        let candidate = y1 + (y1 - y2).abs() / 2.0;
        if depth > 5 {
            return candidate;
        }
        if self
            .lanes
            .iter()
            .all(|lane| (lane - candidate).abs() >= 10.0)
        {
            self.lanes.push(candidate);
            return candidate;
        }
        let diff = (y1 - y2).abs();
        self.find_lane(y1, y2 - diff / 5.0, depth + 1)
    }

    /// `drawBranches`: the line of each branch, and its label, with the lanes they take.
    pub(super) fn draw_branches(&mut self) -> Result<Element, RenderError> {
        let mut group = Element::group();
        let rotate = self.config.rotate_commit_label;
        let font_size = self.palette.font_size;
        let neo = self.neo;
        let id = self.id.to_owned();
        let branches = self.branches.clone();
        for (index, branch) in branches.iter().enumerate() {
            let adjust = Self::colour_index(index, self.palette.colour_limit, true);
            let pos = self.lane(&branch.name)?.pos;
            let spine_y = if self.vertical() {
                pos
            } else {
                pos + LABEL_PADDING_Y / 2.0 + 1.0
            };
            let (x1, y1, x2, y2) = match self.dir {
                Direction::TB => (pos, DEFAULT_POS, pos, self.max_pos),
                Direction::BT => (pos, self.max_pos, pos, DEFAULT_POS),
                Direction::LR => (0.0, spine_y, self.max_pos, spine_y),
            };
            group.push(
                Element::line(x1, y1, x2, y2)
                    .class(format!("branch branch{adjust}"))
                    .attr("stroke", self.palette.line()),
            );
            self.lanes.push(spine_y);

            let (label, bbox) = text::draw_text(&branch.name, text::bold(), font_size)?;
            let rotate_offset = if rotate { 30.0 } else { 0.0 };
            let box_width = bbox.width + 18.0 + LABEL_PADDING_X;
            let box_height = bbox.height + 4.0 + LABEL_PADDING_Y;
            let style = if neo {
                format!("filter:url(#{id}-drop-shadow)")
            } else {
                String::new()
            };
            let (bkg_x, bkg_y, bkg_transform, label_transform) = match self.dir {
                Direction::TB => (
                    pos - bbox.width / 2.0 - 10.0,
                    0.0,
                    Transform::translate(-LABEL_PADDING_X / 2.0 - 3.0, -LABEL_PADDING_Y - 10.0),
                    Transform::translate(
                        pos - bbox.width / 2.0 - 5.0,
                        -LABEL_PADDING_Y * 2.0 + 7.0,
                    ),
                ),
                Direction::BT => (
                    pos - bbox.width / 2.0 - 10.0,
                    self.max_pos,
                    Transform::translate(-LABEL_PADDING_X / 2.0 - 3.0, LABEL_PADDING_Y + 10.0),
                    Transform::translate(
                        pos - bbox.width / 2.0 - 5.0,
                        self.max_pos + LABEL_PADDING_Y * 2.0 + 4.0,
                    ),
                ),
                Direction::LR => (
                    -bbox.width - 4.0 - rotate_offset,
                    -bbox.height / 2.0 + 10.0,
                    Transform::translate(-19.0, spine_y - 12.0 - LABEL_PADDING_Y / 2.0),
                    Transform::translate(
                        -bbox.width - 14.0 - rotate_offset + LABEL_PADDING_X / 2.0,
                        spine_y - bbox.height / 2.0 - 2.0,
                    ),
                ),
            };
            let mut bkg = Element::rect(bkg_x, bkg_y, box_width, box_height)
                .class(format!("branchLabelBkg label{adjust}"))
                .attr("style", style)
                .attr("rx", "0")
                .attr("ry", "0")
                .transform(bkg_transform);
            if neo {
                bkg = bkg.attr("data-look", "neo");
            }
            let label_group = Element::group()
                .class(format!("label branch-label{adjust}"))
                .transform(label_transform)
                .with(label);
            let branch_label = Element::group().class("branchLabel").with(label_group);
            group.push(bkg);
            group.push(branch_label);
        }
        Ok(group)
    }
}

/// The arrows that are not rerouted: a rounded corner of radius 20 between the two commits, or a
/// straight line when they share a lane. `merge_from_other` is a merge whose arrow comes from
/// another branch than its first parent's, which takes the other corner.
fn straight_arrow(dir: Direction, p1: Point, p2: Point, merge_from_other: bool) -> (String, BBox) {
    let radius = 20.0;
    let offset = 20.0;
    let merge2 = merge_from_other;
    let vertical = matches!(dir, Direction::TB | Direction::BT);
    let builder = if vertical && p1.x == p2.x || !vertical && p1.y == p2.y {
        PathBuilder::new(p1).line(p2)
    } else if dir == Direction::TB {
        if p1.x < p2.x {
            if merge2 {
                PathBuilder::new(p1)
                    .line(Point::new(p1.x, p2.y - radius))
                    .arc(radius, false, Point::new(p1.x + offset, p2.y))
                    .line(p2)
            } else {
                PathBuilder::new(p1)
                    .line(Point::new(p2.x - radius, p1.y))
                    .arc(radius, true, Point::new(p2.x, p1.y + offset))
                    .line(p2)
            }
        } else if merge2 {
            PathBuilder::new(p1)
                .line(Point::new(p1.x, p2.y - radius))
                .arc(radius, true, Point::new(p1.x - offset, p2.y))
                .line(p2)
        } else {
            PathBuilder::new(p1)
                .line(Point::new(p2.x + radius, p1.y))
                .arc(radius, false, Point::new(p2.x, p1.y + offset))
                .line(p2)
        }
    } else if dir == Direction::BT {
        if p1.x < p2.x {
            if merge2 {
                PathBuilder::new(p1)
                    .line(Point::new(p1.x, p2.y + radius))
                    .arc(radius, true, Point::new(p1.x + offset, p2.y))
                    .line(p2)
            } else {
                PathBuilder::new(p1)
                    .line(Point::new(p2.x - radius, p1.y))
                    .arc(radius, false, Point::new(p2.x, p1.y - offset))
                    .line(p2)
            }
        } else if merge2 {
            PathBuilder::new(p1)
                .line(Point::new(p1.x, p2.y + radius))
                .arc(radius, false, Point::new(p1.x - offset, p2.y))
                .line(p2)
        } else {
            PathBuilder::new(p1)
                .line(Point::new(p2.x + radius, p1.y))
                .arc(radius, true, Point::new(p2.x, p1.y - offset))
                .line(p2)
        }
    } else if p1.y < p2.y {
        if merge2 {
            PathBuilder::new(p1)
                .line(Point::new(p2.x - radius, p1.y))
                .arc(radius, true, Point::new(p2.x, p1.y + offset))
                .line(p2)
        } else {
            PathBuilder::new(p1)
                .line(Point::new(p1.x, p2.y - radius))
                .arc(radius, false, Point::new(p1.x + offset, p2.y))
                .line(p2)
        }
    } else if merge2 {
        PathBuilder::new(p1)
            .line(Point::new(p2.x - radius, p1.y))
            .arc(radius, false, Point::new(p2.x, p1.y - offset))
            .line(p2)
    } else {
        PathBuilder::new(p1)
            .line(Point::new(p1.x, p2.y + radius))
            .arc(radius, true, Point::new(p1.x + offset, p2.y))
            .line(p2)
    };
    builder.finish()
}

/// The class of a commit's symbol: `getCommitClassType`.
fn type_class(symbol: CommitType) -> &'static str {
    match symbol {
        CommitType::Normal => "commit-normal",
        CommitType::Reverse => "commit-reverse",
        CommitType::Highlight => "commit-highlight",
        CommitType::Merge => "commit-merge",
        CommitType::CherryPick => "commit-cherry-pick",
    }
}

#[cfg(test)]
mod tests {
    //! The in-source renderer tests of `gitGraphRenderer.ts` (lines 1095 to 1489), ported with the
    //! reference's names. The reference's LR block of commit positions expects the TB numbers,
    //! because its `dir` is shared state that the describe blocks set while they are collected;
    //! the LR block here checks the LR geometry instead.

    use ferrigura_model::value::Value;

    use super::*;

    fn commit(
        id: &str,
        seq: u64,
        branch: &str,
        kind: CommitType,
        parents: &[&str],
    ) -> GitGraphCommit {
        GitGraphCommit {
            id: id.to_owned(),
            seq,
            branch: branch.to_owned(),
            kind,
            custom_type: None,
            custom_id: (kind == CommitType::Merge).then_some(true),
            auto_id: false,
            message: String::new(),
            tags: Vec::new(),
            parents: parents.iter().map(|parent| (*parent).to_owned()).collect(),
        }
    }

    fn diagram(direction: Direction, commits: Vec<GitGraphCommit>) -> GitGraphDiagram {
        let mut diagram = GitGraphDiagram::new();
        diagram.direction = direction;
        diagram.commits = commits;
        diagram
    }

    /// The commits of the reference's commit-position and class-type tests.
    fn fixture() -> Vec<GitGraphCommit> {
        vec![
            commit("ZERO", 0, "main", CommitType::Normal, &[]),
            commit("A", 1, "feature", CommitType::Normal, &["ZERO"]),
            commit("B", 2, "feature", CommitType::Normal, &["A"]),
            commit("M", 3, "main", CommitType::Merge, &["ZERO", "B"]),
            commit("C", 4, "release", CommitType::Normal, &["ZERO"]),
            commit(
                "5-8928ea0",
                5,
                "release",
                CommitType::CherryPick,
                &["C", "M"],
            ),
            commit("D", 6, "release", CommitType::Normal, &["5-8928ea0"]),
            commit(
                "7-ed848ba",
                7,
                "release",
                CommitType::CherryPick,
                &["D", "M"],
            ),
        ]
    }

    fn lanes(layout: &mut Layout<'_>, lanes: &[(&'static str, f64)]) {
        for (index, (name, pos)) in lanes.iter().enumerate() {
            layout
                .branch_pos
                .insert(name, BranchPos { pos: *pos, index });
        }
    }

    fn assert_position(actual: CommitOffset, x: f64, y: f64, pos_with_offset: f64) {
        assert_eq!(
            (actual.x, actual.y, actual.pos_with_offset),
            (x, y, pos_with_offset)
        );
    }

    #[test]
    fn should_set_branch_positions_lr_with_two_branches() {
        let diagram = diagram(Direction::LR, Vec::new());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);

        let pos = layout.set_branch_position("main", 0.0, 0, 10.0, true);
        assert_eq!(pos, 90.0);
        assert_eq!(
            layout.branch_pos.get("main").map(|b| (b.pos, b.index)),
            Some((0.0, 0))
        );
        let next = layout.set_branch_position("develop", pos, 1, 10.0, true);
        assert_eq!(next, 180.0);
        assert_eq!(
            layout.branch_pos.get("develop").map(|b| (b.pos, b.index)),
            Some((pos, 1))
        );
    }

    #[test]
    fn should_set_branch_positions_tb_with_two_branches() {
        let diagram = diagram(Direction::TB, Vec::new());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);

        let pos = layout.set_branch_position("main", 0.0, 0, 34.9921875, true);
        assert_eq!(pos, 107.49609375);
        assert_eq!(
            layout.branch_pos.get("main").map(|b| (b.pos, b.index)),
            Some((0.0, 0))
        );
        let next = layout.set_branch_position("develop", pos, 1, 56.421875, true);
        assert_eq!(next, 225.70703125);
        assert_eq!(
            layout.branch_pos.get("develop").map(|b| (b.pos, b.index)),
            Some((pos, 1))
        );
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_zero() {
        check_tb_position(0);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_a() {
        check_tb_position(1);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_b() {
        check_tb_position(2);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_m() {
        check_tb_position(3);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_c() {
        check_tb_position(4);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit5_8928ea0() {
        check_tb_position(5);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit_d() {
        check_tb_position(6);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_commit7_ed848ba() {
        check_tb_position(7);
    }

    /// `getCommitPosition` in TB for commit `index` of the fixture, at 30 plus 50 per commit.
    fn check_tb_position(index: usize) {
        let diagram = diagram(Direction::TB, fixture());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);
        lanes(
            &mut layout,
            &[
                ("main", 0.0),
                ("feature", 107.49609375),
                ("release", 224.03515625),
            ],
        );
        let (x, y) = [
            (0.0, 40.0),
            (107.49609375, 90.0),
            (107.49609375, 140.0),
            (0.0, 190.0),
            (224.03515625, 240.0),
            (224.03515625, 290.0),
            (224.03515625, 340.0),
            (224.03515625, 390.0),
        ][index];
        let position = layout
            .commit_position(index, 30.0 + 50.0 * index as f64, false)
            .unwrap();
        assert_position(position, x, y, y);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_zero_in_lr() {
        check_lr_position(0);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_a_in_lr() {
        check_lr_position(1);
    }

    #[test]
    fn should_give_the_correct_position_for_commit_m_in_lr() {
        check_lr_position(3);
    }

    /// `getCommitPosition` in LR: the time runs along x, and y is the lane's plus the redux offset.
    fn check_lr_position(index: usize) {
        let diagram = diagram(Direction::LR, fixture());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);
        lanes(
            &mut layout,
            &[
                ("main", 0.0),
                ("feature", 107.49609375),
                ("release", 224.03515625),
            ],
        );
        let lane = [0.0, 107.49609375, 107.49609375, 0.0][index];
        let pos = 30.0 + 50.0 * index as f64;
        let position = layout.commit_position(index, pos, false).unwrap();
        assert_position(position, pos + 10.0, lane + 7.0, pos + 10.0);
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_zero() {
        check_class(0, "commit-normal");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_a() {
        check_class(1, "commit-normal");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_b() {
        check_class(2, "commit-normal");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_m() {
        check_class(3, "commit-merge");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_c() {
        check_class(4, "commit-normal");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit5_8928ea0() {
        check_class(5, "commit-cherry-pick");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit_d() {
        check_class(6, "commit-normal");
    }

    #[test]
    fn should_give_the_correct_class_type_for_commit_commit7_ed848ba() {
        check_class(7, "commit-cherry-pick");
    }

    /// `getCommitClassType`: the class of the commit's symbol, its custom type first.
    fn check_class(index: usize, expected: &str) {
        let commits = fixture();
        let commit = &commits[index];
        assert_eq!(
            type_class(commit.custom_type.unwrap_or(commit.kind)),
            expected
        );
    }

    /// The commits of the BT parallel fixture, in creation order.
    fn parallel_fixture() -> Vec<GitGraphCommit> {
        vec![
            commit("1-abcdefg", 0, "main", CommitType::Normal, &[]),
            commit("2-abcdefg", 1, "main", CommitType::Normal, &["1-abcdefg"]),
            commit(
                "3-abcdefg",
                2,
                "develop",
                CommitType::Normal,
                &["2-abcdefg"],
            ),
            commit(
                "4-abcdefg",
                3,
                "develop",
                CommitType::Normal,
                &["3-abcdefg"],
            ),
            commit(
                "5-abcdefg",
                4,
                "feature",
                CommitType::Normal,
                &["2-abcdefg"],
            ),
            commit(
                "6-abcdefg",
                5,
                "feature",
                CommitType::Normal,
                &["5-abcdefg"],
            ),
            commit("7-abcdefg", 6, "main", CommitType::Normal, &["2-abcdefg"]),
            commit("8-abcdefg", 7, "main", CommitType::Normal, &["7-abcdefg"]),
        ]
    }

    #[test]
    fn should_get_the_correct_commit_position_and_current_position() {
        let diagram = diagram(Direction::BT, parallel_fixture());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let commits = parallel_fixture();
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);
        lanes(
            &mut layout,
            &[
                ("main", 0.0),
                ("develop", 107.49609375),
                ("feature", 225.70703125),
            ],
        );
        let expected_position = [
            (0.0, 40.0),
            (0.0, 90.0),
            (107.49609375, 140.0),
            (107.49609375, 190.0),
            (225.70703125, 140.0),
            (225.70703125, 190.0),
            (0.0, 140.0),
            (0.0, 190.0),
        ];
        let expected_current = [30.0, 80.0, 130.0, 180.0, 130.0, 180.0, 130.0, 180.0];
        let mut cur_pos = 30.0;
        for (index, commit) in commits.iter().enumerate() {
            if !commit.parents.is_empty() {
                cur_pos = layout.calculate_commit_position(index).unwrap();
            }
            layout.set_commit_position(index, cur_pos).unwrap();
            let position = layout.commit_pos[commit.id.as_str()];
            assert_eq!((position.x, position.y), expected_position[index]);
            assert_eq!(cur_pos, expected_current[index]);
        }
    }

    #[test]
    fn should_get_the_correct_commit_position_after_parallel_commits() {
        let diagram = diagram(Direction::BT, parallel_fixture());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);
        lanes(
            &mut layout,
            &[
                ("main", 0.0),
                ("develop", 107.49609375),
                ("feature", 225.70703125),
            ],
        );
        let sorted: Vec<usize> = (0..8).collect();
        layout.set_parallel_bt_pos(&sorted, 30.0).unwrap();
        let expected = [
            (0.0, 210.0),
            (0.0, 160.0),
            (107.49609375, 110.0),
            (107.49609375, 60.0),
            (225.70703125, 110.0),
            (225.70703125, 60.0),
            (0.0, 110.0),
            (0.0, 60.0),
        ];
        for (commit, (x, y)) in parallel_fixture().iter().zip(expected) {
            let position = layout.commit_pos[commit.id.as_str()];
            assert_eq!((position.x, position.y), (x, y));
        }
    }

    #[test]
    fn add() {
        let diagram = diagram(Direction::LR, Vec::new());
        let config = GitGraphDiagramConfig::default();
        let palette = Palette::new(&Value::empty_object(), false);
        let mut layout = Layout::new(&diagram, &config, &palette, "id", false);
        layout.commit_pos.insert("parent1", Point::new(1.0, 1.0));
        layout.commit_pos.insert("parent2", Point::new(2.0, 2.0));
        layout.commit_pos.insert("parent3", Point::new(3.0, 3.0));
        let parents: Vec<String> = ["parent1", "parent2", "parent3"]
            .iter()
            .map(|parent| (*parent).to_owned())
            .collect();
        assert_eq!(layout.closest_parent(&parents), Some("parent3"));
    }
}
