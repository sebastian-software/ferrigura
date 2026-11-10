//! The gitGraph diagram: a port of `diagrams/git/gitGraphRenderer.ts` and `styles.js`. It is
//! described in `docs/diagrams/gitgraph.md#layout-and-rendering`.
//!
//! The drawing is built as a tree of elements (`tree.rs`), which knows the box each element has to
//! `getBBox`: the reference places the label boxes and the title from them, and its viewBox is the
//! box of the root. The stylesheet is in `style.rs`, the colors in `palette.rs`, the positions and
//! the elements in `layout.rs`, and the text in `text.rs`.

mod layout;
mod palette;
mod style;
mod text;
mod tree;

use ferrigura_model::config::GitGraphDiagramConfig;
use ferrigura_model::git_graph::GitGraphDiagram;
use ferrigura_text::{EmbeddedFamily, Run, embed_runs};

use self::layout::Layout;
use self::palette::Palette;
use self::tree::Element;
use crate::RenderError;
use crate::a11y;
use crate::context::{Context, ThemeView};
use crate::svg::{escape, number};

/// The font size of the title: `.gitTitleText` in the reference's stylesheet.
const TITLE_FONT_SIZE: f64 = 18.0;

/// Renders the diagram to an SVG document. The context carries the gitGraph section of the
/// configuration as its section, and the theme's tokens.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, or if the layout meets a state the reference
/// reports as an error (a commit without a branch, for example).
pub fn render(
    input: &Context<'_, GitGraphDiagram, GitGraphDiagramConfig>,
) -> Result<String, RenderError> {
    render_with_animation(input, None)
}

/// Renders a gitGraph with optional parent-to-commit connection tracing.
///
/// # Errors
/// Returns the errors of [`render`], or an option error if the speed overflows the timing.
pub fn render_with_animation(
    input: &Context<'_, GitGraphDiagram, GitGraphDiagramConfig>,
    animation: Option<crate::animation::Animation>,
) -> Result<String, RenderError> {
    let regular = input.text_fonts().face(
        ferrigura_text::FontWeight::Normal,
        ferrigura_text::FontStyle::Normal,
    )?;
    let mut bold = input.text_fonts().face(
        ferrigura_text::FontWeight::Bold,
        ferrigura_text::FontStyle::Normal,
    )?;
    let diagram = input.diagram;
    let config = input.section;
    let id = input.id;
    let theme: ThemeView<'_> = input.theme;
    let neo = input.globals.look == "neo";

    // The dark variant is the theme's name, as the reference's `DARK_THEMES` takes it. With two
    // schemes the light set is the primary one and the dark set is dark.
    let dark_name = theme.dark.is_none() && input.globals.theme.contains("dark");
    let light = Palette::new(theme.light, dark_name);
    if regular.css_reference().is_some() {
        let weight = match light.note_font_weight.as_str() {
            "normal" => 400,
            "bold" => 700,
            value => value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map_or(700, |value| value.clamp(1.0, 1000.0) as u16),
        };
        bold = input.text_fonts().face(
            ferrigura_text::FontWeight::Number(weight),
            ferrigura_text::FontStyle::Normal,
        )?;
    }

    let dark = match theme.dark {
        Some(dark) => Palette::new(dark, true),
        None => light.clone(),
    };

    let mut root = Element::container("svg");
    // The empty group `mermaidAPI` appends before the diagram draws.
    root.push(Element::group());
    if light.use_gradient {
        root.push(gradient_defs(id, &light));
    }
    if neo {
        root.push(drop_shadow_defs(id, &light));
    }

    // The lanes of the branches, from the widths of their names in the regular face.
    let mut layout = Layout::new(diagram, config, &light, id, neo, &regular, &bold);
    let rotate = config.rotate_commit_label;
    let mut pos = 0.0;
    let branches: Vec<_> = layout.branches().to_vec();
    for (index, &branch) in branches.iter().enumerate() {
        let (_, bbox) = text::draw_text(&branch.name, &regular, light.font_size)?;
        pos = layout.set_branch_position(&branch.name, pos, index, bbox.width, rotate);
    }

    let (bullets, labels) = layout.draw_commits(false)?;
    root.push(bullets);
    root.push(labels);
    if config.show_branches {
        root.push(layout.draw_branches()?);
    }
    let mut arrows = layout.draw_arrows()?;
    let mut animator = animation
        .map(|options| {
            let nodes: Vec<_> = diagram
                .commits
                .iter()
                .map(|commit| crate::unified::data::Node {
                    id: commit.id.clone(),
                    ..Default::default()
                })
                .collect();
            let edges: Vec<_> = diagram
                .commits
                .iter()
                .flat_map(|commit| {
                    commit
                        .parents
                        .iter()
                        .map(|parent| crate::unified::data::Edge {
                            start: parent.clone(),
                            end: commit.id.clone(),
                            ..Default::default()
                        })
                })
                .collect();
            let mut lengths = Vec::new();
            arrows.visit(&mut |element| {
                if element
                    .get("class")
                    .is_some_and(|classes| classes.split_whitespace().any(|class| class == "arrow"))
                {
                    lengths.push(crate::unified::path::path_length(
                        element.get("d").unwrap_or_default(),
                    ));
                }
            });
            crate::animation::Animator::from_lengths(id, &nodes, &edges, &lengths, options)
        })
        .transpose()?;
    if let Some(animator) = &mut animator {
        let mut index = 0;
        arrows.visit_mut(&mut |element| {
            if element
                .get("class")
                .is_some_and(|classes| classes.split_whitespace().any(|class| class == "arrow"))
            {
                let d = element.get("d").unwrap_or_default().to_owned();
                element.set_markup(animator.edge(index, &element.markup(), &d));
                index += 1;
            }
        });
    }
    root.push(arrows);
    let (bullets, labels) = layout.draw_commits_animated(true, animator.as_mut())?;
    root.push(bullets);
    root.push(labels);

    // `insertTitle`: centered over everything drawn so far, `titleTopMargin` above the origin.
    let title = diagram.title.as_str();
    if !title.is_empty() {
        let drawn = root.local_box().unwrap_or_default();
        // The title is centered by the browser on its advance, and its box is the glyphs' box.
        let width = text::width(&regular, title, TITLE_FONT_SIZE)?;
        let extent = text::extent(&regular, title, TITLE_FONT_SIZE)?;
        let x = drawn.x + drawn.width / 2.0;
        let y = -(config.title_top_margin as f64);
        let bounds = text::line_box(&regular, TITLE_FONT_SIZE, x - width / 2.0, y, extent)?;
        root.push(
            Element::text(title, bounds)
                .attr("x", number(x))
                .attr("y", number(y))
                .attr("text-anchor", "middle")
                .class("gitTitleText")
                .attr("fill", light.text_color()),
        );
    }

    // The fonts: every character the diagram draws, in the regular and the bold face.
    let mut texts = String::new();
    root.collect_text(&mut texts);
    let runs = [
        Run {
            text: &texts,
            face: &regular,
        },
        Run {
            text: &texts,
            face: &bold,
        },
    ];
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let family = regular.css_reference().unwrap_or_else(|| {
        families.first().map_or_else(
            || "\"sans-serif\"".to_owned(),
            |family| family.css_family.clone(),
        )
    });
    let dark_palette = &dark;
    let style = style::style_element(id, theme, &light, dark_palette, &family, &families);

    // `setupGraphViewbox`: the box of the root, with `diagramPadding` around it.
    let bounds = root.local_box().unwrap_or_default();
    let padding = config.diagram_padding;
    let (view_width, view_height) = (bounds.width + 2.0 * padding, bounds.height + 2.0 * padding);
    let size_attributes = if config.use_max_width {
        format!(
            "width=\"100%\" style=\"max-width: {}px;\"",
            number(view_width)
        )
    } else {
        format!(
            "height=\"{}\" width=\"{}\"",
            number(view_height),
            number(view_width)
        )
    };
    let accessibility = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    let accessible_attributes =
        a11y::root_attributes(id, "gitGraph", &diagram.acc_title, &diagram.acc_descr);
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} viewBox=\"{x} {y} {w} {h}\"{accessible_attributes}>{accessibility}{style}",
        id = escape(id),
        x = number(bounds.x - padding),
        y = number(bounds.y - padding),
        w = number(view_width),
        h = number(view_height),
    );
    root.write_children(&mut out);
    if let Some(animator) = animator {
        out.push_str(&animator.finish());
    }
    out.push_str("</svg>");
    Ok(out)
}

/// The linear gradient of a theme that uses one (`useGradient`), defined on the diagram's id.
fn gradient_defs(id: &str, palette: &Palette) -> Element {
    Element::hidden("defs").with(
        Element::hidden("linearGradient")
            .attr("id", format!("{id}-gradient"))
            .attr("gradientUnits", "objectBoundingBox")
            .attr("x1", "0%")
            .attr("y1", "0%")
            .attr("x2", "100%")
            .attr("y2", "0%")
            .with(
                Element::hidden("stop")
                    .attr("offset", "0%")
                    .attr("stop-color", palette.gradient_start.clone())
                    .attr("stop-opacity", "1"),
            )
            .with(
                Element::hidden("stop")
                    .attr("offset", "100%")
                    .attr("stop-color", palette.gradient_stop.clone())
                    .attr("stop-opacity", "1"),
            ),
    )
}

/// The drop shadow of the neo look with a redux theme, defined once on the diagram's id.
fn drop_shadow_defs(id: &str, palette: &Palette) -> Element {
    Element::hidden("defs").with(
        Element::hidden("filter")
            .attr("id", format!("{id}-drop-shadow"))
            .attr("height", "130%")
            .attr("width", "130%")
            .with(
                Element::hidden("feDropShadow")
                    .attr("dx", "4")
                    .attr("dy", "4")
                    .attr("stdDeviation", "0")
                    .attr("flood-opacity", "0.06")
                    .attr("flood-color", palette.filter_color.clone()),
            ),
    )
}
