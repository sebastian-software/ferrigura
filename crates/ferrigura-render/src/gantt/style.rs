//! The style sheet of the gantt chart: `styles.js` as a sheet scoped to the diagram's id, with the
//! colours as custom properties (decision 0007), and the concrete colours of the elements that the
//! rules colour, as presentation attributes.
//!
//! The rules are the reference's, in its order (`styles.js:1-290`): the order decides which rule
//! wins for an element that two rules colour, and the attributes take the winner's colour.

use ferrigura_model::theme::{ThemeTokens, Token};

use crate::context::ThemeView;
use crate::svg::css_text;

/// The colour tokens of the gantt theme (`styles.js`): every colour a rule names, with its name, in
/// the order the sheet declares them.
fn colour_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 21] {
    [
        ("excludeBkgColor", &tokens.exclude_bkg_color),
        ("sectionBkgColor", &tokens.section_bkg_color),
        ("sectionBkgColor2", &tokens.section_bkg_color2),
        ("altSectionBkgColor", &tokens.alt_section_bkg_color),
        ("titleColor", &tokens.title_color),
        ("gridColor", &tokens.grid_color),
        ("textColor", &tokens.text_color),
        ("todayLineColor", &tokens.today_line_color),
        ("taskTextDarkColor", &tokens.task_text_dark_color),
        ("taskTextOutsideColor", &tokens.task_text_outside_color),
        ("taskTextColor", &tokens.task_text_color),
        ("taskBkgColor", &tokens.task_bkg_color),
        ("taskBorderColor", &tokens.task_border_color),
        ("activeTaskBkgColor", &tokens.active_task_bkg_color),
        ("activeTaskBorderColor", &tokens.active_task_border_color),
        ("doneTaskBkgColor", &tokens.done_task_bkg_color),
        ("doneTaskBorderColor", &tokens.done_task_border_color),
        ("critBkgColor", &tokens.crit_bkg_color),
        ("critBorderColor", &tokens.crit_border_color),
        ("vertLineColor", &tokens.vert_line_color),
        ("taskTextClickableColor", &tokens.task_text_clickable_color),
    ]
}

/// The state of a task bar: the class `task`, `active`, `done`, `crit` and their combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bar {
    Task,
    Active,
    Done,
    Crit,
    ActiveCrit,
    DoneCrit,
}

/// The colour tokens of a bar's fill and stroke. The state rules name the section classes `0` to
/// `3` only, so a section beyond them has no colour from them; `vert` sets the stroke of every
/// vertical marker (`.vert`, which comes last in the sheet).
pub(crate) fn bar_tokens(
    tokens: &ThemeTokens,
    bar: Bar,
    section: usize,
    vert: bool,
) -> (Option<&Token>, Option<&Token>) {
    let (fill, stroke) = if section < 4 {
        match bar {
            Bar::Task => (
                Some(&tokens.task_bkg_color),
                Some(&tokens.task_border_color),
            ),
            Bar::Active => (
                Some(&tokens.active_task_bkg_color),
                Some(&tokens.active_task_border_color),
            ),
            Bar::Done => (
                Some(&tokens.done_task_bkg_color),
                Some(&tokens.done_task_border_color),
            ),
            Bar::Crit => (
                Some(&tokens.crit_bkg_color),
                Some(&tokens.crit_border_color),
            ),
            Bar::ActiveCrit => (
                Some(&tokens.active_task_bkg_color),
                Some(&tokens.crit_border_color),
            ),
            Bar::DoneCrit => (
                Some(&tokens.done_task_bkg_color),
                Some(&tokens.crit_border_color),
            ),
        }
    } else {
        (None, None)
    };
    (
        fill,
        if vert {
            Some(&tokens.vert_line_color)
        } else {
            stroke
        },
    )
}

/// The fill token of a section's background: `.section0`, `.section2`, and `.section1`/`.section3`.
pub(crate) fn section_token(tokens: &ThemeTokens, section: usize) -> Option<&Token> {
    match section {
        0 => Some(&tokens.section_bkg_color),
        1 | 3 => Some(&tokens.alt_section_bkg_color),
        2 => Some(&tokens.section_bkg_color2),
        _ => None,
    }
}

/// The fill token of the text of a task, from its class tokens, by the cascade of `styles.js`. A
/// rule that is `!important` beats one that is not; among rules of the same kind the more specific
/// (two classes) wins, and then the later one in the sheet. Returns `None` when no rule colours the
/// text.
pub(crate) fn text_token<'t>(tokens: &'t ThemeTokens, classes: &[&str]) -> Option<&'t Token> {
    let has = |class: &str| classes.contains(&class);
    let indexed = |prefix: &str| (0..4).any(|n| has(&format!("{prefix}{n}")));
    let outside = has("taskTextOutsideLeft") || has("taskTextOutsideRight");
    // (important, specificity, sheet order, token). The sheet order is the line order of styles.js.
    let mut best: Option<(bool, u8, u8, &'t Token)> = None;
    let mut consider = |important: bool, specificity: u8, order: u8, token: &'t Token| {
        let candidate = (important, specificity, order, token);
        if best.is_none_or(|(i, s, o, _)| (important, specificity, order) >= (i, s, o)) {
            best = Some(candidate);
        }
    };
    if outside {
        consider(false, 1, 1, &tokens.task_text_dark_color);
    }
    if has("clickable") && (has("taskText") || outside) {
        consider(true, 2, 2, &tokens.task_text_clickable_color);
    }
    if has("taskText") {
        consider(false, 1, 3, &tokens.task_text_color);
    }
    if indexed("taskTextOutside") {
        consider(false, 1, 4, &tokens.task_text_outside_color);
    }
    if indexed("activeText") {
        consider(true, 1, 10, &tokens.task_text_dark_color);
    }
    if indexed("doneText") {
        consider(true, 1, 11, &tokens.task_text_dark_color);
        if outside {
            consider(true, 2, 12, &tokens.task_text_outside_color);
        }
    }
    if indexed("doneCritText") {
        consider(true, 1, 13, &tokens.task_text_dark_color);
        if outside {
            consider(true, 2, 14, &tokens.task_text_outside_color);
        }
    }
    if has("vertText") {
        consider(true, 1, 15, &tokens.vert_line_color);
    }
    if indexed("activeCritText") {
        consider(true, 1, 16, &tokens.task_text_dark_color);
    }
    best.map(|(_, _, _, token)| token)
}

/// The token of the section titles and the diagram title: `titleColor`, or the text colour where
/// the theme gives none (`options.titleColor || options.textColor`).
pub(crate) fn title_token(tokens: &ThemeTokens) -> &Token {
    match tokens.title_color.text() {
        Some(colour) if !colour.is_empty() => &tokens.title_color,
        _ => &tokens.text_color,
    }
}

/// The presentation attribute of a colour token, with a leading space, or nothing when the theme
/// does not give the token.
pub(crate) fn attribute(name: &str, token: &Token) -> String {
    token
        .text()
        .map(|colour| format!(" {name}=\"{}\"", crate::svg::escape(&colour)))
        .unwrap_or_default()
}

/// The `<style>` element of the chart: the colour tokens (one set, or a light and a dark set under
/// `prefers-color-scheme`), the embedded font faces, and the rules of `styles.js` with every
/// selector starting with the diagram's id.
pub(crate) fn style_element(id: &str, theme: ThemeView<'_>, family: &str, faces: &str) -> String {
    let tokens = match theme.dark {
        None => format!("#{id}{{{}}}", declarations(theme.light)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            declarations(theme.light),
            declarations(dark)
        ),
    };
    // `family` is the quoted `font-family` list of the embedded faces, or empty.
    let font = if family.is_empty() {
        "sans-serif".to_owned()
    } else {
        family.to_owned()
    };
    // `font-size` of the root is the theme's `fontSize`, as `getStyles` writes it (`styles.ts`).
    let size = theme
        .light
        .font_size
        .text()
        .unwrap_or_else(|| "16px".to_owned());
    let root = format!("#{id}{{font-family: {font};font-size: {size};fill: var(--textColor);}}");
    let rules: String = rules(&font)
        .into_iter()
        .map(|(selectors, body)| {
            let scoped: Vec<String> = selectors
                .split(',')
                .map(|selector| format!("#{id} {}", selector.trim()))
                .collect();
            format!("{}{{{body}}}", scoped.join(", "))
        })
        .collect();
    format!("<style>{}{tokens}{root}{rules}</style>", css_text(faces))
}

/// The custom property declarations of the colour tokens a set gives.
fn declarations(tokens: &ThemeTokens) -> String {
    colour_tokens(tokens)
        .into_iter()
        .filter_map(|(name, token)| {
            token
                .text()
                .map(|colour| format!("--{name}: {};", css_text(&colour)))
        })
        .collect()
}

/// The `styles.js` rules in their order: each selector list, and its declarations. The colours are
/// `var(--token)` references. `font` is the family the text is set in.
fn rules(font: &str) -> Vec<(String, String)> {
    let indexed = |prefix: &str| -> String {
        (0..4)
            .map(|n| format!(".{prefix}{n}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let doubled = |first: &str, second: &str| -> String {
        (0..4)
            .flat_map(|n| {
                [
                    format!(".{first}{n}.{second}Left"),
                    format!(".{first}{n}.{second}Right"),
                ]
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let clickable =
        "cursor: pointer; fill: var(--taskTextClickableColor) !important; font-weight: bold";
    let dark_important = "fill: var(--taskTextDarkColor) !important";
    let outside_important = "fill: var(--taskTextOutsideColor) !important";
    vec![
        (".exclude-range".to_owned(), "fill: var(--excludeBkgColor)".to_owned()),
        (".section".to_owned(), "stroke: none; opacity: 0.2".to_owned()),
        (".section0".to_owned(), "fill: var(--sectionBkgColor)".to_owned()),
        (".section2".to_owned(), "fill: var(--sectionBkgColor2)".to_owned()),
        (
            ".section1, .section3".to_owned(),
            "fill: var(--altSectionBkgColor); opacity: 0.2".to_owned(),
        ),
        (indexed("sectionTitle"), "fill: var(--titleColor)".to_owned()),
        (".sectionTitle".to_owned(), format!("text-anchor: start; font-family: {font}")),
        (
            ".grid .tick".to_owned(),
            "stroke: var(--gridColor); opacity: 0.8; shape-rendering: crispEdges".to_owned(),
        ),
        (".grid .tick text".to_owned(), format!("font-family: {font}; fill: var(--textColor)")),
        (".grid path".to_owned(), "stroke-width: 0".to_owned()),
        (
            ".today".to_owned(),
            "fill: none; stroke: var(--todayLineColor); stroke-width: 2px".to_owned(),
        ),
        (".task".to_owned(), "stroke-width: 2".to_owned()),
        (".taskText".to_owned(), format!("text-anchor: middle; font-family: {font}")),
        (
            ".taskTextOutsideRight".to_owned(),
            format!("fill: var(--taskTextDarkColor); text-anchor: start; font-family: {font}"),
        ),
        (
            ".taskTextOutsideLeft".to_owned(),
            "fill: var(--taskTextDarkColor); text-anchor: end".to_owned(),
        ),
        (".task.clickable".to_owned(), "cursor: pointer".to_owned()),
        (".taskText.clickable".to_owned(), clickable.to_owned()),
        (".taskTextOutsideLeft.clickable".to_owned(), clickable.to_owned()),
        (".taskTextOutsideRight.clickable".to_owned(), clickable.to_owned()),
        (indexed("taskText"), "fill: var(--taskTextColor)".to_owned()),
        (indexed("task"), "fill: var(--taskBkgColor); stroke: var(--taskBorderColor)".to_owned()),
        (
            ".taskTextOutside0, .taskTextOutside2".to_owned(),
            "fill: var(--taskTextOutsideColor)".to_owned(),
        ),
        (
            ".taskTextOutside1, .taskTextOutside3".to_owned(),
            "fill: var(--taskTextOutsideColor)".to_owned(),
        ),
        (
            indexed("active"),
            "fill: var(--activeTaskBkgColor); stroke: var(--activeTaskBorderColor)".to_owned(),
        ),
        (indexed("activeText"), dark_important.to_owned()),
        (
            indexed("done"),
            "stroke: var(--doneTaskBorderColor); fill: var(--doneTaskBkgColor); stroke-width: 2"
                .to_owned(),
        ),
        (indexed("doneText"), dark_important.to_owned()),
        (doubled("doneText", "taskTextOutside"), outside_important.to_owned()),
        (
            indexed("crit"),
            "stroke: var(--critBorderColor); fill: var(--critBkgColor); stroke-width: 2".to_owned(),
        ),
        (
            indexed("activeCrit"),
            "stroke: var(--critBorderColor); fill: var(--activeTaskBkgColor); stroke-width: 2"
                .to_owned(),
        ),
        (
            indexed("doneCrit"),
            "stroke: var(--critBorderColor); fill: var(--doneTaskBkgColor); stroke-width: 2; cursor: pointer; shape-rendering: crispEdges"
                .to_owned(),
        ),
        (".milestone".to_owned(), "transform: rotate(45deg) scale(0.8,0.8)".to_owned()),
        (".milestoneText".to_owned(), "font-style: italic".to_owned()),
        (indexed("doneCritText"), dark_important.to_owned()),
        (doubled("doneCritText", "taskTextOutside"), outside_important.to_owned()),
        (".vert".to_owned(), "stroke: var(--vertLineColor)".to_owned()),
        (
            ".vertText".to_owned(),
            "font-size: 15px; text-anchor: middle; fill: var(--vertLineColor) !important".to_owned(),
        ),
        (indexed("activeCritText"), dark_important.to_owned()),
        (
            ".titleText".to_owned(),
            format!("text-anchor: middle; font-size: 18px; fill: var(--titleColor); font-family: {font}"),
        ),
    ]
}
