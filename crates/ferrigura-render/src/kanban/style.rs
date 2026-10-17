//! The colours and rules of a kanban diagram: `styles.ts` (`genSections` and `getStyles`) for the
//! redux themes, with every colour a custom property on the diagram's id (`docs-port/decisions/0007`)
//! and the same colour as a presentation attribute on each element that takes it.
//!
//! The rules that match no element of a kanban are left out, each for its reason: `.edge` and the
//! edge rules (no edges), `.node-icon-N` (the icon is never drawn), `.section-edge-N` and
//! `.edge-depth-N` (no element carries them), `.section-N line` (no line in a section), `.disabled*`
//! (no element is disabled), `.section-root` and `.icon-container` (no git root and no icon),
//! and `.kanban-label` (no element has the class).
//! `.section-N` is the class of section `N`, which the reference numbers from 1 (`kanbanRenderer.ts`
//! 50, 59), so section `N` takes the colour of `cScale<N+1>` (`styles.ts` 24-26): the sheet keeps
//! that offset.

use ferrigura_model::theme::khroma::{darken, lighten};
use ferrigura_model::value::Value;

use crate::theme::Themes;
use crate::variables::{text_of, truthy};

/// `adjuster(color, 10)` of `genSections`: lightened, or darkened in the dark mode.
const SECTION_SHADE: f64 = 10.0;

/// One colour slot of the theme: the fill and stroke of a section, and the colour of its label.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Slot {
    /// `adjuster(cScale<i>, 10)`: the fill and stroke of the section rectangles.
    pub fill: String,
    /// `cScaleLabel<i>`: the colour of the section's title.
    pub label: String,
}

/// The colours of one scheme: the slots and the colours every element of the kanban takes.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Palette {
    /// `THEME_COLOR_LIMIT` slots, from `cScale0`.
    pub slots: Vec<Slot>,
    /// `options.background`: the fill of an item.
    pub background: String,
    /// `options.nodeBorder`: the stroke of an item.
    pub node_border: String,
    /// `options.textColor`: the colour of the item labels and of the cluster labels.
    pub text_color: String,
}

impl Palette {
    /// The palette of one set of theme variables (`genSections`, `styles.ts` 6-16 and 19).
    pub fn new(variables: &Value) -> Self {
        let limit = limit_of(variables);
        let dark_mode = truthy(variables, "darkMode");
        let slots = (0..limit)
            .map(|index| {
                let scale = text_of(variables, &format!("cScale{index}")).unwrap_or_default();
                let fill = if dark_mode {
                    darken(&scale, SECTION_SHADE)
                } else {
                    lighten(&scale, SECTION_SHADE)
                }
                .unwrap_or(scale);
                let label = text_of(variables, &format!("cScaleLabel{index}")).unwrap_or_default();
                Slot { fill, label }
            })
            .collect();
        Self {
            slots,
            background: text_of(variables, "background").unwrap_or_default(),
            node_border: text_of(variables, "nodeBorder").unwrap_or_default(),
            text_color: text_of(variables, "textColor").unwrap_or_default(),
        }
    }

    /// The slot the rules give to section `number` (1-based, as the class `section-<number>`):
    /// `.section-<number>` is `cScale<number + 1>` (`styles.ts` 24-26), when the theme has one.
    pub fn section_slot(&self, number: usize) -> Option<&Slot> {
        self.slots.get(number + 1)
    }
}

/// `THEME_COLOR_LIMIT` of the theme: twelve in the redux themes.
fn limit_of(variables: &Value) -> usize {
    text_of(variables, "THEME_COLOR_LIMIT")
        .and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|limit| limit.is_finite() && *limit >= 1.0)
        .map_or(12, |limit| limit.floor() as usize)
}

/// The colours of the schemes: the primary one and, when both are written, the dark one.
pub(super) struct Schemes {
    /// The palette the presentation attributes take.
    pub primary: Palette,
    /// The palette of the dark scheme, under `prefers-color-scheme: dark`.
    pub dark: Option<Palette>,
}

impl Schemes {
    pub fn new(themes: Themes<'_>) -> Self {
        match themes {
            Themes::Single(variables) => Self {
                primary: Palette::new(variables),
                dark: None,
            },
            Themes::Schemes { light, dark } => Self {
                primary: Palette::new(light),
                dark: Some(Palette::new(dark)),
            },
        }
    }
}

/// The custom properties of one scheme, on the diagram's id.
fn tokens(palette: &Palette) -> String {
    let mut out = format!(
        "--kanbanBackground:{};--kanbanTextColor:{};",
        palette.background, palette.text_color
    );
    for (index, slot) in palette.slots.iter().enumerate() {
        out.push_str(&format!(
            "--kanbanFill{index}:{};--kanbanLabel{index}:{};",
            slot.fill, slot.label
        ));
    }
    out
}

/// The rules the kanban adds to the base style sheet, as CSS text: the colour properties of the
/// schemes, then the section and item rules. The caller escapes the whole sheet once.
pub(super) fn own_rules(id: &str, schemes: &Schemes) -> String {
    let mut css = match &schemes.dark {
        None => format!("#{id}{{{}}}", tokens(&schemes.primary)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            tokens(&schemes.primary),
            tokens(dark)
        ),
    };
    // `genSections`: the rules of section `i - 1`, for each colour slot `i` (styles.ts 21-44).
    for index in 0..schemes.primary.slots.len() {
        let class = index as i64 - 1;
        css.push_str(&format!(
            "#{id} .section-{class} rect,#{id} .section-{class} path,#{id} .section-{class} circle,#{id} .section-{class} polygon{{fill:var(--kanbanFill{index});stroke:var(--kanbanFill{index});}}\
#{id} .section-{class} text{{fill:var(--kanbanLabel{index});}}"
        ));
    }
    // The item rules (styles.ts 52-58, written once: every slot writes the same copy), and the link
    // of a ticket (styles.ts 60-63).
    css.push_str(&format!(
        "#{id} .node rect,#{id} .node circle,#{id} .node ellipse,#{id} .node polygon,#{id} .node path{{fill:var(--kanbanBackground);stroke:var(--nodeBorder);stroke-width:1px;}}\
#{id} .kanban-ticket-link{{fill:var(--kanbanBackground);stroke:var(--nodeBorder);text-decoration:underline;}}"
    ));
    // The colour of the labels (styles.ts 98-101).
    css.push_str(&format!(
        "#{id} .cluster-label,#{id} .label{{color:var(--kanbanTextColor);fill:var(--kanbanTextColor);}}"
    ));
    css
}
