//! The style sheet of the use case diagram: `diagrams/usecase/styles.ts`, `getStyles` and `genColor`.
//! The rules come after the base rules of the diagram (`common/style.rs`), in the reference's order: the
//! palette rules first, then the actors, use cases, labels, boundaries, notes, JSON tables, relationships
//! and markers. Every selector starts with `#{id}`, which is the reference's `&` nesting.
//!
//! A theme colour is written `var(--name, value)`. `name` is the custom property of the token, or of the
//! palette slot (`borderColor<i>` and `bkgColor<i>`, as the flowchart and the ER name theirs), and `value`
//! is what this scheme gives it. The base sheet declares only `textColor`, `lineColor`, `nodeBorder`,
//! `errorBkgColor` and `errorTextColor` on the id, so those follow a dark scheme; the others use their
//! value until a caller declares them per scheme, as `flowchart/style.rs` does for its own colours.
//!
//! The `isHandDrawn ? '' : ...` splits of the reference are not ported. The handDrawn look is out of scope
//! for ferrigura (the harness excludes fixtures with it), so the text the reference keeps for the other
//! looks is always written.

use ferrigura_model::theme::{ThemeTokens, Token};

use crate::context::Globals;
use crate::unified::palette::{Palette, palette};

/// The rules of the use case diagram, to append to the base style sheet of the diagram. `rotate` is the
/// reference's `usecase.colorScheme === 'rotate'`: actors and use cases take a palette slot only then,
/// while the system boundaries take one under every colour theme.
#[must_use]
pub fn own_rules(id: &str, tokens: &ThemeTokens, globals: &Globals, rotate: bool) -> String {
    // `look` is validated before it reaches a selector (`safeLook`, `colorThemeGate.ts`).
    let look = safe_look(&globals.look);
    let attr = format!("[data-look=\"{look}\"]");
    let font = globals.font_family.as_str();
    let role = Roles::of(tokens);
    let actor_text = colour(&[
        ("actorTextColor", &tokens.actor_text_color),
        ("primaryTextColor", &tokens.primary_text_color),
    ]);
    let text = colour(&[("primaryTextColor", &tokens.primary_text_color)]);
    let title = colour(&[
        ("titleColor", &tokens.title_color),
        ("primaryTextColor", &tokens.primary_text_color),
    ]);
    let main_bkg = colour(&[("mainBkg", &tokens.main_bkg)]);
    let line = colour(&[("lineColor", &tokens.line_color)]);
    let edge_label = colour(&[("edgeLabelBackground", &tokens.edge_label_background)]);
    let json_border = colour(&[
        ("nodeBorder", &tokens.node_border),
        ("primaryColor", &tokens.primary_color),
    ]);
    let note_bkg = colour(&[("noteBkgColor", &tokens.note_bkg_color)]);
    let note_border = colour(&[("noteBorderColor", &tokens.note_border_color)]);
    let note_text = colour(&[("noteTextColor", &tokens.note_text_color)]);
    let actor_font = format!("var(--mermaid-usecase-actor-font-family, {font})");
    let font_family = format!("var(--mermaid-usecase-font-family, {font})");

    let mut css = String::new();
    if let Some(palette) = palette(globals, tokens) {
        css.push_str(&palette_rules(id, look, &palette, rotate));
    }

    // Actors (`styles.ts` 197-208).
    css.push_str(&rule(
        id,
        &[".usecase-actor"],
        &[decl("color", actor_text.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[
            ".usecase-actor-shape",
            ".usecase-actor-hollow",
            ".usecase-actor-awesome",
            ".usecase-actor-icon",
        ],
        &[
            decl("fill", role.actor_bkg.as_deref()),
            decl("stroke", role.actor_border.as_deref()),
            decl("stroke-width", "2px"),
        ],
    ));
    // The neo look's `[data-look="neo"].node path` stroke reaches the glyph's own paths, and a value set
    // on a child beats one inherited from its parent, so the children are named. `.node` outranks that
    // rule instead of tying with it. Stroke only: a hollow actor's `fill="none"` has to keep winning.
    css.push_str(&rule(
        id,
        &[
            ".node.usecase-actor .usecase-actor-glyph path",
            ".node.usecase-actor .usecase-actor-glyph circle",
        ],
        &[decl("stroke", role.actor_border.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[".usecase-actor .nodeLabel", ".actor-label"],
        &[
            decl("color", actor_text.as_deref()),
            decl("fill", actor_text.as_deref()),
            decl("font-family", actor_font.as_str()),
            decl("font-size", "var(--mermaid-usecase-actor-font-size, 14px)"),
            decl(
                "font-weight",
                "var(--mermaid-usecase-actor-font-weight, normal)",
            ),
        ],
    ));

    // Use cases and business use cases (`styles.ts` 240-247). The neo rules repeat the colours on the
    // `.node` form, for the same reason as the glyph above: the neo look's `rect` and `path` rules would
    // otherwise tie with the plain selectors (`styles.ts` 252-273).
    css.push_str(&rule(
        id,
        &[
            ".usecase-element ellipse",
            ".usecase-element rect",
            ".usecase-business ellipse",
            ".usecase-business rect",
        ],
        &[
            decl("fill", role.bkg.as_deref()),
            decl("stroke", role.border.as_deref()),
            decl("stroke-width", "2px"),
        ],
    ));
    css.push_str(&rule(
        id,
        &[
            format!("{attr}.node.usecase-element ellipse"),
            format!("{attr}.node.usecase-element rect"),
        ],
        &[
            decl("fill", role.bkg.as_deref()),
            decl("stroke", role.border.as_deref()),
        ],
    ));
    // The business marker is a path with `fill="none"`, which the rule leaves alone (`styles.ts` 269-273).
    css.push_str(&rule(
        id,
        &[format!(
            "{attr}.node.usecase-element .usecase-business-marker"
        )],
        &[decl("stroke", role.border.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[".usecase-element .nodeLabel", ".usecase-label"],
        &[
            decl("color", text.as_deref()),
            decl("fill", text.as_deref()),
            decl("font-family", font_family.as_str()),
            decl("font-size", "var(--mermaid-usecase-font-size, 12px)"),
            decl("font-weight", "var(--mermaid-usecase-font-weight, normal)"),
        ],
    ));
    css.push_str(&rule(
        id,
        &[".usecase-stereotype", ".usecase-business-marker"],
        &[
            decl("color", text.as_deref()),
            decl("fill", text.as_deref()),
            decl("stroke", role.border.as_deref()),
        ],
    ));

    // System boundaries (`styles.ts` 292-309).
    css.push_str(&rule(
        id,
        &[
            ".system-boundary rect.boundary-body",
            ".system-boundary rect.boundary-tab",
            ".system-boundary-package-tab",
        ],
        &[
            decl("fill", role.boundary_bkg.as_deref()),
            decl("stroke", role.boundary_border.as_deref()),
            decl("stroke-width", "1px"),
        ],
    ));
    css.push_str(&rule(
        id,
        &[".system-boundary-title text"],
        &[decl("fill", title.as_deref())],
    ));
    // Only the span: the renderer puts a user's `color` on the span, and its children inherit it.
    css.push_str(&rule(
        id,
        &[".system-boundary-title span"],
        &[decl("color", title.as_deref())],
    ));

    // Notes (`styles.ts` 310-319).
    css.push_str(&rule(
        id,
        &[".usecase-note"],
        &[
            decl("fill", note_bkg.as_deref()),
            decl("stroke", note_border.as_deref()),
            decl("color", note_text.as_deref()),
        ],
    ));
    css.push_str(&rule(
        id,
        &[".usecase-note .nodeLabel"],
        &[
            decl("color", note_text.as_deref()),
            decl("fill", note_text.as_deref()),
        ],
    ));

    // JSON tables (`styles.ts` 321-333).
    css.push_str(&rule(
        id,
        &[
            ".usecase-json-table",
            ".usecase-json-table rect",
            ".usecase-json-cell",
        ],
        &[
            decl("fill", main_bkg.as_deref()),
            decl("stroke", json_border.as_deref()),
        ],
    ));
    css.push_str(&rule(
        id,
        &[
            ".usecase-json-title",
            ".usecase-json-key",
            ".usecase-json-value",
        ],
        &[
            decl("color", text.as_deref()),
            decl("fill", text.as_deref()),
        ],
    ));

    // Relationships (`styles.ts` 335-359). Include and extend are both dashed, so their colours are
    // told apart by the role tokens, which default to `lineColor`.
    css.push_str(&rule(
        id,
        &[".relationship"],
        &[decl("fill", "none"), decl("stroke", line.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[
            ".relationship-include",
            ".relationship-extend",
            ".relationship-note",
        ],
        &[decl("stroke-dasharray", "3")],
    ));
    css.push_str(&rule(
        id,
        &[".relationship-include"],
        &[decl("stroke", role.include_line.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[".relationship-extend"],
        &[decl("stroke", role.extend_line.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[
            ".relationship.edge-animation-fast",
            ".relationship.edge-animation-slow",
        ],
        &[decl("stroke-linecap", "round")],
    ));

    // Edge labels and markers (`styles.ts` 361-395).
    css.push_str(&rule(
        id,
        &[".edgeLabel", ".edgeLabel p"],
        &[decl("background-color", edge_label.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[".labelBkg"],
        &[
            decl("background-color", edge_label.as_deref()),
            decl("padding", "0 2px"),
        ],
    ));
    css.push_str(&rule(
        id,
        &[".edgeLabel .label rect"],
        &[decl("fill", edge_label.as_deref())],
    ));
    css.push_str(&rule(
        id,
        &[".relationship-label", ".edgeLabel"],
        &[
            decl("color", text.as_deref()),
            decl("fill", text.as_deref()),
            decl("font-family", font),
            decl("font-size", "10px"),
            decl("font-weight", "normal"),
        ],
    ));
    css.push_str(&rule(
        id,
        &[
            ".marker",
            ".marker.point",
            ".marker.circle",
            ".marker.cross",
        ],
        &[
            decl("fill", line.as_deref()),
            decl("stroke", line.as_deref()),
        ],
    ));
    css.push_str(&rule(
        id,
        &[".marker.extension"],
        &[
            decl("fill", main_bkg.as_deref()),
            decl("stroke", line.as_deref()),
        ],
    ));
    css
}

/// The colours of the roles (`roleColors`, `styles.ts` 51-60): each role's usecase token, else the older
/// token it replaced, else the token the stylesheet used before the usecase tokens existed.
struct Roles {
    actor_bkg: Option<String>,
    actor_border: Option<String>,
    bkg: Option<String>,
    border: Option<String>,
    boundary_bkg: Option<String>,
    boundary_border: Option<String>,
    include_line: Option<String>,
    extend_line: Option<String>,
}

impl Roles {
    fn of(tokens: &ThemeTokens) -> Self {
        Self {
            actor_bkg: colour(&[
                ("usecaseActorBkg", &tokens.usecase_actor_bkg),
                ("actorBkg", &tokens.actor_bkg),
                ("mainBkg", &tokens.main_bkg),
            ]),
            actor_border: colour(&[
                ("usecaseActorBorder", &tokens.usecase_actor_border),
                ("actorBorder", &tokens.actor_border),
                ("primaryColor", &tokens.primary_color),
            ]),
            bkg: colour(&[
                ("usecaseBkg", &tokens.usecase_bkg),
                ("mainBkg", &tokens.main_bkg),
            ]),
            border: colour(&[
                ("usecaseBorder", &tokens.usecase_border),
                ("nodeBorder", &tokens.node_border),
                ("primaryColor", &tokens.primary_color),
            ]),
            boundary_bkg: colour(&[
                ("usecaseBoundaryBkg", &tokens.usecase_boundary_bkg),
                ("clusterBkg", &tokens.cluster_bkg),
            ]),
            boundary_border: colour(&[
                ("usecaseBoundaryBorder", &tokens.usecase_boundary_border),
                ("clusterBorder", &tokens.cluster_border),
            ]),
            include_line: colour(&[
                ("usecaseIncludeLine", &tokens.usecase_include_line),
                ("lineColor", &tokens.line_color),
            ]),
            extend_line: colour(&[
                ("usecaseExtendLine", &tokens.usecase_extend_line),
                ("lineColor", &tokens.line_color),
            ]),
        }
    }
}

/// The palette rules of `genColor` (`styles.ts` 75-187), for each slot of the border palette. The slots
/// are the border palette's length, so each element's `data-color-id` has a rule.
fn palette_rules(id: &str, look: &str, palette: &Palette, rotate: bool) -> String {
    let mut css = String::new();
    for (index, border) in palette.border.iter().enumerate() {
        let slot = format!("[data-look=\"{look}\"][data-color-id=\"color-{index}\"]");
        let stroke = var(&format!("borderColor{index}"), border);
        let fill = fill_slot(palette, index);
        let body = [
            decl("stroke", stroke.as_str()),
            decl("fill", fill.as_deref()),
        ];
        // System boundaries take a slot under both schemes. A boundary is a container, and numbering
        // the containers is the one place where the counter carries information.
        css.push_str(&rule(
            id,
            &[
                format!("{slot}.system-boundary rect.boundary-body"),
                format!("{slot}.system-boundary rect.boundary-tab"),
                format!("{slot}.system-boundary .boundary-body path"),
                format!("{slot}.system-boundary .boundary-tab path"),
            ],
            &body,
        ));
        if !rotate {
            continue;
        }
        // Use case bodies: the ellipse and the `[Rect]` form. Element selectors only, never a bare
        // `path`, so that a body is not repainted as one block.
        css.push_str(&rule(
            id,
            &[
                format!("{slot}.usecase-element ellipse"),
                format!("{slot}.usecase-element rect"),
            ],
            &body,
        ));
        // The business marker is a classed path, so it is reached by name. No fill: it is drawn with
        // `fill="none"`.
        css.push_str(&rule(
            id,
            &[format!("{slot}.usecase-element .usecase-business-marker")],
            &[decl("stroke", stroke.as_str())],
        ));
        // Actor glyphs take the fill on the group, never on its children, so a hollow actor stays hollow.
        css.push_str(&rule(
            id,
            &[
                format!("{slot}.usecase-actor .usecase-actor-shape"),
                format!("{slot}.usecase-actor .usecase-actor-hollow"),
                format!("{slot}.usecase-actor .usecase-actor-awesome"),
                format!("{slot}.usecase-actor .usecase-actor-icon"),
            ],
            &body,
        ));
        css.push_str(&rule(
            id,
            &[
                format!("{slot}.usecase-actor .usecase-actor-glyph path"),
                format!("{slot}.usecase-actor .usecase-actor-glyph circle"),
            ],
            &[decl("stroke", stroke.as_str())],
        ));
    }
    css
}

/// The fill of a palette slot: the background entry at the slot's index, wrapped around the background
/// list, which may be shorter than the border list (`i % bkgColorArray.length`). `None` without fills.
fn fill_slot(palette: &Palette, index: usize) -> Option<String> {
    let wrapped = index.checked_rem(palette.background.len())?;
    let colour = palette.background.get(wrapped)?;
    Some(var(&format!("bkgColor{wrapped}"), colour))
}

/// The first token of a fallback chain that the theme gives, as `var(--name, value)`. A token whose value
/// is not text or a number counts as absent. `None` when no token of the chain has a value.
fn colour(chain: &[(&str, &Token)]) -> Option<String> {
    chain
        .iter()
        .find_map(|(name, token)| Some(var(name, &token.text()?)))
}

/// A theme colour as `var(--name, value)`: the custom property `name`, with `value` as its fallback.
fn var(name: &str, value: &str) -> String {
    format!("var(--{name}, {value})")
}

/// One declaration, `property:value;`, or nothing when there is no value to write. The reference writes
/// `undefined` in that case, which the browser drops.
fn decl<'a>(property: &str, value: impl Into<Option<&'a str>>) -> String {
    value
        .into()
        .map_or_else(String::new, |value| format!("{property}:{value};"))
}

/// One rule: each selector scoped under `#id`, and the declarations in order. A rule without declarations
/// is left out.
fn rule<S: AsRef<str>>(id: &str, selectors: &[S], declarations: &[String]) -> String {
    let body = declarations.concat();
    if body.is_empty() {
        return String::new();
    }
    let selectors: Vec<String> = selectors
        .iter()
        .map(|selector| format!("#{id} {}", selector.as_ref()))
        .collect();
    format!("{}{{{body}}}", selectors.join(","))
}

/// The look as a selector takes it: a bare word, else `classic` (`safeLook`, `colorThemeGate.ts`). The
/// flowchart's `safe_look` is the same function, private to its module.
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::theme::Theme;
    use ferrigura_model::value::Value;

    use super::*;

    /// The globals of a diagram with the look `neo` and the given theme name.
    fn globals(theme: &str) -> Globals {
        Globals {
            look: "neo".to_owned(),
            theme: theme.to_owned(),
            font_family: "\"trebuchet ms\", verdana, arial, sans-serif".to_owned(),
            font_size: 16.0,
            security_level: "strict".to_owned(),
            layout: "elk".to_owned(),
            html_labels: None,
        }
    }

    fn redux() -> ThemeTokens {
        ThemeTokens::from_value(&Theme::ReduxColor.variables(None))
    }

    /// The selectors of every rule of a sheet, one entry per comma-separated selector.
    fn selectors(css: &str) -> Vec<&str> {
        css.split('}')
            .filter_map(|block| block.split_once('{'))
            .flat_map(|(selectors, _)| selectors.split(','))
            .map(str::trim)
            .collect()
    }

    /// Without a colour theme, or without a palette in one, there is no slot rule.
    #[test]
    fn without_a_colour_theme_there_is_no_slot_rule() {
        let css = own_rules("usecase-test", &redux(), &globals("default"), true);
        assert!(!css.contains("data-color-id"), "{css}");

        let empty =
            ThemeTokens::from_value(&Value::from(serde_json::json!({"borderColorArray": []})));
        let css = own_rules("usecase-test", &empty, &globals("redux-color"), true);
        assert!(!css.contains("data-color-id"), "{css}");
    }

    /// Under a colour theme there is a boundary rule for every palette slot.
    #[test]
    fn a_colour_theme_has_a_slot_rule_for_every_palette_slot() {
        let tokens = redux();
        let slots = tokens.border_color_array.texts().len();
        assert!(slots > 0);
        let css = own_rules("usecase-test", &tokens, &globals("redux-color"), false);
        let all = selectors(&css);
        for index in 0..slots {
            let boundary = format!(
                "[data-look=\"neo\"][data-color-id=\"color-{index}\"].system-boundary rect.boundary-body"
            );
            assert!(
                all.iter().any(|selector| selector.ends_with(&boundary)),
                "slot {index}: {css}"
            );
        }
        assert!(
            !css.contains(&format!("color-{slots}\"]")),
            "a slot past the palette: {css}"
        );
        assert!(
            css.contains("{stroke:var(--borderColor0, #E879F9);fill:var(--bkgColor0, #FDF4FF);}"),
            "{css}"
        );
    }

    /// Actors and use cases take a slot only under `rotate`; the boundaries take one either way.
    #[test]
    fn rotate_adds_the_actor_and_element_slot_rules() {
        let tokens = redux();
        let rotated = own_rules("usecase-test", &tokens, &globals("redux-color"), true);
        assert!(
            rotated.contains("[data-color-id=\"color-0\"].usecase-element ellipse,"),
            "{rotated}"
        );
        assert!(
            rotated.contains(
                "[data-color-id=\"color-0\"].usecase-actor .usecase-actor-glyph circle{stroke:var(--borderColor0, #E879F9);}"
            ),
            "{rotated}"
        );

        let plain = own_rules("usecase-test", &tokens, &globals("redux-color"), false);
        assert!(!plain.contains("].usecase-element"), "{plain}");
        assert!(!plain.contains("].usecase-actor"), "{plain}");
        assert!(
            plain.contains("[data-color-id=\"color-0\"].system-boundary rect.boundary-body"),
            "{plain}"
        );
    }

    /// Every rule is scoped under the diagram's id.
    #[test]
    fn every_selector_starts_with_the_id() {
        let tokens = redux();
        for rotate in [false, true] {
            let css = own_rules("usecase-test", &tokens, &globals("redux-color"), rotate);
            let all = selectors(&css);
            assert!(!all.is_empty());
            for selector in all {
                assert!(
                    selector.starts_with("#usecase-test "),
                    "{selector} is not scoped: {css}"
                );
            }
        }
        let dark = ThemeTokens::from_value(&Theme::ReduxDarkColor.variables(None));
        let css = own_rules("usecase-test", &dark, &globals("redux-dark-color"), true);
        assert!(
            selectors(&css)
                .iter()
                .all(|s| s.starts_with("#usecase-test "))
        );
    }

    /// The role tokens of a redux theme are written under their own names, with their values.
    #[test]
    fn the_roles_take_their_usecase_tokens() {
        let css = own_rules("usecase-test", &redux(), &globals("redux-color"), false);
        assert!(
            css.contains(
                "#usecase-test .usecase-element ellipse,#usecase-test .usecase-element rect,#usecase-test .usecase-business ellipse,#usecase-test .usecase-business rect{fill:var(--usecaseBkg, #F0FDFA);stroke:var(--usecaseBorder, #2DD4BF);stroke-width:2px;}"
            ),
            "{css}"
        );
        assert!(
            css.contains(".relationship-include{stroke:var(--usecaseIncludeLine, #38BDF8);}"),
            "{css}"
        );
    }

    /// A role without its usecase token falls back to the older token, then to the next, in order.
    #[test]
    fn a_role_falls_back_along_its_chain() {
        let tokens = ThemeTokens::from_value(&Value::from(serde_json::json!({
            "mainBkg": "#abcdef",
            "nodeBorder": "#123456",
            "lineColor": "#654321",
        })));
        let css = own_rules("usecase-test", &tokens, &globals("default"), false);
        assert!(
            css.contains(
                "fill:var(--mainBkg, #abcdef);stroke:var(--nodeBorder, #123456);stroke-width:2px;}"
            ),
            "{css}"
        );
        assert!(
            css.contains(".relationship-include{stroke:var(--lineColor, #654321);}"),
            "{css}"
        );
    }

    /// The background entries wrap around a shorter list, and a palette without fills has no fill.
    #[test]
    fn the_fills_wrap_around_a_shorter_background_palette() {
        let tokens = ThemeTokens::from_value(&Value::from(serde_json::json!({
            "borderColorArray": ["#a", "#b", "#c"],
            "bkgColorArray": ["#d", "#e"],
        })));
        let css = own_rules("usecase-test", &tokens, &globals("redux-color"), false);
        assert!(
            css.contains("{stroke:var(--borderColor2, #c);fill:var(--bkgColor0, #d);}"),
            "{css}"
        );
        assert!(
            css.contains("{stroke:var(--borderColor1, #b);fill:var(--bkgColor1, #e);}"),
            "{css}"
        );

        let no_fill = ThemeTokens::from_value(&Value::from(serde_json::json!({
            "borderColorArray": ["#a"],
        })));
        let css = own_rules("usecase-test", &no_fill, &globals("redux-color"), false);
        assert!(css.contains("{stroke:var(--borderColor0, #a);}"), "{css}");
    }

    /// A look that is not a bare word is replaced, and nothing of it reaches the selectors.
    #[test]
    fn an_unsafe_look_is_not_written_into_a_selector() {
        let mut globals = globals("redux-color");
        globals.look = "neo]{x}".to_owned();
        let css = own_rules("usecase-test", &redux(), &globals, true);
        assert!(
            css.contains("[data-look=\"classic\"][data-color-id=\"color-0\"]"),
            "{css}"
        );
        assert!(!css.contains("{x}"), "{css}");
    }

    /// A theme without the values gives rules without the colour declarations, and no panic.
    #[test]
    fn a_theme_without_the_values_writes_no_colour_declaration() {
        let css = own_rules(
            "usecase-test",
            &ThemeTokens::default(),
            &globals("default"),
            true,
        );
        assert!(!css.contains("fill:var("), "{css}");
        assert!(!css.contains("stroke:var("), "{css}");
        assert!(css.contains("#usecase-test .usecase-actor-shape,"), "{css}");
    }
}
