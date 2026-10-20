//! The rules of the venn diagram: `styles.ts` (the four rules, with the font of the diagram), and
//! the custom properties the colours of the circles and labels read. Every colour the theme gives
//! is a custom property on the diagram's id, declared once per scheme, and the elements also carry
//! the light colour as a presentation attribute (`docs-port/decisions/0007`).

/// The custom properties of one scheme: `(name, value)` pairs in the order they are declared.
pub(super) type Declarations = Vec<(String, String)>;

/// The rules that come after the base sheet: the custom properties of the colours (light, then dark
/// under `prefers-color-scheme: dark` when there is a dark scheme), then `styles.ts`, which keeps
/// its rules as the reference writes them. The font is the diagram's font, as `styles.ts` names it.
///
/// The reference's `font-size: 32px` of the title and its 48px of the labels are kept: the style
/// sheet beats the presentation attributes, which is why the title is drawn at 32px whatever its
/// attribute says (`docs-port/venn/NOTES.md` 6.3).
pub(super) fn own_rules(
    id: &str,
    font_family: &str,
    light: &Declarations,
    dark: Option<&Declarations>,
) -> String {
    let mut css = String::new();
    if !light.is_empty() {
        css.push_str(&format!("#{id}{{{}}}", declare(light)));
    }
    if let Some(dark) = dark.filter(|dark| !dark.is_empty()) {
        css.push_str(&format!(
            "@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            declare(dark)
        ));
    }
    css.push_str(&format!(
        "#{id} .venn-title{{font-size:32px;fill:var(--vennTitleTextColor);font-family:{font_family};}}\
#{id} .venn-circle text{{font-size:48px;font-family:{font_family};}}\
#{id} .venn-intersection text{{font-size:48px;fill:var(--vennSetTextColor);font-family:{font_family};}}\
#{id} .venn-text-node{{font-family:{font_family};color:var(--vennTextNodeColor);}}"
    ));
    css
}

/// `name:value;` for each custom property.
fn declare(declarations: &Declarations) -> String {
    declarations
        .iter()
        .map(|(name, value)| format!("{name}:{value};"))
        .collect()
}
