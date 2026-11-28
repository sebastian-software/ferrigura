//! The unified renderer: a port of `packages/mermaid/src/rendering-util/`, which the flowchart,
//! class, state and ER diagrams share. Each of them fills the layout data with its own `getData`;
//! the labels, the node shapes, the elkjs graph and the post-processing of its result, the edges,
//! the markers and the cluster frames are common. See `docs/diagrams/flowchart.md#layout-and-rendering`.

pub mod clusters;
pub mod data;
pub mod edges;
pub mod elk_apply;
pub mod elk_build;
pub mod elk_geometry;
pub mod elk_straighten;
pub mod elk_tree;
pub mod geom;
pub mod label;
pub mod line_jumps;
pub(crate) mod loops;
pub mod markdown;
mod marked;
pub mod markers;
pub mod palette;
pub mod path;
pub mod pipeline;
pub(crate) mod room;
pub mod shapes;
pub mod terminal;
pub mod tidy_tree;

use ferrigura_model::config::{
    ClassDiagramConfig, ErDiagramConfig, FlowchartDiagramConfig, MermaidConfig, MermaidConfigElk,
    StateDiagramConfig,
};
use ferrigura_model::theme::ThemeTokens;

use crate::context::{Context, Globals};
use crate::design::{Roles, Tone};

/// The sections of the configuration that the unified renderer reads, borrowed from the view that the
/// facade resolves. The `flowchart` section gives the layout's spacing, the curve and the title
/// margins; the classes, states, entity relations and mindmaps read it as the flowchart does. The `elk`
/// section gives the layout options. The `class` and `er` sections are read by the shapes of those two
/// diagrams. The `state` section is read across types too: the class diagram takes its title margin
/// and its `useMaxWidth` from it.
#[derive(Debug, Clone, Copy)]
pub struct Section<'a> {
    /// The tone of the default rendering, when the render draws in it
    /// (`docs/decisions/0023-the-default-rendering.md`).
    pub design: Option<Tone>,
    /// The shapes that carry a role in the default rendering: the flowchart's, or the state
    /// diagram's.
    pub roles: Roles,
    /// The `flowchart` section.
    pub flowchart: &'a FlowchartDiagramConfig,
    /// The `elk` section.
    pub elk: &'a MermaidConfigElk,
    /// The `class` section.
    pub class: &'a ClassDiagramConfig,
    /// The `er` section.
    pub er: &'a ErDiagramConfig,
    /// The `state` section.
    pub state: &'a StateDiagramConfig,
}

impl<'a> Section<'a> {
    /// The sections of Mermaid configuration, with Mermaid-compatible flowchart routing.
    /// The facade applies its separate render option for flowcharts.
    #[must_use]
    pub fn of(config: &'a MermaidConfig) -> Self {
        Self {
            design: None,
            roles: Roles::Flowchart,
            flowchart: &config.flowchart,
            elk: &config.elk,
            class: &config.class,
            er: &config.er,
            state: &config.state,
        }
    }
}

/// What the unified renderer reads besides its diagram: its sections, the global options and the
/// light theme as tokens. A diagram makes it from its context, and the pipeline, the shapes and the
/// clusters take it by reference.
#[derive(Debug, Clone)]
pub struct Config<'a> {
    /// Explicit font inputs and the configured primary family.
    pub fonts: crate::fonts::FontContext,
    /// The configuration sections.
    pub section: Section<'a>,
    /// The global options.
    pub globals: &'a Globals,
    /// The tokens of the theme that the sizes and the colors come from.
    pub theme: &'a ThemeTokens,
}

impl<'a> Config<'a> {
    /// The configuration that a unified diagram reads from its context: its section, the globals and
    /// the light scheme's tokens, which give the sizes and the colors of a render with one scheme.
    #[must_use]
    pub fn of<Model>(input: &Context<'a, Model, Section<'a>>) -> Self {
        Self {
            fonts: input.text_fonts(),
            section: *input.section,
            globals: input.globals,
            theme: input.theme.light,
        }
    }
}

/// The configuration and theme that the tests of the unified renderer build from a resolved
/// configuration, its theme variables and the globals.
#[cfg(test)]
pub(crate) struct Fixture {
    config: MermaidConfig,
    globals: Globals,
    theme: ThemeTokens,
}

#[cfg(test)]
impl Fixture {
    /// The fixture of a configuration with the theme variables given.
    pub(crate) fn new(config: MermaidConfig, variables: &ferrigura_model::value::Value) -> Self {
        Self {
            globals: Globals::of(&config),
            theme: ThemeTokens::from_value(variables),
            config,
        }
    }

    /// The configuration of the fixture, as the renderer reads it.
    pub(crate) fn config(&self) -> Config<'_> {
        Config {
            fonts: crate::fonts::FontContext::bundled(),
            section: Section::of(&self.config),
            globals: &self.globals,
            theme: &self.theme,
        }
    }
}
