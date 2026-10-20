//! The unified renderer: a port of `packages/mermaid/src/rendering-util/`, which the flowchart,
//! class, state and ER diagrams share. Each of them fills the layout data with its own `getData`;
//! the labels, the node shapes, the elkjs graph and the post-processing of its result, the edges,
//! the markers and the cluster frames are common. See `docs-port/flowchart/RENDER-PLAN.md`.

pub mod clusters;
pub mod data;
pub mod edges;
pub mod elk_apply;
pub mod elk_build;
pub mod elk_geometry;
pub mod elk_straighten;
pub mod elk_tree;
pub mod flow_group;
pub mod geom;
pub mod label;
pub mod line_jumps;
pub mod markdown;
pub mod markers;
pub mod palette;
pub mod path;
pub mod pipeline;
pub mod shapes;
pub mod swimlane_lane;
pub mod swimlane_layout;
pub mod terminal;
pub mod tidy_tree;

use ferrigura_model::config::{
    ClassDiagramConfig, ErDiagramConfig, FlowchartDiagramConfig, MermaidConfig, MermaidConfigElk,
    StateDiagramConfig,
};
use ferrigura_model::theme::ThemeTokens;

use crate::context::{Context, Globals};

/// The sections of the configuration that the unified renderer reads, borrowed from the view that the
/// facade resolves. The `flowchart` section gives the layout's spacing, the curve and the title
/// margins; the classes, states, entity relations and mindmaps read it as the flowchart does. The `elk`
/// section gives the layout options. The `class` and `er` sections are read by the shapes of those two
/// diagrams. The `state` section is read across types too: the class and requirement diagrams take
/// their title margin and their `useMaxWidth` from it.
#[derive(Debug, Clone, Copy)]
pub struct Section<'a> {
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
    /// The sections of a resolved configuration.
    #[must_use]
    pub fn of(config: &'a MermaidConfig) -> Self {
        Self {
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
#[derive(Debug, Clone, Copy)]
pub struct Config<'a> {
    /// The configuration sections.
    pub section: Section<'a>,
    /// The global options.
    pub globals: &'a Globals,
    /// The tokens of the theme that the sizes and the colours come from.
    pub theme: &'a ThemeTokens,
}

impl<'a> Config<'a> {
    /// The configuration that a unified diagram reads from its context: its section, the globals and
    /// the light scheme's tokens, which give the sizes and the colours of a render with one scheme.
    #[must_use]
    pub fn of<Model>(input: &Context<'a, Model, Section<'a>>) -> Self {
        Self {
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
            section: Section::of(&self.config),
            globals: &self.globals,
            theme: &self.theme,
        }
    }
}
