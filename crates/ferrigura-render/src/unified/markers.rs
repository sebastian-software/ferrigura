//! The arrow markers: the flowchart's `point`, `circle` and `cross` of `rendering-elements/markers.js`,
//! with their end and start variants and the neo `-margin` variants, and the coloured copies that
//! `edgeMarker.ts` makes for an edge with a stroke colour; and the cardinality markers of ER
//! diagrams (`only_one`, `zero_or_one`, `one_or_more`, `zero_or_more`, and their `_neo` sets,
//! `markers.js` 580-867).
//!
//! The colours of the default markers are left to the stylesheet (`.marker` and `.marker.cross`,
//! decision 0007), so the definitions carry no fill or stroke. A coloured copy carries its colour
//! as a presentation attribute, as the reference writes it: the colour is the edge's own, not a
//! theme token.

use crate::svg::{escape, number};

/// The diagram type that the reference puts in every marker id and class.
pub const FLOWCHART: &str = "flowchart-v2";

/// The diagram type of an ER diagram, in its marker ids and classes (`erDetector.ts`).
pub const ER: &str = "er";

/// The diagram type of a requirement diagram, in its marker ids (`requirementRenderer.ts` 19).
pub const REQUIREMENT: &str = "requirement";

/// The marker kinds an edge end can use: the flowchart's three in the order `markers.js` writes
/// them, and the four cardinalities of ER diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `arrow_point`: a filled triangle.
    Point,
    /// `arrow_circle`: a circle.
    Circle,
    /// `arrow_cross`: a cross.
    Cross,
    /// `only_one`: two bars (ER).
    OnlyOne,
    /// `zero_or_one`: a bar and a circle (ER).
    ZeroOrOne,
    /// `one_or_more`: a crow's foot and a bar (ER).
    OneOrMore,
    /// `zero_or_more`: a crow's foot and a circle (ER).
    ZeroOrMore,
    /// `arrow_barb` and `arrow_barb_neo`: the barb of the state diagram (`markers.js` `barb`).
    Barb,
    /// `requirement_arrow`: the open arrow of a requirement relationship (`markers.js` 869-958).
    RequirementArrow,
    /// `requirement_contains`: the circle with a cross of a `contains` relationship.
    RequirementContains,
}

impl Kind {
    /// The kind of an arrow type; `None` for `none` and for the types that have no marker.
    #[must_use]
    pub fn from_arrow(arrow: &str) -> Option<Self> {
        match arrow {
            "arrow_point" => Some(Self::Point),
            "arrow_circle" => Some(Self::Circle),
            "arrow_cross" => Some(Self::Cross),
            // `edgeMarker.ts` 51-54: the ER arrow types and their marker names.
            "only_one" => Some(Self::OnlyOne),
            "zero_or_one" => Some(Self::ZeroOrOne),
            "one_or_more" => Some(Self::OneOrMore),
            "zero_or_more" => Some(Self::ZeroOrMore),
            "arrow_barb" | "arrow_barb_neo" => Some(Self::Barb),
            // `edgeMarker.ts` 55-56: the requirement arrows of a requirement diagram.
            "requirement_arrow" => Some(Self::RequirementArrow),
            "requirement_contains" => Some(Self::RequirementContains),
            _ => None,
        }
    }

    /// The name of the marker type in the reference (`endMarkerType`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Circle => "circle",
            Self::Cross => "cross",
            Self::OnlyOne => "onlyOne",
            Self::ZeroOrOne => "zeroOrOne",
            Self::OneOrMore => "oneOrMore",
            Self::ZeroOrMore => "zeroOrMore",
            Self::Barb => "barb",
            Self::RequirementArrow => "requirement_arrow",
            Self::RequirementContains => "requirement_contains",
        }
    }

    /// Whether the reference's `arrowTypesMap` fills the marker with the stroke colour.
    #[must_use]
    pub const fn fills(self) -> bool {
        matches!(self, Self::Point | Self::Barb)
    }

    /// Whether the reference's `arrowTypesWithMarginSupport` lists the kind: every flowchart kind
    /// does, and no ER kind does (`edgeMarker.ts` 59-69).
    #[must_use]
    pub const fn has_margin(self) -> bool {
        // The ER cardinalities have no margin variant; the state diagram's barb has one.
        matches!(self, Self::Point | Self::Circle | Self::Cross | Self::Barb)
    }
}

/// The child element of a marker: a path, a polygon, or a circle of radius 5 at (5, 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Child {
    Path(&'static str),
    Polygon(&'static str),
    Circle,
}

/// One marker definition of `markers.js`.
struct Definition {
    /// The part of the id after `<diagram>-`, such as `pointEnd-margin`.
    name: &'static str,
    kind: Kind,
    view_box: &'static str,
    ref_x: &'static str,
    ref_y: &'static str,
    width: &'static str,
    height: &'static str,
    child: Child,
    stroke_width: &'static str,
    dash: bool,
}

/// Every marker of the flowchart, in the order `insertMarkers` writes them: `point`, `circle`,
/// then `cross`, each with its end, start, end-margin and start-margin variants.
const DEFINITIONS: [Definition; 12] = [
    Definition {
        name: "pointEnd",
        kind: Kind::Point,
        view_box: "0 0 10 10",
        ref_x: "5",
        ref_y: "5",
        width: "8",
        height: "8",
        child: Child::Path("M 0 0 L 10 5 L 0 10 z"),
        stroke_width: "1",
        dash: true,
    },
    Definition {
        name: "pointStart",
        kind: Kind::Point,
        view_box: "0 0 10 10",
        ref_x: "4.5",
        ref_y: "5",
        width: "8",
        height: "8",
        child: Child::Path("M 0 5 L 10 10 L 10 0 z"),
        stroke_width: "1",
        dash: true,
    },
    Definition {
        name: "pointEnd-margin",
        kind: Kind::Point,
        view_box: "0 0 11.5 14",
        ref_x: "11.5",
        ref_y: "7",
        width: "10.5",
        height: "14",
        child: Child::Path("M 0 0 L 11.5 7 L 0 14 z"),
        stroke_width: "0",
        dash: true,
    },
    Definition {
        name: "pointStart-margin",
        kind: Kind::Point,
        view_box: "0 0 11.5 14",
        ref_x: "1",
        ref_y: "7",
        width: "11.5",
        height: "14",
        child: Child::Polygon("0,7 11.5,14 11.5,0"),
        stroke_width: "0",
        dash: true,
    },
    Definition {
        name: "circleEnd",
        kind: Kind::Circle,
        view_box: "0 0 10 10",
        ref_x: "11",
        ref_y: "5",
        width: "11",
        height: "11",
        child: Child::Circle,
        stroke_width: "1",
        dash: true,
    },
    Definition {
        name: "circleStart",
        kind: Kind::Circle,
        view_box: "0 0 10 10",
        ref_x: "-1",
        ref_y: "5",
        width: "11",
        height: "11",
        child: Child::Circle,
        stroke_width: "1",
        dash: true,
    },
    Definition {
        name: "circleEnd-margin",
        kind: Kind::Circle,
        view_box: "0 0 10 10",
        ref_x: "12.25",
        ref_y: "5",
        width: "14",
        height: "14",
        child: Child::Circle,
        stroke_width: "0",
        dash: true,
    },
    Definition {
        name: "circleStart-margin",
        kind: Kind::Circle,
        view_box: "0 0 10 10",
        ref_x: "-2",
        ref_y: "5",
        width: "14",
        height: "14",
        child: Child::Circle,
        stroke_width: "0",
        dash: true,
    },
    Definition {
        name: "crossEnd",
        kind: Kind::Cross,
        view_box: "0 0 11 11",
        ref_x: "12",
        ref_y: "5.2",
        width: "11",
        height: "11",
        child: Child::Path("M 1,1 l 9,9 M 10,1 l -9,9"),
        stroke_width: "2",
        dash: true,
    },
    Definition {
        name: "crossStart",
        kind: Kind::Cross,
        view_box: "0 0 11 11",
        ref_x: "-1",
        ref_y: "5.2",
        width: "11",
        height: "11",
        child: Child::Path("M 1,1 l 9,9 M 10,1 l -9,9"),
        stroke_width: "2",
        dash: true,
    },
    Definition {
        name: "crossEnd-margin",
        kind: Kind::Cross,
        view_box: "0 0 15 15",
        ref_x: "17.7",
        ref_y: "7.5",
        width: "12",
        height: "12",
        child: Child::Path("M 1,1 L 14,14 M 1,14 L 14,1"),
        stroke_width: "2.5",
        dash: false,
    },
    Definition {
        name: "crossStart-margin",
        kind: Kind::Cross,
        view_box: "0 0 15 15",
        ref_x: "-3.5",
        ref_y: "7.5",
        width: "12",
        height: "12",
        child: Child::Path("M 1,1 L 14,14 M 1,14 L 14,1"),
        stroke_width: "2.5",
        dash: true,
    },
];

/// The id of a marker, as `edgeMarker.ts` builds it: `<svgId>_<diagramType>-<name>`, with the end
/// or start name of the kind and `-margin` for the neo variants.
#[must_use]
pub fn marker_id(
    svg_id: &str,
    diagram_type: &str,
    kind: Kind,
    start: bool,
    margin: bool,
) -> String {
    let position = if start { "Start" } else { "End" };
    let suffix = if margin && kind.has_margin() {
        "-margin"
    } else {
        ""
    };
    format!("{svg_id}_{diagram_type}-{}{position}{suffix}", kind.name())
}

/// The id of the copy of a marker that carries a stroke colour: the marker's id, then the colour
/// with every character outside `[0-9A-Za-z]` replaced by `_` (one per UTF-16 unit, as the
/// reference's regular expression does).
#[must_use]
pub fn coloured_id(marker_id: &str, colour: &str) -> String {
    let mut sanitised = String::new();
    for c in colour.chars() {
        if c.is_ascii_alphanumeric() {
            sanitised.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                sanitised.push('_');
            }
        }
    }
    format!("{marker_id}_{sanitised}")
}

fn definition_xml(
    id: &str,
    diagram_type: &str,
    definition: &Definition,
    colour: Option<&str>,
) -> String {
    let class = match definition.kind {
        Kind::Cross => format!("marker cross {diagram_type}"),
        _ => format!("marker {diagram_type}"),
    };
    let style = if definition.dash {
        format!(
            "stroke-width: {}; stroke-dasharray: 1, 0;",
            definition.stroke_width
        )
    } else {
        format!("stroke-width: {};", definition.stroke_width)
    };
    // A coloured copy puts the colour on the children that the reference's
    // `querySelectorAll('path, circle, line')` finds: paths and circles, not polygons. A point also
    // takes it as its fill.
    let paint = |fills: bool| -> String {
        match colour {
            Some(colour) => {
                let colour = escape(colour);
                let fill = if fills {
                    format!(" fill=\"{colour}\"")
                } else {
                    String::new()
                };
                format!(" stroke=\"{colour}\"{fill}")
            }
            None => String::new(),
        }
    };
    let child = match definition.child {
        Child::Path(d) => format!(
            "<path d=\"{d}\" class=\"arrowMarkerPath\" style=\"{style}\"{}/>",
            paint(definition.kind.fills())
        ),
        Child::Polygon(points) => {
            format!("<polygon points=\"{points}\" class=\"arrowMarkerPath\" style=\"{style}\"/>")
        }
        Child::Circle => format!(
            "<circle cx=\"5\" cy=\"5\" r=\"5\" class=\"arrowMarkerPath\" style=\"{style}\"{}/>",
            paint(false)
        ),
    };
    format!(
        "<marker id=\"{id}\" class=\"{class}\" viewBox=\"{}\" refX=\"{}\" refY=\"{}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{}\" markerHeight=\"{}\" orient=\"auto\">{child}</marker>",
        definition.view_box,
        definition.ref_x,
        definition.ref_y,
        definition.width,
        definition.height,
    )
}

/// The marker definitions of a flowchart, as `insertMarkers(element, ['point', 'circle', 'cross'])`
/// writes them: twelve markers, each with its id under the SVG's id.
#[must_use]
pub fn markers(svg_id: &str, diagram_type: &str) -> String {
    DEFINITIONS
        .iter()
        .map(|definition| {
            let id = format!("{svg_id}_{diagram_type}-{}", definition.name);
            definition_xml(&id, diagram_type, definition, None)
        })
        .collect()
}

/// The four `point` markers of a diagram, as `insertMarkers(element, ['point'], type, id)` writes them
/// (`markers.js` 323-390): the first four definitions of [`markers`], under the diagram type's own name
/// (`<svgId>_<type>-pointEnd`, and so on). The mindmap appends them to its root group.
#[must_use]
pub fn point_markers(svg_id: &str, diagram_type: &str) -> String {
    DEFINITIONS
        .iter()
        .take(4)
        .map(|definition| {
            let id = format!("{svg_id}_{diagram_type}-{}", definition.name);
            definition_xml(&id, diagram_type, definition, None)
        })
        .collect()
}

/// The barb markers of the state diagram, as `insertMarkers` writes them for the look: `barb` (the
/// classic look) or `barbNeo` (the neo look, with its margin variant), each in its own `defs` as
/// `markers.js` 534-578 writes them. The margin variant's fill is the theme's `transitionColor`.
/// The ids are `<svgId>_<diagramType>-barbEnd` and `-barbEnd-margin`.
#[must_use]
pub fn barb_markers(
    svg_id: &str,
    diagram_type: &str,
    neo: bool,
    transition_colour: &str,
) -> String {
    let id = format!("{svg_id}_{diagram_type}-barbEnd");
    if !neo {
        return format!(
            "<defs><marker id=\"{id}\" refX=\"19\" refY=\"7\" markerWidth=\"20\" markerHeight=\"14\" markerUnits=\"userSpaceOnUse\" orient=\"auto\"><path d=\"M 19,7 L9,13 L14,7 L9,1 Z\"></path></marker></defs>"
        );
    }
    format!(
        "<defs><marker id=\"{id}\" refX=\"19\" refY=\"7\" markerWidth=\"20\" markerHeight=\"14\" markerUnits=\"strokeWidth\" orient=\"auto\"><path d=\"M 19,7 L11,14 L13,7 L11,0 Z\"></path></marker></defs>\
         <defs><marker id=\"{id}-margin\" refX=\"17\" refY=\"7\" markerWidth=\"20\" markerHeight=\"14\" markerUnits=\"userSpaceOnUse\" orient=\"auto\"><path d=\"M 19,7 L11,14 L13,7 L11,0 Z\" fill=\"{}\"></path></marker></defs>",
        escape(transition_colour),
    )
}

/// The name of a marker in `markers.js` (`pointEnd-margin`, `crossStart`, ...), from its kind,
/// its end and whether it is the neo margin variant.
fn definition_name(kind: Kind, start: bool, margin: bool) -> String {
    let position = if start { "Start" } else { "End" };
    let suffix = if margin && kind.has_margin() {
        "-margin"
    } else {
        ""
    };
    format!("{}{position}{suffix}", kind.name())
}

/// The copy of a marker with a stroke colour, as `addEdgeMarker` makes it: the same definition
/// with the colour on its paths and circles (and on the fill of a point), under the coloured id.
#[must_use]
pub fn coloured_marker(
    svg_id: &str,
    diagram_type: &str,
    kind: Kind,
    start: bool,
    margin: bool,
    colour: &str,
) -> String {
    let name = definition_name(kind, start, margin);
    let original = marker_id(svg_id, diagram_type, kind, start, margin);
    let id = coloured_id(&original, colour);
    let definition = DEFINITIONS
        .iter()
        .find(|definition| definition.name == name)
        .unwrap_or(&DEFINITIONS[0]);
    definition_xml(&id, diagram_type, definition, Some(colour))
}

/// One child of an ER marker: a path, or a circle with its centre and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ErChild {
    /// A path with its data.
    Path(&'static str),
    /// A circle with radius `r`, centred on `y`, at `x` in the classic set and at `neo_x` in the neo
    /// set. Its fill is the background colour in the neo set and white in the classic one.
    Circle { x: f64, neo_x: f64, y: f64, r: f64 },
}

/// One ER marker of `markers.js`: its kind, its end or start, the reference point, its size, and
/// its children.
struct ErDefinition {
    kind: Kind,
    start: bool,
    ref_x: f64,
    ref_y: f64,
    width: f64,
    height: f64,
    children: &'static [ErChild],
}

/// The ER markers, in the order `insertMarkers` writes them: `only_one`, `zero_or_one`,
/// `one_or_more`, `zero_or_more`, each with its start and its end (`markers.js` 580-700). The
/// `zero_or_more` circles move to `45.5` and `11` in the neo set and `48` and `9` in the classic one
/// (`markers.js` 685, 698, 836, 859). The neo set differs in the other way too: its markers set
/// `markerUnits="userSpaceOnUse"`, and their stroke width is the theme's (`markers.js` 702-867).
const ER_DEFINITIONS: [ErDefinition; 8] = [
    ErDefinition {
        kind: Kind::OnlyOne,
        start: true,
        ref_x: 0.0,
        ref_y: 9.0,
        width: 18.0,
        height: 18.0,
        children: &[ErChild::Path("M9,0 L9,18 M15,0 L15,18")],
    },
    ErDefinition {
        kind: Kind::OnlyOne,
        start: false,
        ref_x: 18.0,
        ref_y: 9.0,
        width: 18.0,
        height: 18.0,
        children: &[ErChild::Path("M3,0 L3,18 M9,0 L9,18")],
    },
    ErDefinition {
        kind: Kind::ZeroOrOne,
        start: true,
        ref_x: 0.0,
        ref_y: 9.0,
        width: 30.0,
        height: 18.0,
        children: &[
            ErChild::Circle {
                x: 21.0,
                neo_x: 21.0,
                y: 9.0,
                r: 6.0,
            },
            ErChild::Path("M9,0 L9,18"),
        ],
    },
    ErDefinition {
        kind: Kind::ZeroOrOne,
        start: false,
        ref_x: 30.0,
        ref_y: 9.0,
        width: 30.0,
        height: 18.0,
        children: &[
            ErChild::Circle {
                x: 9.0,
                neo_x: 9.0,
                y: 9.0,
                r: 6.0,
            },
            ErChild::Path("M21,0 L21,18"),
        ],
    },
    ErDefinition {
        kind: Kind::OneOrMore,
        start: true,
        ref_x: 18.0,
        ref_y: 18.0,
        width: 45.0,
        height: 36.0,
        children: &[ErChild::Path(
            "M0,18 Q 18,0 36,18 Q 18,36 0,18 M42,9 L42,27",
        )],
    },
    ErDefinition {
        kind: Kind::OneOrMore,
        start: false,
        ref_x: 27.0,
        ref_y: 18.0,
        width: 45.0,
        height: 36.0,
        children: &[ErChild::Path("M3,9 L3,27 M9,18 Q27,0 45,18 Q27,36 9,18")],
    },
    ErDefinition {
        kind: Kind::ZeroOrMore,
        start: true,
        ref_x: 18.0,
        ref_y: 18.0,
        width: 57.0,
        height: 36.0,
        children: &[
            ErChild::Circle {
                x: 48.0,
                neo_x: 45.5,
                y: 18.0,
                r: 6.0,
            },
            ErChild::Path("M0,18 Q18,0 36,18 Q18,36 0,18"),
        ],
    },
    ErDefinition {
        kind: Kind::ZeroOrMore,
        start: false,
        ref_x: 39.0,
        ref_y: 18.0,
        width: 57.0,
        height: 36.0,
        children: &[
            ErChild::Circle {
                x: 9.0,
                neo_x: 11.0,
                y: 18.0,
                r: 6.0,
            },
            ErChild::Path("M21,18 Q39,0 57,18 Q39,36 21,18"),
        ],
    },
];

/// The ER markers of a diagram on the `neo` look or the `classic` one, as `insertMarkers` writes
/// them for `data4Layout.markers` (`erRenderer-unified.ts` 32-41): eight markers, each in its own
/// `<defs>`, under the diagram's id.
///
/// `stroke_width` is the theme's `strokeWidth` and `background` its `mainBkg`: the neo circles are
/// filled with the background, the classic ones with white (`markers.js` 754, 774, 835, 858). The
/// neo set takes its stroke width from the theme; the classic set has none.
#[must_use]
pub fn er_markers(svg_id: &str, neo: bool, stroke_width: &str, background: &str) -> String {
    let mut out = String::new();
    for definition in &ER_DEFINITIONS {
        let position = if definition.start { "Start" } else { "End" };
        let id = format!("{svg_id}_{ER}-{}{position}", definition.kind.name());
        let class = format!("marker {} {ER}", definition.kind.name());
        let units = if neo {
            " markerUnits=\"userSpaceOnUse\""
        } else {
            ""
        };
        let stroke = if neo {
            format!(" stroke-width=\"{}\"", escape(stroke_width))
        } else {
            String::new()
        };
        let children: String = definition
            .children
            .iter()
            .map(|child| match *child {
                ErChild::Path(d) => format!("<path d=\"{d}\"{stroke}/>"),
                ErChild::Circle { x, neo_x, y, r } => {
                    let (fill, cx) = if neo {
                        (background, neo_x)
                    } else {
                        ("white", x)
                    };
                    format!(
                        "<circle fill=\"{}\" cx=\"{}\" cy=\"{}\"{stroke} r=\"{}\"/>",
                        escape(fill),
                        number(cx),
                        number(y),
                        number(r),
                    )
                }
            })
            .collect();
        out.push_str(&format!(
            "<defs><marker id=\"{}\" class=\"{class}\" refX=\"{}\" refY=\"{}\" markerWidth=\"{}\" markerHeight=\"{}\" orient=\"auto\"{units}>{children}</marker></defs>",
            escape(&id),
            number(definition.ref_x),
            number(definition.ref_y),
            number(definition.width),
            number(definition.height),
        ));
    }
    out
}

/// The requirement markers of a diagram on the `neo` look or the `classic` one, as `insertMarkers`
/// writes them for `data4Layout.markers` (`requirementRenderer.ts` 19-24, `markers.js` 869-958):
/// `requirement_arrow` and `requirement_contains`, each in its own `<defs>`, under the diagram's id.
/// The reference writes the neo set with the theme's stroke width and `markerUnits` set to
/// `userSpaceOnUse`; the arrow has a `viewBox` and a miter join, and the contains marker's circle and
/// lines take the stroke width. The classic set has neither.
///
/// The arrow's path keeps the line breaks and indentation of the reference's template literal, which
/// the attribute holds.
#[must_use]
pub fn requirement_markers(svg_id: &str, neo: bool, stroke_width: &str) -> String {
    let units = if neo {
        " markerUnits=\"userSpaceOnUse\""
    } else {
        ""
    };
    let stroke = if neo {
        format!(" stroke-width=\"{}\"", escape(stroke_width))
    } else {
        String::new()
    };
    let arrow_id = marker_id(svg_id, REQUIREMENT, Kind::RequirementArrow, false, false);
    let contains_id = marker_id(svg_id, REQUIREMENT, Kind::RequirementContains, true, false);
    // The neo arrow takes the theme's stroke width and a viewBox, then a miter join on its path.
    let arrow_neo = if neo {
        format!(
            " stroke-width=\"{}\" viewBox=\"0 0 25 20\"",
            escape(stroke_width)
        )
    } else {
        String::new()
    };
    let arrow_join = if neo {
        " stroke-linejoin=\"miter\""
    } else {
        ""
    };
    // The reference's template literal: the line breaks and the six spaces of its indentation.
    let arrow_path = "M0,0\n      L20,10\n      M20,10\n      L0,20";
    let contains_children = format!(
        "<circle cx=\"10\" cy=\"10\" r=\"9\" fill=\"none\"{stroke}/>\
         <line x1=\"1\" x2=\"19\" y1=\"10\" y2=\"10\"{stroke}/>\
         <line y1=\"1\" y2=\"19\" x1=\"10\" x2=\"10\"{stroke}/>"
    );
    format!(
        "<defs><marker id=\"{}\" refX=\"20\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"{units}{arrow_neo}><path d=\"{arrow_path}\"{arrow_join}/></marker></defs>\
         <defs><marker id=\"{}\" refX=\"0\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"{units}><g>{contains_children}</g></marker></defs>",
        escape(&arrow_id),
        escape(&contains_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ids_follow_the_reference() {
        assert_eq!(
            marker_id("mermaid-svg", FLOWCHART, Kind::Point, false, false),
            "mermaid-svg_flowchart-v2-pointEnd"
        );
        assert_eq!(
            marker_id("mermaid-svg", FLOWCHART, Kind::Point, false, true),
            "mermaid-svg_flowchart-v2-pointEnd-margin"
        );
        assert_eq!(
            marker_id("mermaid-svg", FLOWCHART, Kind::Cross, true, true),
            "mermaid-svg_flowchart-v2-crossStart-margin"
        );
    }

    #[test]
    fn a_colour_becomes_an_id_suffix() {
        assert_eq!(
            coloured_id("m_flowchart-v2-pointEnd", "#f00"),
            "m_flowchart-v2-pointEnd__f00"
        );
        assert_eq!(coloured_id("m", "red"), "m_red");
    }

    #[test]
    fn there_are_twelve_markers_with_their_margin_variants() {
        let svg = markers("svg", FLOWCHART);
        assert_eq!(svg.matches("<marker ").count(), 12);
        assert!(svg.contains("id=\"svg_flowchart-v2-pointEnd-margin\""));
        assert!(svg.contains("id=\"svg_flowchart-v2-crossStart-margin\""));
        assert!(svg.contains(
            "<marker id=\"svg_flowchart-v2-pointStart-margin\" class=\"marker flowchart-v2\" viewBox=\"0 0 11.5 14\" refX=\"1\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"11.5\" markerHeight=\"14\" orient=\"auto\"><polygon points=\"0,7 11.5,14 11.5,0\" class=\"arrowMarkerPath\" style=\"stroke-width: 0; stroke-dasharray: 1, 0;\"/></marker>"
        ));
    }

    #[test]
    fn the_requirement_markers_are_two_with_the_ids_the_edges_use() {
        let classic = requirement_markers("svg", false, "2");
        assert_eq!(classic.matches("<defs><marker ").count(), 2);
        assert!(classic.contains("id=\"svg_requirement-requirement_arrowEnd\""));
        assert!(classic.contains("id=\"svg_requirement-requirement_containsStart\""));
        // The classic set has no marker units and no stroke width.
        assert!(!classic.contains("markerUnits"));
        let neo = requirement_markers("svg", true, "2");
        assert!(neo.contains("markerUnits=\"userSpaceOnUse\""));
        assert!(neo.contains("viewBox=\"0 0 25 20\""));
        assert!(neo.contains("stroke-width=\"2\""));
    }

    #[test]
    fn the_requirement_arrow_types_have_their_kinds() {
        assert_eq!(
            Kind::from_arrow("requirement_arrow"),
            Some(Kind::RequirementArrow)
        );
        assert_eq!(
            Kind::from_arrow("requirement_contains"),
            Some(Kind::RequirementContains)
        );
    }

    #[test]
    fn the_er_kinds_map_from_their_arrow_types() {
        assert_eq!(Kind::from_arrow("only_one"), Some(Kind::OnlyOne));
        assert_eq!(Kind::from_arrow("zero_or_one"), Some(Kind::ZeroOrOne));
        assert_eq!(Kind::from_arrow("one_or_more"), Some(Kind::OneOrMore));
        assert_eq!(Kind::from_arrow("zero_or_more"), Some(Kind::ZeroOrMore));
        // `md_parent` has no marker in the reference (`edgeMarker.ts` 92-94).
        assert_eq!(Kind::from_arrow("md_parent"), None);
    }

    #[test]
    fn the_er_marker_ids_have_no_margin_variant() {
        // `arrowTypesWithMarginSupport` lists no ER type, so the neo look asks for the plain id.
        assert_eq!(
            marker_id("svg", ER, Kind::OnlyOne, false, true),
            "svg_er-onlyOneEnd"
        );
        assert_eq!(
            marker_id("svg", ER, Kind::ZeroOrMore, true, true),
            "svg_er-zeroOrMoreStart"
        );
    }

    #[test]
    fn the_er_markers_are_eight_in_defs_with_the_ids_the_edges_use() {
        for neo in [false, true] {
            let svg = er_markers("svg", neo, "2", "#ffffff");
            assert_eq!(svg.matches("<defs><marker ").count(), 8, "{svg}");
            for kind in ["onlyOne", "zeroOrOne", "oneOrMore", "zeroOrMore"] {
                for position in ["Start", "End"] {
                    let id = format!("id=\"svg_er-{kind}{position}\"");
                    assert!(svg.contains(&id), "{id} in {svg}");
                }
            }
        }
    }

    #[test]
    fn the_neo_set_takes_the_theme_stroke_and_background_and_the_classic_one_does_not() {
        let neo = er_markers("svg", true, "2", "#ffffff");
        assert!(neo.contains("markerUnits=\"userSpaceOnUse\""), "{neo}");
        assert!(neo.contains("stroke-width=\"2\""), "{neo}");
        assert!(neo.contains("fill=\"#ffffff\" cx=\"45.5\""), "{neo}");
        let classic = er_markers("svg", false, "2", "#ffffff");
        assert!(!classic.contains("markerUnits"), "{classic}");
        assert!(!classic.contains("stroke-width"), "{classic}");
        assert!(classic.contains("fill=\"white\" cx=\"48\""), "{classic}");
    }

    #[test]
    fn a_coloured_point_carries_its_colour_on_path_and_fill() {
        let marker = coloured_marker("svg", FLOWCHART, Kind::Point, false, false, "#f00");
        assert!(
            marker.starts_with("<marker id=\"svg_flowchart-v2-pointEnd__f00\""),
            "{marker}"
        );
        assert!(marker.contains("stroke=\"#f00\" fill=\"#f00\""), "{marker}");
    }

    #[test]
    fn a_coloured_polygon_keeps_its_attributes() {
        let marker = coloured_marker("svg", FLOWCHART, Kind::Point, true, true, "red");
        assert!(!marker.contains("stroke=\"red\""), "{marker}");
    }
}

/// The relation markers of a class diagram: the five kinds of `markers.js` (aggregation, extension,
/// composition, dependency, lollipop), each at both ends and in the neo variant with the margin.
/// The names are the arrow types of the layout data (`classDb.ts` `getArrowMarker`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    /// `aggregation`: a hollow diamond.
    Aggregation,
    /// `extension`: a hollow triangle.
    Extension,
    /// `composition`: a filled diamond.
    Composition,
    /// `dependency`: an open arrow head.
    Dependency,
    /// `lollipop`: a circle.
    Lollipop,
}

impl ClassKind {
    /// The kind of an arrow type of a class relation; `None` for `none` and the flowchart arrows.
    #[must_use]
    pub fn from_arrow(arrow: &str) -> Option<Self> {
        match arrow {
            "aggregation" => Some(Self::Aggregation),
            "extension" => Some(Self::Extension),
            "composition" => Some(Self::Composition),
            "dependency" => Some(Self::Dependency),
            "lollipop" => Some(Self::Lollipop),
            _ => None,
        }
    }

    /// The name of the kind in the marker's id and class.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aggregation => "aggregation",
            Self::Extension => "extension",
            Self::Composition => "composition",
            Self::Dependency => "dependency",
            Self::Lollipop => "lollipop",
        }
    }
}

/// The diagram type the class markers are named with (`edgeMarker.ts` 99-100).
pub const CLASS: &str = "classDiagram";

/// The id of a class relation's marker: `<svgId>_classDiagram-<kind>Start|End`, with `-margin` for
/// the neo variant (`edgeMarker.ts` 59-69, 99-100).
#[must_use]
pub fn class_marker_id(svg_id: &str, kind: ClassKind, start: bool, margin: bool) -> String {
    let position = if start { "Start" } else { "End" };
    let suffix = if margin { "-margin" } else { "" };
    format!("{svg_id}_{CLASS}-{}{position}{suffix}", kind.name())
}

/// One class marker: its `<marker>` with the attributes `markers.js` sets, and its child. `view_box`
/// is the marker's own (the margin polygons of extension have one); `in_defs` wraps the marker in
/// its own `<defs>`, as `markers.js` does for all but one marker.
struct ClassMarker {
    id: String,
    class: &'static str,
    ref_x: &'static str,
    ref_y: &'static str,
    width: &'static str,
    height: &'static str,
    view_box: Option<&'static str>,
    child: String,
    in_defs: bool,
}

impl ClassMarker {
    fn xml(&self) -> String {
        let view_box = self
            .view_box
            .map_or_else(String::new, |view_box| format!(" viewBox=\"{view_box}\""));
        let marker = format!(
            "<marker id=\"{}\" class=\"{}\" refX=\"{}\" refY=\"{}\" markerWidth=\"{}\" markerHeight=\"{}\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"{view_box}>{}</marker>",
            escape(&self.id),
            self.class,
            self.ref_x,
            self.ref_y,
            self.width,
            self.height,
            self.child,
        );
        if self.in_defs {
            format!("<defs>{marker}</defs>")
        } else {
            marker
        }
    }
}

/// A path child of a class marker: its path data, its `stroke-width` as a style, and the marker's
/// own viewBox when the path has one (`composition` margin, `markers.js` 105-119).
fn class_path(d: &str, stroke_width: Option<&str>, view_box: Option<&str>) -> String {
    let style = stroke_width.map_or_else(String::new, |width| {
        format!(" style=\"stroke-width: {width};\"")
    });
    let view_box = view_box.map_or_else(String::new, |view_box| format!(" viewBox=\"{view_box}\""));
    format!("<path{style}{view_box} d=\"{d}\"/>")
}

/// The diamond of `aggregation` and `composition`, and the arrow head of `dependency`, as
/// `markers.js` draws them.
const DIAMOND: &str = "M 18,7 L9,13 L1,7 L9,1 Z";

/// The markers of a class diagram, in the order `insertMarkers` writes them for the five kinds
/// (`classRenderer-v3-unified.ts` 64: aggregation, extension, composition, dependency, lollipop).
/// Every kind has its start and end, each with the neo margin variant. `elem` receives them: all
/// but the extension's start-margin marker sit in a `<defs>` of their own.
#[must_use]
pub fn class_markers(svg_id: &str) -> String {
    let id =
        |kind: ClassKind, start: bool, margin: bool| class_marker_id(svg_id, kind, start, margin);
    let class = |kind: ClassKind| -> &'static str {
        match kind {
            ClassKind::Aggregation => "marker aggregation classDiagram",
            ClassKind::Extension => "marker extension classDiagram",
            ClassKind::Composition => "marker composition classDiagram",
            ClassKind::Dependency => "marker dependency classDiagram",
            ClassKind::Lollipop => "marker lollipop classDiagram",
        }
    };
    let mut out = String::new();
    let mut push = |marker: ClassMarker| out.push_str(&marker.xml());

    // aggregation (markers.js 136-194)
    let kind = ClassKind::Aggregation;
    for (start, margin, ref_x, width, height, stroke) in [
        (true, false, "18", "190", "240", None),
        (false, false, "1", "20", "28", None),
        (true, true, "15", "190", "240", Some("2")),
        (false, true, "1", "20", "28", Some("2")),
    ] {
        push(ClassMarker {
            id: id(kind, start, margin),
            class: class(kind),
            ref_x,
            ref_y: "7",
            width,
            height,
            view_box: None,
            child: class_path(DIAMOND, stroke, None),
            in_defs: true,
        });
    }

    // extension (markers.js 12-75): the start-margin marker is appended to `elem` itself.
    let kind = ClassKind::Extension;
    push(ClassMarker {
        id: id(kind, true, false),
        class: class(kind),
        ref_x: "18",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path("M 1,7 L18,13 V 1 Z", None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, false, false),
        class: class(kind),
        ref_x: "1",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path("M 1,1 V 13 L18,7 Z", None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, true, true),
        class: class(kind),
        ref_x: "18",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: Some("0 0 20 14"),
        child:
            "<polygon style=\"stroke-width: 2; stroke-dasharray: 0;\" points=\"10,7 18,13 18,1\"/>"
                .to_owned(),
        in_defs: false,
    });
    push(ClassMarker {
        id: id(kind, false, true),
        class: class(kind),
        ref_x: "9",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: Some("0 0 20 14"),
        child:
            "<polygon style=\"stroke-width: 2; stroke-dasharray: 0;\" points=\"10,1 10,13 18,7\"/>"
                .to_owned(),
        in_defs: true,
    });

    // composition (markers.js 76-135)
    let kind = ClassKind::Composition;
    push(ClassMarker {
        id: id(kind, true, false),
        class: class(kind),
        ref_x: "18",
        ref_y: "7",
        width: "190",
        height: "240",
        view_box: None,
        child: class_path(DIAMOND, None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, false, false),
        class: class(kind),
        ref_x: "1",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path(DIAMOND, None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, true, true),
        class: class(kind),
        ref_x: "15",
        ref_y: "7",
        width: "190",
        height: "240",
        view_box: None,
        child: class_path(DIAMOND, Some("0"), Some("0 0 15 15")),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, false, true),
        class: class(kind),
        ref_x: "3.5",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path(DIAMOND, Some("0"), None),
        in_defs: true,
    });

    // dependency (markers.js 195-252)
    let kind = ClassKind::Dependency;
    push(ClassMarker {
        id: id(kind, true, false),
        class: class(kind),
        ref_x: "6",
        ref_y: "7",
        width: "190",
        height: "240",
        view_box: None,
        child: class_path("M 5,7 L9,13 L1,7 L9,1 Z", None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, false, false),
        class: class(kind),
        ref_x: "13",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path("M 18,7 L9,13 L14,7 L9,1 Z", None, None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, true, true),
        class: class(kind),
        ref_x: "4",
        ref_y: "7",
        width: "190",
        height: "240",
        view_box: None,
        child: class_path("M 5,7 L9,13 L1,7 L9,1 Z", Some("0"), None),
        in_defs: true,
    });
    push(ClassMarker {
        id: id(kind, false, true),
        class: class(kind),
        ref_x: "16",
        ref_y: "7",
        width: "20",
        height: "28",
        view_box: None,
        child: class_path("M 18,7 L9,13 L14,7 L9,1 Z", Some("0"), None),
        in_defs: true,
    });

    // lollipop (markers.js 253-321)
    let kind = ClassKind::Lollipop;
    for (start, margin, ref_x, stroke) in [
        (true, false, "13", None),
        (false, false, "1", None),
        (true, true, "13", Some("2")),
        (false, true, "1", Some("2")),
    ] {
        let stroke_width =
            stroke.map_or_else(String::new, |width| format!(" stroke-width=\"{width}\""));
        push(ClassMarker {
            id: id(kind, start, margin),
            class: class(kind),
            ref_x,
            ref_y: "7",
            width: "190",
            height: "240",
            view_box: None,
            child: format!(
                "<circle fill=\"transparent\" cx=\"7\" cy=\"7\" r=\"6\"{stroke_width}/>"
            ),
            in_defs: true,
        });
    }
    out
}
