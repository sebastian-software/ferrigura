//! Port of `packages/mermaid/src/diagrams/quadrant-chart/parser/quadrant.jison.spec.ts`. Test names
//! follow the spec's titles in snake_case: the title of a `describe` is joined in front of the
//! title of its `it`, and the punctuation of a title becomes separators.
//!
//! The reference's `not.toThrow()` becomes `parse(…).expect("parses")`, and its `toThrow()` becomes
//! `parse(…).is_err()`. Each DB call the spec asserts becomes an assertion on the state the call
//! leaves in the model: the texts of the axes and quadrants, the points found by their text, and the
//! classes. The spec's `type: 'text'` objects have no counterpart in the model, so they are not
//! asserted. The DB sanitises and trims the texts, so the spec's untrimmed texts are compared trimmed.
//!
//! The spec's constructor test (`classDef constructor fill:#ff0000`) runs against a mock DB that
//! takes any style. The reference's real DB rejects `fill`, so the port asserts the rejection and,
//! for the protocol the spec is about, the class name with a style the DB accepts.
//!
//! No test here renders the chart, so none is ignored.

use ferrigura_model::quadrant_chart::{QuadrantChartDiagram, QuadrantPoint, QuadrantStyle};
use ferrigura_syntax::quadrant_chart::parse;

fn parses(text: &str) -> QuadrantChartDiagram {
    parse(text).expect("parses")
}

/// The point with the text, which the DB holds in the reverse of the source order.
fn point<'a>(diagram: &'a QuadrantChartDiagram, text: &str) -> &'a QuadrantPoint {
    diagram
        .points
        .iter()
        .find(|point| point.text == text)
        .expect("a point with that text")
}

fn no_style() -> QuadrantStyle {
    QuadrantStyle::default()
}

#[test]
fn should_throw_error_if_quadrant_chart_text_is_not_there() {
    assert!(parse("quadrant-1 do").is_err());
}

#[test]
fn should_not_throw_error_if_only_quadrant_chart_is_there() {
    parse("quadrantChart").expect("parses");
}

#[test]
fn should_be_able_to_parse_x_axis_text() {
    let diagram = parses("quadrantChart\nx-axis urgent --> not urgent");
    assert_eq!(diagram.x_axis_left_text, "urgent");
    assert_eq!(diagram.x_axis_right_text, "not urgent");

    let diagram =
        parses("quadrantChart\n       x-AxIs         Urgent     -->        Not Urgent    \n");
    assert_eq!(diagram.x_axis_left_text, "Urgent");
    assert_eq!(diagram.x_axis_right_text, "Not Urgent");

    let diagram = parses(
        "quadrantChart\n       x-AxIs         \"Urgent(* +=[❤\"     -->        \"Not Urgent (* +=[❤\"\n    ",
    );
    assert_eq!(diagram.x_axis_left_text, "Urgent(* +=[❤");
    assert_eq!(diagram.x_axis_right_text, "Not Urgent (* +=[❤");

    let diagram = parses("quadrantChart\n       x-AxIs         \"Urgent(* +=[❤\"");
    assert_eq!(diagram.x_axis_left_text, "Urgent(* +=[❤");
    assert_eq!(diagram.x_axis_right_text, "");

    let diagram = parses("quadrantChart\n       x-AxIs         \"Urgent(* +=[❤\"  --> ");
    assert_eq!(diagram.x_axis_left_text, "Urgent(* +=[❤ ⟶");
    assert_eq!(diagram.x_axis_right_text, "");
}

#[test]
fn should_be_able_to_parse_y_axis_text() {
    let diagram = parses("quadrantChart\ny-axis urgent --> not urgent");
    assert_eq!(diagram.y_axis_bottom_text, "urgent");
    assert_eq!(diagram.y_axis_top_text, "not urgent");

    let diagram =
        parses("quadrantChart\n       y-AxIs         Urgent     -->        Not Urgent    \n");
    assert_eq!(diagram.y_axis_bottom_text, "Urgent");
    assert_eq!(diagram.y_axis_top_text, "Not Urgent");

    let diagram = parses(
        "quadrantChart\n       Y-AxIs         \"Urgent(* +=[❤\"     -->        \"Not Urgent (* +=[❤\"\n    ",
    );
    assert_eq!(diagram.y_axis_bottom_text, "Urgent(* +=[❤");
    assert_eq!(diagram.y_axis_top_text, "Not Urgent (* +=[❤");

    let diagram = parses("quadrantChart\n       y-AxIs         \"Urgent(* +=[❤\"");
    assert_eq!(diagram.y_axis_bottom_text, "Urgent(* +=[❤");
    assert_eq!(diagram.y_axis_top_text, "");

    let diagram = parses("quadrantChart\n       y-AxIs         \"Urgent(* +=[❤\"  --> ");
    assert_eq!(diagram.y_axis_bottom_text, "Urgent(* +=[❤ ⟶");
    assert_eq!(diagram.y_axis_top_text, "");
}

#[test]
fn should_be_able_to_parse_quadrant1_text() {
    assert_eq!(
        parses("quadrantChart\nquadrant-1 Plan").quadrant1_text,
        "Plan"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-1 Plan    ").quadrant1_text,
        "Plan"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-1 \"Plan(* +=[❤\"").quadrant1_text,
        "Plan(* +=[❤"
    );
}

#[test]
fn should_be_able_to_parse_quadrant2_text() {
    assert_eq!(parses("quadrantChart\nquadrant-2 do").quadrant2_text, "do");
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-2 Do    ").quadrant2_text,
        "Do"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-2 \"Do(* +=[❤\"").quadrant2_text,
        "Do(* +=[❤"
    );
}

#[test]
fn should_be_able_to_parse_quadrant3_text() {
    assert_eq!(
        parses("quadrantChart\nquadrant-3 delegate").quadrant3_text,
        "delegate"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-3 Delegate    ").quadrant3_text,
        "Delegate"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-3 \"Delegate(* +=[❤\"").quadrant3_text,
        "Delegate(* +=[❤"
    );
}

#[test]
fn should_be_able_to_parse_quadrant4_text() {
    assert_eq!(
        parses("quadrantChart\nquadrant-4 delete").quadrant4_text,
        "delete"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-4 Delete    ").quadrant4_text,
        "Delete"
    );
    assert_eq!(
        parses("QuadRantChart   \n     QuaDrant-4 \"Delete(* +=[❤\"").quadrant4_text,
        "Delete(* +=[❤"
    );
}

#[test]
fn should_be_able_to_parse_title() {
    assert_eq!(
        parses("quadrantChart\ntitle this is title").title,
        "this is title"
    );
    assert_eq!(
        parses("QuadRantChart   \n     TiTle this Is title    ").title,
        "this Is title"
    );
    assert_eq!(
        parses("QuadRantChart   \n     title \"this is title (* +=[❤\"").title,
        "\"this is title (* +=[❤\""
    );
}

#[test]
fn should_be_able_to_parse_points() {
    let diagram = parses("quadrantChart\npoint1: [0.1, 0.4]");
    assert_eq!(diagram.points.len(), 1);
    let found = point(&diagram, "point1");
    assert_eq!(
        (found.class_name.as_str(), found.x, found.y),
        ("", 0.1, 0.4)
    );
    assert_eq!(found.style, no_style());

    let diagram = parses("QuadRantChart   \n     Point1      : [0.1, 0.4]   ");
    let found = point(&diagram, "Point1");
    assert_eq!(
        (found.class_name.as_str(), found.x, found.y),
        ("", 0.1, 0.4)
    );

    let diagram = parses("QuadRantChart   \n     \"Point1 : (* +=[❤\": [1, 0]   ");
    let found = point(&diagram, "Point1 : (* +=[❤");
    assert_eq!(
        (found.class_name.as_str(), found.x, found.y),
        ("", 1.0, 0.0)
    );

    assert!(parse("QuadRantChart   \n     Point1 : [1.2, 0.4]   ").is_err());
}

#[test]
fn should_be_able_to_parse_the_whole_chart() {
    let text = "quadrantChart
      title Analytics and Business Intelligence Platforms
      x-axis \"Completeness of Vision ❤\" --> \"x-axis-2\"
      y-axis Ability to Execute --> \"y-axis-2\"
      quadrant-1 Leaders
      quadrant-2 Challengers
      quadrant-3 Niche
      quadrant-4 Visionaries
      Microsoft: [0.75, 0.75]
      Salesforce: [0.55, 0.60]
      IBM: [0.51, 0.40]
      Incorta: [0.20, 0.30]";
    let diagram = parses(text);
    assert_eq!(diagram.x_axis_left_text, "Completeness of Vision ❤");
    assert_eq!(diagram.x_axis_right_text, "x-axis-2");
    assert_eq!(diagram.y_axis_top_text, "y-axis-2");
    assert_eq!(diagram.y_axis_bottom_text, "Ability to Execute");
    assert_eq!(diagram.quadrant1_text, "Leaders");
    assert_eq!(diagram.quadrant2_text, "Challengers");
    assert_eq!(diagram.quadrant3_text, "Niche");
    assert_eq!(diagram.quadrant4_text, "Visionaries");
    for (text, x, y) in [
        ("Microsoft", 0.75, 0.75),
        ("Salesforce", 0.55, 0.60),
        ("IBM", 0.51, 0.40),
        ("Incorta", 0.20, 0.30),
    ] {
        let found = point(&diagram, text);
        assert_eq!((found.class_name.as_str(), found.x, found.y), ("", x, y));
        assert_eq!(found.style, no_style());
    }
}

#[test]
fn should_be_able_to_parse_the_whole_chart_with_point_styling_with_all_params_or_some_params() {
    let text = "quadrantChart
      title Analytics and Business Intelligence Platforms
      x-axis \"Completeness of Vision ❤\" --> \"x-axis-2\"
      y-axis Ability to Execute --> \"y-axis-2\"
      quadrant-1 Leaders
      quadrant-2 Challengers
      quadrant-3 Niche
      quadrant-4 Visionaries
      Microsoft: [0.75, 0.75] radius: 10
      Salesforce: [0.55, 0.60] radius: 10, color: #ff0000
      IBM: [0.51, 0.40] radius: 10, color: #ff0000, stroke-color: #ff00ff
      Incorta: [0.20, 0.30] radius: 10 ,color: #ff0000 ,stroke-color: #ff00ff ,stroke-width: 10px";
    let diagram = parses(text);
    assert_eq!(diagram.x_axis_left_text, "Completeness of Vision ❤");
    assert_eq!(diagram.quadrant4_text, "Visionaries");
    assert_eq!(
        point(&diagram, "Microsoft").style,
        QuadrantStyle {
            radius: Some(10.0),
            ..no_style()
        }
    );
    assert_eq!(
        point(&diagram, "Salesforce").style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            ..no_style()
        }
    );
    assert_eq!(
        point(&diagram, "IBM").style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            stroke_color: Some("#ff00ff".to_owned()),
            ..no_style()
        }
    );
    assert_eq!(
        point(&diagram, "Incorta").style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            stroke_color: Some("#ff00ff".to_owned()),
            stroke_width: Some("10px".to_owned()),
        }
    );
}

#[test]
fn should_be_able_to_parse_the_whole_chart_with_point_styling_with_params_in_a_random_order_class_names()
 {
    let text = "quadrantChart
      title Analytics and Business Intelligence Platforms
      x-axis \"Completeness of Vision ❤\" --> \"x-axis-2\"
      y-axis Ability to Execute --> \"y-axis-2\"
      quadrant-1 Leaders
      quadrant-2 Challengers
      quadrant-3 Niche
      quadrant-4 Visionaries
      Microsoft: [0.75, 0.75] stroke-color: #ff00ff ,stroke-width: 10px, color: #ff0000, radius: 10
      Salesforce:::class1: [0.55, 0.60] radius: 10, color: #ff0000
      IBM: [0.51, 0.40] stroke-color: #ff00ff ,stroke-width: 10px
      Incorta: [0.20, 0.30] stroke-width: 10px";
    let diagram = parses(text);
    assert_eq!(diagram.y_axis_top_text, "y-axis-2");
    assert_eq!(diagram.quadrant2_text, "Challengers");
    assert_eq!(
        point(&diagram, "Microsoft").style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            stroke_color: Some("#ff00ff".to_owned()),
            stroke_width: Some("10px".to_owned()),
        }
    );
    let salesforce = point(&diagram, "Salesforce");
    assert_eq!(salesforce.class_name, "class1");
    assert_eq!(
        salesforce.style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            ..no_style()
        }
    );
    assert_eq!(
        point(&diagram, "IBM").style,
        QuadrantStyle {
            stroke_color: Some("#ff00ff".to_owned()),
            stroke_width: Some("10px".to_owned()),
            ..no_style()
        }
    );
    assert_eq!(
        point(&diagram, "Incorta").style,
        QuadrantStyle {
            stroke_width: Some("10px".to_owned()),
            ..no_style()
        }
    );
}

#[test]
fn should_be_able_to_handle_constructor_as_a_class_name() {
    // The spec's mock DB takes any style. The reference's DB does not know `fill`, so the chart is
    // rejected; the class name itself is tested with a style the DB knows.
    let with_fill = "quadrantChart
    classDef constructor fill:#ff0000
    Microsoft:::constructor: [0.75, 0.75]
    ";
    assert!(parse(with_fill).is_err());

    let diagram = parses(
        "quadrantChart
    classDef constructor color:#ff0000
    Microsoft:::constructor: [0.75, 0.75]
    ",
    );
    assert_eq!(
        diagram.classes,
        vec![(
            "constructor".to_owned(),
            QuadrantStyle {
                color: Some("#ff0000".to_owned()),
                ..no_style()
            }
        )]
    );
    assert_eq!(point(&diagram, "Microsoft").class_name, "constructor");
}

mod unicode_support_cjk_emoji {
    use super::*;

    #[test]
    fn should_be_able_to_parse_chinese_text_in_quadrant_labels() {
        let diagram = parses("quadrantChart\nquadrant-1 需要扩展");
        assert_eq!(diagram.quadrant1_text, "需要扩展");

        let diagram =
            parses("quadrantChart\nquadrant-2 需要推广\nquadrant-3 重新评估\nquadrant-4 可以改进");
        assert_eq!(diagram.quadrant2_text, "需要推广");
        assert_eq!(diagram.quadrant3_text, "重新评估");
        assert_eq!(diagram.quadrant4_text, "可以改进");
    }

    #[test]
    fn should_be_able_to_parse_chinese_text_in_x_axis() {
        let diagram = parses("quadrantChart\nx-axis 低覆盖率 --> 高覆盖率");
        assert_eq!(diagram.x_axis_left_text, "低覆盖率");
        assert_eq!(diagram.x_axis_right_text, "高覆盖率");
    }

    #[test]
    fn should_be_able_to_parse_chinese_text_in_y_axis() {
        let diagram = parses("quadrantChart\ny-axis 低参与度 --> 高参与度");
        assert_eq!(diagram.y_axis_bottom_text, "低参与度");
        assert_eq!(diagram.y_axis_top_text, "高参与度");
    }

    #[test]
    fn should_be_able_to_parse_chinese_point_names() {
        let diagram = parses("quadrantChart\n产品A: [0.3, 0.6]");
        let found = point(&diagram, "产品A");
        assert_eq!((found.x, found.y), (0.3, 0.6));
    }

    #[test]
    fn should_be_able_to_parse_a_full_chart_with_chinese() {
        let text = "quadrantChart
title 分析象限图
x-axis 低覆盖率 --> 高覆盖率
y-axis 低参与度 --> 高参与度
quadrant-1 需要扩展
quadrant-2 需要推广
quadrant-3 重新评估
quadrant-4 可以改进
产品A: [0.3, 0.6]
产品B: [0.45, 0.23]";
        let diagram = parses(text);
        assert_eq!(diagram.title, "分析象限图");
        assert_eq!(diagram.x_axis_left_text, "低覆盖率");
        assert_eq!(diagram.x_axis_right_text, "高覆盖率");
        assert_eq!(diagram.y_axis_bottom_text, "低参与度");
        assert_eq!(diagram.y_axis_top_text, "高参与度");
        assert_eq!(diagram.quadrant1_text, "需要扩展");
        assert_eq!(
            (point(&diagram, "产品A").x, point(&diagram, "产品A").y),
            (0.3, 0.6)
        );
        assert_eq!(
            (point(&diagram, "产品B").x, point(&diagram, "产品B").y),
            (0.45, 0.23)
        );
    }

    #[test]
    fn should_be_able_to_parse_japanese_text() {
        let diagram = parses("quadrantChart\nquadrant-1 拡張が必要\nx-axis 低い --> 高い");
        assert_eq!(diagram.quadrant1_text, "拡張が必要");
    }

    #[test]
    fn should_be_able_to_parse_korean_text() {
        let diagram = parses("quadrantChart\nx-axis 낮음 --> 높음");
        assert_eq!(diagram.x_axis_left_text, "낮음");
    }

    #[test]
    fn should_be_able_to_parse_emoji_in_text() {
        let diagram = parses("quadrantChart\nquadrant-2 🚀Growth");
        assert_eq!(diagram.quadrant2_text, "🚀Growth");
    }

    #[test]
    fn should_parse_latin1_accented_text_french_spanish_german() {
        let diagram = parses("quadrantChart\nx-axis Café --> Größe");
        assert_eq!(diagram.x_axis_left_text, "Café");
        assert_eq!(diagram.x_axis_right_text, "Größe");
    }

    #[test]
    fn should_parse_accented_characters_in_quadrant_labels() {
        let diagram = parses("quadrantChart\nquadrant-1 catégoría\nquadrant-2 naïve");
        assert_eq!(diagram.quadrant1_text, "catégoría");
        assert_eq!(diagram.quadrant2_text, "naïve");
    }
}
