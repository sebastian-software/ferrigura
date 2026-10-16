//! Port of the `displayText` and `cssStyle` tests of `packages/mermaid/src/diagrams/class/classTypes.spec.ts`
//! (`getDisplayDetails`). The inputs and expectations are the ones the ignored stubs of the syntax
//! crate recorded (`ferrigura-syntax/tests/class_types_spec.rs`, which this file replaces). The
//! member is parsed as an attribute or a method (`Attribute`/`Method` in the stub's comment, else a
//! method when the input has a `(`), as the reference's `new ClassMember(input, type)` takes it.

use ferrigura_render::class::member::{css_style, display_text};
use ferrigura_syntax::class::class_member;

#[test]
fn should_parse_correctly() {
    let member = class_member("getTime()", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime()");
}

#[test]
fn should_handle_public_visibility() {
    let member = class_member("+getTime()", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime()");
}

#[test]
fn should_handle_private_visibility() {
    let member = class_member("-getTime()", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime()");
}

#[test]
fn should_handle_protected_visibility() {
    let member = class_member("#getTime()", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime()");
}

#[test]
fn should_handle_internal_visibility() {
    let member = class_member("~getTime()", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime()");
}

#[test]
fn should_return_correct_css_for_static_classifier() {
    let member = class_member("getTime()$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime()");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier() {
    let member = class_member("getTime()*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime()");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_2() {
    let member = class_member("getTime(int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int)");
}

#[test]
fn should_handle_public_visibility_2() {
    let member = class_member("+getTime(int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime(int)");
}

#[test]
fn should_handle_private_visibility_2() {
    let member = class_member("-getTime(int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime(int)");
}

#[test]
fn should_handle_protected_visibility_2() {
    let member = class_member("#getTime(int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime(int)");
}

#[test]
fn should_handle_internal_visibility_2() {
    let member = class_member("~getTime(int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime(int)");
}

#[test]
fn should_return_correct_css_for_static_classifier_2() {
    let member = class_member("getTime(int)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_2() {
    let member = class_member("getTime(int)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_3() {
    let member = class_member("getTime(int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int count)");
}

#[test]
fn should_handle_public_visibility_3() {
    let member = class_member("+getTime(int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime(int count)");
}

#[test]
fn should_handle_private_visibility_3() {
    let member = class_member("-getTime(int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime(int count)");
}

#[test]
fn should_handle_protected_visibility_3() {
    let member = class_member("#getTime(int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime(int count)");
}

#[test]
fn should_handle_internal_visibility_3() {
    let member = class_member("~getTime(int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime(int count)");
}

#[test]
fn should_return_correct_css_for_static_classifier_3() {
    let member = class_member("getTime(int count)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int count)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_3() {
    let member = class_member("getTime(int count)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(int count)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_4() {
    let member = class_member("getTime(count int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(count int)");
}

#[test]
fn should_handle_public_visibility_4() {
    let member = class_member("+getTime(count int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime(count int)");
}

#[test]
fn should_handle_private_visibility_4() {
    let member = class_member("-getTime(count int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime(count int)");
}

#[test]
fn should_handle_protected_visibility_4() {
    let member = class_member("#getTime(count int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime(count int)");
}

#[test]
fn should_handle_internal_visibility_4() {
    let member = class_member("~getTime(count int)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime(count int)");
}

#[test]
fn should_return_correct_css_for_static_classifier_4() {
    let member = class_member("getTime(count int)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(count int)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_4() {
    let member = class_member("getTime(count int)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(count int)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_5() {
    let member = class_member("getTime(string text, int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(string text, int count)");
}

#[test]
fn should_handle_public_visibility_5() {
    let member = class_member("+getTime(string text, int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime(string text, int count)");
}

#[test]
fn should_handle_private_visibility_5() {
    let member = class_member("-getTime(string text, int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime(string text, int count)");
}

#[test]
fn should_handle_protected_visibility_5() {
    let member = class_member("#getTime(string text, int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime(string text, int count)");
}

#[test]
fn should_handle_internal_visibility_5() {
    let member = class_member("~getTime(string text, int count)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime(string text, int count)");
}

#[test]
fn should_return_correct_css_for_static_classifier_5() {
    let member = class_member("getTime(string text, int count)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(string text, int count)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_5() {
    let member = class_member("getTime(string text, int count)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime(string text, int count)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_6() {
    let member = class_member("getTime() DateTime", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime() : DateTime");
}

#[test]
fn should_handle_public_visibility_6() {
    let member = class_member("+getTime() DateTime", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTime() : DateTime");
}

#[test]
fn should_handle_private_visibility_6() {
    let member = class_member("-getTime() DateTime", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTime() : DateTime");
}

#[test]
fn should_handle_protected_visibility_6() {
    let member = class_member("#getTime() DateTime", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTime() : DateTime");
}

#[test]
fn should_handle_internal_visibility_6() {
    let member = class_member("~getTime() DateTime", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTime() : DateTime");
}

#[test]
fn should_return_correct_css_for_static_classifier_6() {
    let member = class_member("getTime() DateTime$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime() : DateTime");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_6() {
    let member = class_member("getTime()  DateTime*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTime() : DateTime");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_7() {
    let member = class_member("getTimes(List~T~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>)");
}

#[test]
fn should_handle_public_visibility_7() {
    let member = class_member("+getTimes(List~T~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimes(List<T>)");
}

#[test]
fn should_handle_private_visibility_7() {
    let member = class_member("-getTimes(List~T~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimes(List<T>)");
}

#[test]
fn should_handle_protected_visibility_7() {
    let member = class_member("#getTimes(List~T~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimes(List<T>)");
}

#[test]
fn should_handle_internal_visibility_7() {
    let member = class_member("~getTimes(List~T~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimes(List<T>)");
}

#[test]
fn should_return_correct_css_for_static_classifier_7() {
    let member = class_member("getTimes(List~T~)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_7() {
    let member = class_member("getTimes(List~T~)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_8() {
    let member = class_member("getTimes(List~T~, List~OT~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>, List<OT>)");
}

#[test]
fn should_handle_public_visibility_8() {
    let member = class_member("+getTimes(List~T~, List~OT~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimes(List<T>, List<OT>)");
}

#[test]
fn should_handle_private_visibility_8() {
    let member = class_member("-getTimes(List~T~, List~OT~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimes(List<T>, List<OT>)");
}

#[test]
fn should_handle_protected_visibility_8() {
    let member = class_member("#getTimes(List~T~, List~OT~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimes(List<T>, List<OT>)");
}

#[test]
fn should_handle_internal_visibility_8() {
    let member = class_member("~getTimes(List~T~, List~OT~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimes(List<T>, List<OT>)");
}

#[test]
fn should_return_correct_css_for_static_classifier_8() {
    let member = class_member("getTimes(List~T~, List~OT~)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>, List<OT>)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_8() {
    let member = class_member("getTimes(List~T~, List~OT~)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<T>, List<OT>)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_9() {
    let member = class_member("getTimetableList(List~List~T~~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList(List<List<T>>)");
}

#[test]
fn should_handle_public_visibility_9() {
    let member = class_member("+getTimetableList(List~List~T~~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimetableList(List<List<T>>)");
}

#[test]
fn should_handle_private_visibility_9() {
    let member = class_member("-getTimetableList(List~List~T~~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimetableList(List<List<T>>)");
}

#[test]
fn should_handle_protected_visibility_9() {
    let member = class_member("#getTimetableList(List~List~T~~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimetableList(List<List<T>>)");
}

#[test]
fn should_handle_internal_visibility_9() {
    let member = class_member("~getTimetableList(List~List~T~~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimetableList(List<List<T>>)");
}

#[test]
fn should_return_correct_css_for_static_classifier_9() {
    let member = class_member("getTimetableList(List~List~T~~)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList(List<List<T>>)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_9() {
    let member = class_member("getTimetableList(List~List~T~~)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList(List<List<T>>)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_10() {
    let member = class_member("getTimes(List~K, V~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<K, V>)");
}

#[test]
fn should_handle_public_visibility_10() {
    let member = class_member("+getTimes(List~K, V~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimes(List<K, V>)");
}

#[test]
fn should_handle_private_visibility_10() {
    let member = class_member("-getTimes(List~K, V~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimes(List<K, V>)");
}

#[test]
fn should_handle_protected_visibility_10() {
    let member = class_member("#getTimes(List~K, V~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimes(List<K, V>)");
}

#[test]
fn should_handle_internal_visibility_10() {
    let member = class_member("~getTimes(List~K, V~)", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimes(List<K, V>)");
}

#[test]
fn should_return_correct_css_for_static_classifier_10() {
    let member = class_member("getTimes(List~K, V~)$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<K, V>)");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_10() {
    let member = class_member("getTimes(List~K, V~)*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes(List<K, V>)");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_11() {
    let member = class_member("getTimes() List~T~", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes() : List<T>");
}

#[test]
fn should_handle_public_visibility_11() {
    let member = class_member("+getTimes() List~T~", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimes() : List<T>");
}

#[test]
fn should_handle_private_visibility_11() {
    let member = class_member("-getTimes() List~T~", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimes() : List<T>");
}

#[test]
fn should_handle_protected_visibility_11() {
    let member = class_member("#getTimes() List~T~", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimes() : List<T>");
}

#[test]
fn should_handle_internal_visibility_11() {
    let member = class_member("~getTimes() List~T~", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimes() : List<T>");
}

#[test]
fn should_return_correct_css_for_static_classifier_11() {
    let member = class_member("getTimes() List~T~$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes() : List<T>");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_11() {
    let member = class_member("getTimes() List~T~*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimes() : List<T>");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn should_parse_correctly_12() {
    let member = class_member("getTimetableList() List~List~T~~", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList() : List<List<T>>");
}

#[test]
fn should_handle_public_visibility_12() {
    let member =
        class_member("+getTimetableList() List~List~T~~", true).expect("the member parses");
    assert_eq!(display_text(&member), "+getTimetableList() : List<List<T>>");
}

#[test]
fn should_handle_private_visibility_12() {
    let member =
        class_member("-getTimetableList() List~List~T~~", true).expect("the member parses");
    assert_eq!(display_text(&member), "-getTimetableList() : List<List<T>>");
}

#[test]
fn should_handle_protected_visibility_12() {
    let member =
        class_member("#getTimetableList() List~List~T~~", true).expect("the member parses");
    assert_eq!(display_text(&member), "#getTimetableList() : List<List<T>>");
}

#[test]
fn should_handle_internal_visibility_12() {
    let member =
        class_member("~getTimetableList() List~List~T~~", true).expect("the member parses");
    assert_eq!(display_text(&member), "~getTimetableList() : List<List<T>>");
}

#[test]
fn should_return_correct_css_for_static_classifier_12() {
    let member =
        class_member("getTimetableList() List~List~T~~$", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList() : List<List<T>>");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_return_correct_css_for_abstract_classifier_12() {
    let member =
        class_member("getTimetableList() List~List~T~~*", true).expect("the member parses");
    assert_eq!(display_text(&member), "getTimetableList() : List<List<T>>");
    assert_eq!(css_style(&member), "font-style:italic;");
}

#[test]
fn member_name_should_handle_double_colons() {
    let member = class_member("std::map ~int,string~ pMap;", false).expect("the member parses");
    assert_eq!(display_text(&member), "std::map <int,string> pMap;");
}

#[test]
fn member_name_should_handle_generic_type() {
    let member =
        class_member("getTime~T~(this T, int seconds)$ DateTime", true).expect("the member parses");
    assert_eq!(
        display_text(&member),
        "getTime<T>(this T, int seconds) : DateTime"
    );
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_parse_the_display_text_correctly() {
    let member = class_member("name String", false).expect("the member parses");
    assert_eq!(display_text(&member), "name String");
    assert_eq!(css_style(&member), "");
}

#[test]
fn should_parse_the_display_text_correctly_2() {
    let member = class_member("+name String", false).expect("the member parses");
    assert_eq!(display_text(&member), "+name String");
    assert_eq!(css_style(&member), "");
}

#[test]
fn should_parse_the_display_text_correctly_3() {
    let member = class_member("#name String", false).expect("the member parses");
    assert_eq!(display_text(&member), "#name String");
    assert_eq!(css_style(&member), "");
}

#[test]
fn should_parse_the_display_text_correctly_4() {
    let member = class_member("-name String", false).expect("the member parses");
    assert_eq!(display_text(&member), "-name String");
    assert_eq!(css_style(&member), "");
}

#[test]
fn should_parse_the_display_text_correctly_5() {
    let member = class_member("~name String", false).expect("the member parses");
    assert_eq!(display_text(&member), "~name String");
    assert_eq!(css_style(&member), "");
}

#[test]
fn should_parse_the_display_text_correctly_and_apply_static_css_style() {
    let member = class_member("name String$", false).expect("the member parses");
    assert_eq!(display_text(&member), "name String");
    assert_eq!(css_style(&member), "text-decoration:underline;");
}

#[test]
fn should_parse_the_display_text_correctly_and_apply_abstract_css_style() {
    let member = class_member("name String*", false).expect("the member parses");
    assert_eq!(display_text(&member), "name String");
    assert_eq!(css_style(&member), "font-style:italic;");
}
