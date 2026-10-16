//! The text and the style of a class member as `ClassMember` gives them (`classTypes.ts`):
//! `getDisplayDetails` for the HTML label form (`displayText`), and `parseClassifier` for the style
//! of the row (`cssStyle`), which `shapeUtil.ts` gives to the member's row.

use ferrigura_model::class::{ClassMember, MemberType};
use ferrigura_syntax::class::parse_generic_types;

/// `displayText` of `getDisplayDetails` (`classTypes.ts` 62-78): the visibility and the name with
/// the generic types written as `<T>`, and for a method its parameters and its return type.
#[must_use]
pub fn display_text(member: &ClassMember) -> String {
    let mut text = format!("{}{}", member.visibility, parse_generic_types(&member.id));
    if member.member_type == MemberType::Method {
        let parameters = member.parameters.as_deref().unwrap_or_default().trim();
        text.push_str(&format!("({})", parse_generic_types(parameters)));
        if let Some(return_type) = member
            .return_type
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            text.push_str(" : ");
            text.push_str(&parse_generic_types(return_type));
        }
    }
    text.trim().to_owned()
}

/// `cssStyle` of `getDisplayDetails`, which is `parseClassifier` (`classTypes.ts` 136-146): an
/// abstract member is italic, a static one underlined, and any other keeps no style.
#[must_use]
pub fn css_style(member: &ClassMember) -> &'static str {
    match member.classifier.as_str() {
        "*" => "font-style:italic;",
        "$" => "text-decoration:underline;",
        _ => "",
    }
}
