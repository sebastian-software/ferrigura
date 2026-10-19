//! The colours of the event modeling diagram and its stylesheet rules.
//!
//! The reference's `styles.js` returns no rules (`docs-port/eventmodeling/NOTES.md` 6.8), so the
//! sheet is the common base only, and the colours are the code's fallbacks: none of the shipped
//! themes defines an `em` key (6.9). Every colour is a custom property declared on the diagram's id,
//! for the light scheme and, when both schemes are written, for the dark one under
//! `prefers-color-scheme: dark` (decision 0007). The elements carry their colours as presentation
//! attributes too, so a viewer without custom properties draws the same diagram.

use ferrigura_model::theme::{ThemeTokens, Token};

use crate::context::ThemeView;

/// The colours of one entity type: its fill and its stroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visual {
    /// The fill of the box.
    pub fill: String,
    /// The stroke of the box.
    pub stroke: String,
}

/// The colours of the diagram, as the theme gives them or the reference's fallbacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colours {
    /// The fill of the lanes, odd and even alike (`renderer.ts` 108-118).
    pub swimlane_fill: String,
    /// The stroke of the lanes.
    pub swimlane_stroke: String,
    /// The fill of the arrowhead's polygon (`renderer.ts` 150-160).
    pub arrowhead: String,
    /// The stroke of the relations (`renderer.ts` 86).
    pub relation_stroke: String,
}

/// The entity types' colours, in the order of the DB's `calculateEntityVisualProps` (`db.ts`
/// 290-328): the UI, the processor, the read model, the command and the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityColours {
    /// The UI's colours.
    pub ui: Visual,
    /// The processor's colours (`pcr` and `processor`).
    pub processor: Visual,
    /// The read model's colours (`rmo` and `readmodel`).
    pub read_model: Visual,
    /// The command's colours (`cmd` and `command`).
    pub command: Visual,
    /// The event's colours (`evt`, `event`, and any other keyword, which the grammar does not give).
    pub event: Visual,
}

/// The colours of the diagram in a theme's variables, with the reference's fallbacks.
#[must_use]
pub fn colours(tokens: &ThemeTokens) -> Colours {
    let or = |token: &Token, fallback: &str| token.text().unwrap_or_else(|| fallback.to_owned());
    Colours {
        swimlane_fill: or(&tokens.em_swimlane_background_odd, "rgb(250,250,250)"),
        swimlane_stroke: or(&tokens.em_swimlane_background_stroke, "rgb(240,240,240)"),
        arrowhead: or(&tokens.em_arrowhead, "#000000"),
        relation_stroke: or(&tokens.em_relation_stroke, "#000"),
    }
}

/// The colours of the entity types in a theme's variables, with the reference's fallbacks.
#[must_use]
pub fn entity_colours(tokens: &ThemeTokens) -> EntityColours {
    let visual =
        |fill: &Token, fill_fallback: &str, stroke: &Token, stroke_fallback: &str| Visual {
            fill: fill.text().unwrap_or_else(|| fill_fallback.to_owned()),
            stroke: stroke.text().unwrap_or_else(|| stroke_fallback.to_owned()),
        };
    EntityColours {
        ui: visual(&tokens.em_ui_fill, "white", &tokens.em_ui_stroke, "#dbdada"),
        processor: visual(
            &tokens.em_processor_fill,
            "#edb3f6",
            &tokens.em_processor_stroke,
            "#b88cbf",
        ),
        read_model: visual(
            &tokens.em_read_model_fill,
            "#d3f1a2",
            &tokens.em_read_model_stroke,
            "#a3b732",
        ),
        command: visual(
            &tokens.em_command_fill,
            "#bcd6fe",
            &tokens.em_command_stroke,
            "#679ac3",
        ),
        event: visual(
            &tokens.em_event_fill,
            "#ffb778",
            &tokens.em_event_stroke,
            "#c19a0f",
        ),
    }
}

impl EntityColours {
    /// The colours of an entity type as written (`db.ts` 290-328). The DB's `default` branch (red
    /// and black) is not reached: the grammar gives only the nine keywords.
    #[must_use]
    pub fn of(&self, entity_type: &str) -> &Visual {
        match entity_type {
            "ui" => &self.ui,
            "pcr" | "processor" => &self.processor,
            "rmo" | "readmodel" => &self.read_model,
            "cmd" | "command" => &self.command,
            _ => &self.event,
        }
    }
}

/// The custom properties of the colours of one scheme, as the rules use them.
fn declarations(tokens: &ThemeTokens) -> String {
    let colours = colours(tokens);
    let entities = entity_colours(tokens);
    let mut out = format!(
        "--emSwimlaneFill:{};--emSwimlaneStroke:{};--emArrowhead:{};--emRelationStroke:{};",
        colours.swimlane_fill, colours.swimlane_stroke, colours.arrowhead, colours.relation_stroke
    );
    for (name, visual) in [
        ("Ui", &entities.ui),
        ("Processor", &entities.processor),
        ("ReadModel", &entities.read_model),
        ("Command", &entities.command),
        ("Event", &entities.event),
    ] {
        out.push_str(&format!(
            "--em{name}Fill:{};--em{name}Stroke:{};",
            visual.fill, visual.stroke
        ));
    }
    out
}

/// The colour properties on the id, for each scheme (`Themes::Schemes` writes the dark one under the
/// media query).
#[must_use]
pub fn tokens(svg_id: &str, theme: ThemeView<'_>) -> String {
    match theme.dark {
        None => format!("#{svg_id}{{{}}}", declarations(theme.light)),
        Some(dark) => format!(
            "#{svg_id}{{{}}}@media (prefers-color-scheme: dark){{#{svg_id}{{{}}}}}",
            declarations(theme.light),
            declarations(dark)
        ),
    }
}

/// The rules of the diagram on the id, after the properties: the lanes, the relations and the
/// arrowhead take their colours from the custom properties.
#[must_use]
pub fn rules(svg_id: &str) -> String {
    format!(
        "#{svg_id} .em-swimlane rect{{fill:var(--emSwimlaneFill);stroke:var(--emSwimlaneStroke);}}\
#{svg_id} .em-relation{{stroke:var(--emRelationStroke);}}\
#{svg_id} marker polygon{{fill:var(--emArrowhead);}}"
    )
}

/// The custom-property name of a box's fill and stroke for an entity type, so a box can refer to
/// its colours with `var(…)` (`--emUiFill`, `--emUiStroke`, and so on).
#[must_use]
pub fn box_token(entity_type: &str) -> (&'static str, &'static str) {
    match entity_type {
        "ui" => ("--emUiFill", "--emUiStroke"),
        "pcr" | "processor" => ("--emProcessorFill", "--emProcessorStroke"),
        "rmo" | "readmodel" => ("--emReadModelFill", "--emReadModelStroke"),
        "cmd" | "command" => ("--emCommandFill", "--emCommandStroke"),
        _ => ("--emEventFill", "--emEventStroke"),
    }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::value::Value;

    use super::*;

    #[test]
    fn the_fallbacks_are_the_reference_colours_when_the_theme_has_none() {
        let tokens = ThemeTokens::from_value(&Value::Object(Default::default()));
        let entities = entity_colours(&tokens);
        assert_eq!(entities.of("ui").fill, "white");
        assert_eq!(entities.of("pcr").stroke, "#b88cbf");
        assert_eq!(entities.of("command").fill, "#bcd6fe");
        assert_eq!(entities.of("evt").stroke, "#c19a0f");
        assert_eq!(colours(&tokens).swimlane_fill, "rgb(250,250,250)");
    }

    #[test]
    fn a_theme_variable_replaces_its_fallback() {
        let variables = Value::Object(
            [("emUiFill".to_owned(), Value::String("#123456".to_owned()))]
                .into_iter()
                .collect(),
        );
        let tokens = ThemeTokens::from_value(&variables);
        assert_eq!(entity_colours(&tokens).ui.fill, "#123456");
    }
}
