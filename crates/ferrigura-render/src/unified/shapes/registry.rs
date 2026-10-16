//! The shape registry: the names and aliases of `rendering-elements/shapes.ts` that the flowchart
//! reaches, mapped to the shapes of stages A and C. A name that the registry does not know is an
//! error, as `insertNode` throws `No such shape` for it. The names of the other stages are not in
//! the registry yet, so they are reported the same way until their stage ports them.

use crate::unified::data::Node;

use super::util;

/// The shapes of stages A and C, one per handler of `shapes/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    SquareRect,
    RoundedRect,
    Stadium,
    Question,
    Circle,
    DoubleCircle,
    Hexagon,
    LeanRight,
    LeanLeft,
    Trapezoid,
    InvTrapezoid,
    Subroutine,
    Cylinder,
    RectLeftInvArrow,
    Text,
    FilledCircle,
    ForkJoin,
    StateStart,
    StateEnd,
    Anchor,
    LabelRect,
    CollapsedGroup,
    // The shapes of stage C.
    Bang,
    BowTieRect,
    Browser,
    Bucket,
    Card,
    Choice,
    Cloud,
    Console,
    CrossedCircle,
    CurlyBraceLeft,
    CurlyBraceRight,
    CurlyBraces,
    CurvedTrapezoid,
    Datastore,
    DividedRectangle,
    FlippedTriangle,
    Folder,
    HalfRoundedRectangle,
    Hourglass,
    Note,
    Person,
    LightningBolt,
    LinedCylinder,
    LinedWaveEdgedRect,
    MultiRect,
    MultiWaveEdgedRectangle,
    ShadedProcess,
    SlopedRect,
    TaggedRect,
    TaggedWaveEdgedRectangle,
    TiltedCylinder,
    TrapezoidalPentagon,
    Triangle,
    WaveEdgedRectangle,
    WaveRectangle,
    WindowPane,
    Icon,
    IconCircle,
    IconRounded,
    IconSquare,
    ImageSquare,
}

/// The shape a name stands for, with `rect` taken as `roundedRect` when it has both radii
/// (`nodes.ts` 22-29).
pub(super) fn resolve_with_radii(name: &str, rounded: bool) -> Option<Kind> {
    Some(match name {
        "rect" => {
            if rounded {
                Kind::RoundedRect
            } else {
                Kind::SquareRect
            }
        }
        "squareRect" | "proc" | "process" | "rectangle" => Kind::SquareRect,
        "rounded" | "roundedRect" | "event" => Kind::RoundedRect,
        "stadium" | "terminal" | "pill" => Kind::Stadium,
        "fr-rect" | "subprocess" | "subproc" | "framed-rectangle" | "subroutine" => {
            Kind::Subroutine
        }
        "cyl" | "db" | "database" | "cylinder" => Kind::Cylinder,
        "circle" | "circ" => Kind::Circle,
        "diam" | "decision" | "diamond" | "question" => Kind::Question,
        "hex" | "hexagon" | "prepare" => Kind::Hexagon,
        "lean-r" | "lean-right" | "in-out" | "lean_right" => Kind::LeanRight,
        "lean-l" | "lean-left" | "out-in" | "lean_left" => Kind::LeanLeft,
        "trap-b" | "priority" | "trapezoid-bottom" | "trapezoid" => Kind::Trapezoid,
        "trap-t" | "manual" | "trapezoid-top" | "inv-trapezoid" | "inv_trapezoid" => {
            Kind::InvTrapezoid
        }
        "dbl-circ" | "double-circle" | "doublecircle" => Kind::DoubleCircle,
        "text" => Kind::Text,
        "f-circ" | "junction" | "filled-circle" => Kind::FilledCircle,
        "fork" | "join" | "forkJoin" => Kind::ForkJoin,
        "sm-circ" | "start" | "small-circle" | "stateStart" => Kind::StateStart,
        "fr-circ" | "stop" | "framed-circle" | "stateEnd" => Kind::StateEnd,
        "anchor" => Kind::Anchor,
        "odd" | "rect_left_inv_arrow" => Kind::RectLeftInvArrow,
        "labelRect" => Kind::LabelRect,
        "collapsedGroup" => Kind::CollapsedGroup,
        "bang" => Kind::Bang,
        "bow-rect" | "stored-data" | "bow-tie-rectangle" => Kind::BowTieRect,
        "browser" => Kind::Browser,
        "bucket" => Kind::Bucket,
        "notch-rect" | "card" | "notched-rectangle" => Kind::Card,
        "choice" => Kind::Choice,
        "cloud" => Kind::Cloud,
        "console" => Kind::Console,
        "cross-circ" | "summary" | "crossed-circle" => Kind::CrossedCircle,
        "brace" | "comment" | "brace-l" => Kind::CurlyBraceLeft,
        "brace-r" => Kind::CurlyBraceRight,
        "braces" => Kind::CurlyBraces,
        "curv-trap" | "curved-trapezoid" | "display" => Kind::CurvedTrapezoid,
        "datastore" | "data-store" => Kind::Datastore,
        "div-rect" | "div-proc" | "divided-rectangle" | "divided-process" => Kind::DividedRectangle,
        "flip-tri" | "manual-file" | "flipped-triangle" => Kind::FlippedTriangle,
        "folder" | "directory" => Kind::Folder,
        "delay" | "half-rounded-rectangle" => Kind::HalfRoundedRectangle,
        "hourglass" | "collate" => Kind::Hourglass,
        "note" => Kind::Note,
        "person" => Kind::Person,
        "bolt" | "com-link" | "lightning-bolt" => Kind::LightningBolt,
        "lin-cyl" | "disk" | "lined-cylinder" => Kind::LinedCylinder,
        "lin-doc" | "lined-document" => Kind::LinedWaveEdgedRect,
        "st-rect" | "procs" | "processes" | "stacked-rectangle" => Kind::MultiRect,
        "docs" | "documents" | "st-doc" | "stacked-document" => Kind::MultiWaveEdgedRectangle,
        "lin-rect" | "lined-rectangle" | "lined-process" | "lin-proc" | "shaded-process" => {
            Kind::ShadedProcess
        }
        "sl-rect" | "manual-input" | "sloped-rectangle" => Kind::SlopedRect,
        "tag-rect" | "tagged-rectangle" | "tag-proc" | "tagged-process" => Kind::TaggedRect,
        "tag-doc" | "tagged-document" => Kind::TaggedWaveEdgedRectangle,
        "h-cyl" | "das" | "horizontal-cylinder" => Kind::TiltedCylinder,
        "notch-pent" | "loop-limit" | "notched-pentagon" => Kind::TrapezoidalPentagon,
        "tri" | "extract" | "triangle" => Kind::Triangle,
        "doc" | "document" => Kind::WaveEdgedRectangle,
        "flag" | "paper-tape" => Kind::WaveRectangle,
        "win-pane" | "internal-storage" | "window-pane" => Kind::WindowPane,
        "icon" => Kind::Icon,
        "iconCircle" => Kind::IconCircle,
        "iconRounded" => Kind::IconRounded,
        "iconSquare" => Kind::IconSquare,
        "imageSquare" => Kind::ImageSquare,
        _ => return None,
    })
}

/// The shape a node's shape name stands for. The flowchart's nodes carry no radii, so `rect` is a
/// `squareRect` there.
pub(super) fn resolve(name: &str) -> Option<Kind> {
    resolve_with_radii(name, false)
}

impl Kind {
    /// Whether the shape draws a label. The shapes without one set the node's label to nothing
    /// or ignore it (`filledCircle`, `forkJoin`, `anchor`, `stateStart`, `stateEnd`, `bolt`), or
    /// do not call `labelHelper` (`choice`, `crossedCircle`).
    pub(super) fn draws_label(self) -> bool {
        !matches!(
            self,
            Self::FilledCircle
                | Self::ForkJoin
                | Self::Anchor
                | Self::StateStart
                | Self::StateEnd
                | Self::Choice
                | Self::CrossedCircle
                | Self::LightningBolt
        )
    }

    /// Whether the shape sets the node's label to nothing before it lays it out, so that its label
    /// group is empty whatever the node's text (`hourglass.ts` sets `node.label = ''`).
    pub(super) fn empties_label(self) -> bool {
        matches!(self, Self::Hourglass)
    }

    /// Whether the shape stores its label declarations on the node (`node.labelStyle`), which
    /// its label group then takes as its style. `collapsedGroup` and `labelRect` do not.
    pub(super) fn sets_label_style(self) -> bool {
        self.draws_label() && !matches!(self, Self::CollapsedGroup | Self::LabelRect)
    }

    /// The class of the node's `<g>`, as the shape's handler sets it.
    pub(super) fn group_class(self, node: &Node) -> String {
        match self {
            Self::StateStart | Self::StateEnd => "node default".to_owned(),
            Self::LabelRect => "label edgeLabel".to_owned(),
            // The icon and image handlers pass their own class to `labelHelper`.
            Self::Icon | Self::IconCircle | Self::IconRounded | Self::IconSquare => {
                "icon-shape default".to_owned()
            }
            Self::ImageSquare => "image-shape default".to_owned(),
            _ => util::node_classes(node),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // nodes.spec.ts, "Test Alias for shapes": every alias resolves to the shape of its short name.

    #[test]
    fn should_support_alias_for_rectangle_shape() {
        assert_eq!(resolve("process"), resolve("rect"));
        assert_eq!(resolve("proc"), resolve("rect"));
        assert_eq!(resolve("rectangle"), resolve("rect"));
    }

    #[test]
    fn should_support_alias_for_rounded_shape() {
        assert_eq!(resolve("event"), resolve("rounded"));
    }

    #[test]
    fn should_support_alias_for_stadium_shape() {
        assert_eq!(resolve("pill"), resolve("stadium"));
        assert_eq!(resolve("terminal"), resolve("stadium"));
    }

    #[test]
    fn should_support_alias_for_subroutine_shape() {
        assert_eq!(resolve("framed-rectangle"), resolve("fr-rect"));
        assert_eq!(resolve("subproc"), resolve("fr-rect"));
        assert_eq!(resolve("subroutine"), resolve("fr-rect"));
    }

    #[test]
    fn should_support_alias_for_cylinder_shape() {
        assert_eq!(resolve("db"), resolve("cylinder"));
        assert_eq!(resolve("cyl"), resolve("cylinder"));
    }

    #[test]
    fn should_support_alias_for_diamond_shape() {
        assert_eq!(resolve("diam"), resolve("decision"));
        assert_eq!(resolve("diamond"), resolve("decision"));
    }

    #[test]
    fn should_support_alias_for_hexagon_shape() {
        assert_eq!(resolve("hex"), resolve("hexagon"));
        assert_eq!(resolve("prepare"), resolve("hexagon"));
    }

    #[test]
    fn should_support_alias_for_lean_right_shape() {
        assert_eq!(resolve("lean-r"), resolve("lean-right"));
        assert_eq!(resolve("in-out"), resolve("lean-right"));
    }

    #[test]
    fn should_support_alias_for_lean_left_shape() {
        assert_eq!(resolve("lean-l"), resolve("lean-left"));
        assert_eq!(resolve("out-in"), resolve("lean-left"));
    }

    #[test]
    fn should_support_alias_for_trapezoid_shape() {
        assert_eq!(resolve("trapezoid-bottom"), resolve("trap-b"));
        assert_eq!(resolve("priority"), resolve("trap-b"));
        assert_eq!(resolve("trapezoid"), resolve("trap-b"));
    }

    #[test]
    fn should_support_alias_for_inv_trapezoid_shape() {
        assert_eq!(resolve("trapezoid-top"), resolve("trap-t"));
        assert_eq!(resolve("manual"), resolve("trap-t"));
        assert_eq!(resolve("inv-trapezoid"), resolve("trap-t"));
    }

    #[test]
    fn should_support_alias_for_doublecircle_shape() {
        assert_eq!(resolve("double-circle"), resolve("dbl-circ"));
    }

    #[test]
    fn should_support_alias_for_small_circle_shape() {
        assert_eq!(resolve("start"), resolve("sm-circ"));
        assert_eq!(resolve("small-circle"), resolve("sm-circ"));
    }

    #[test]
    fn should_support_alias_for_framed_circle_shape() {
        assert_eq!(resolve("stop"), resolve("fr-circ"));
        assert_eq!(resolve("framed-circle"), resolve("fr-circ"));
    }

    #[test]
    fn should_support_alias_for_fork_shape() {
        assert_eq!(resolve("join"), resolve("fork"));
    }

    #[test]
    fn should_support_alias_for_filledcircle_shape() {
        assert_eq!(resolve("junction"), resolve("f-circ"));
        assert_eq!(resolve("filled-circle"), resolve("f-circ"));
    }

    // shapes.ts: the aliases of the stage C shapes resolve to the shape of their short name.

    #[test]
    fn should_support_alias_for_stage_c_shapes() {
        assert_eq!(resolve("stored-data"), resolve("bow-rect"));
        assert_eq!(resolve("bow-tie-rectangle"), resolve("bow-rect"));
        assert_eq!(resolve("notched-rectangle"), resolve("notch-rect"));
        assert_eq!(resolve("comment"), resolve("brace"));
        assert_eq!(resolve("brace-l"), resolve("brace"));
        assert_eq!(resolve("summary"), resolve("cross-circ"));
        assert_eq!(resolve("crossed-circle"), resolve("cross-circ"));
        assert_eq!(resolve("display"), resolve("curv-trap"));
        assert_eq!(resolve("curved-trapezoid"), resolve("curv-trap"));
        assert_eq!(resolve("data-store"), resolve("datastore"));
        assert_eq!(resolve("div-proc"), resolve("div-rect"));
        assert_eq!(resolve("divided-process"), resolve("div-rect"));
        assert_eq!(resolve("manual-file"), resolve("flip-tri"));
        assert_eq!(resolve("directory"), resolve("folder"));
        assert_eq!(resolve("half-rounded-rectangle"), resolve("delay"));
        assert_eq!(resolve("collate"), resolve("hourglass"));
    }

    #[test]
    fn the_shapes_that_draw_no_label_are_known_to_the_registry() {
        for name in ["choice", "summary", "note", "folder", "bang"] {
            assert!(resolve(name).is_some(), "{name}");
        }
        assert!(!Kind::Choice.draws_label());
        assert!(!Kind::CrossedCircle.draws_label());
        assert!(Kind::Hourglass.draws_label());
        assert!(Kind::Hourglass.empties_label());
    }

    #[test]
    fn rect_with_both_radii_is_a_rounded_rectangle() {
        assert_eq!(resolve_with_radii("rect", true), Some(Kind::RoundedRect));
        assert_eq!(resolve_with_radii("rect", false), Some(Kind::SquareRect));
    }

    #[test]
    fn an_unknown_name_is_not_a_shape() {
        assert_eq!(resolve("no-such-shape"), None);
    }
}
