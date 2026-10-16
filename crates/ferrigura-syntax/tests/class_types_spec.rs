//! Port of packages/mermaid/src/diagrams/class/classTypes.spec.ts (762 lines). Test names follow the
//! spec's titles in snake_case; a title that more than one test uses gets `_2`, `_3`, and so on.
//!
//! Every test reads its result through `getDisplayDetails()`. Its `displayText` is the HTML label form
//! of a member, and its `cssStyle` is the style of a classifier, which the model keeps only as the
//! classifier. Both are rendering, so every test is ignored and none runs:
//!
//! - `displayText is the HTML label form (rendering)`: the 61 tests that check `displayText` only.
//! - `cssStyle is rendering (the model keeps the classifier)`: the 32 tests that check `cssStyle`:
//!   the classifier tests of the methods, the generic-type test, and every attribute test.
//!
//! The spec asserts no model field directly (`visibility`, `classifier`, `return_type`, `parameters`
//! or `text`), and no `ClassMember` throws, so the file calls neither `class_member` nor the parser.

// given text representing a method / when method has no parameters
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly() {
    // Input "getTime()": displayText "getTime()".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility() {
    // Input "+getTime()": displayText "+getTime()".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility() {
    // Input "-getTime()": displayText "-getTime()".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility() {
    // Input "#getTime()": displayText "#getTime()".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility() {
    // Input "~getTime()": displayText "~getTime()".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier() {
    // Input "getTime()$": displayText "getTime()", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier() {
    // Input "getTime()*": displayText "getTime()", cssStyle "font-style:italic;".
}

// given text representing a method / when method has single parameter value
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_2() {
    // Input "getTime(int)": displayText "getTime(int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_2() {
    // Input "+getTime(int)": displayText "+getTime(int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_2() {
    // Input "-getTime(int)": displayText "-getTime(int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_2() {
    // Input "#getTime(int)": displayText "#getTime(int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_2() {
    // Input "~getTime(int)": displayText "~getTime(int)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_2() {
    // Input "getTime(int)$": displayText "getTime(int)", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_2() {
    // Input "getTime(int)*": displayText "getTime(int)", cssStyle "font-style:italic;".
}

// given text representing a method / when method has single parameter type and name (type first)
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_3() {
    // Input "getTime(int count)": displayText "getTime(int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_3() {
    // Input "+getTime(int count)": displayText "+getTime(int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_3() {
    // Input "-getTime(int count)": displayText "-getTime(int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_3() {
    // Input "#getTime(int count)": displayText "#getTime(int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_3() {
    // Input "~getTime(int count)": displayText "~getTime(int count)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_3() {
    // Input "getTime(int count)$": displayText "getTime(int count)", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_3() {
    // Input "getTime(int count)*": displayText "getTime(int count)", cssStyle "font-style:italic;".
}

// given text representing a method / when method has single parameter type and name (name first)
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_4() {
    // Input "getTime(count int)": displayText "getTime(count int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_4() {
    // Input "+getTime(count int)": displayText "+getTime(count int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_4() {
    // Input "-getTime(count int)": displayText "-getTime(count int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_4() {
    // Input "#getTime(count int)": displayText "#getTime(count int)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_4() {
    // Input "~getTime(count int)": displayText "~getTime(count int)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_4() {
    // Input "getTime(count int)$": displayText "getTime(count int)", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_4() {
    // Input "getTime(count int)*": displayText "getTime(count int)", cssStyle "font-style:italic;".
}

// given text representing a method / when method has multiple parameters
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_5() {
    // Input "getTime(string text, int count)": displayText "getTime(string text, int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_5() {
    // Input "+getTime(string text, int count)": displayText "+getTime(string text, int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_5() {
    // Input "-getTime(string text, int count)": displayText "-getTime(string text, int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_5() {
    // Input "#getTime(string text, int count)": displayText "#getTime(string text, int count)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_5() {
    // Input "~getTime(string text, int count)": displayText "~getTime(string text, int count)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_5() {
    // Input "getTime(string text, int count)$": displayText "getTime(string text, int count)",
    // cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_5() {
    // Input "getTime(string text, int count)*": displayText "getTime(string text, int count)",
    // cssStyle "font-style:italic;".
}

// given text representing a method / when method has return type
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_6() {
    // Input "getTime() DateTime": displayText "getTime() : DateTime".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_6() {
    // Input "+getTime() DateTime": displayText "+getTime() : DateTime".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_6() {
    // Input "-getTime() DateTime": displayText "-getTime() : DateTime".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_6() {
    // Input "#getTime() DateTime": displayText "#getTime() : DateTime".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_6() {
    // Input "~getTime() DateTime": displayText "~getTime() : DateTime".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_6() {
    // Input "getTime() DateTime$": displayText "getTime() : DateTime", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_6() {
    // Input "getTime()  DateTime*" (two spaces): displayText "getTime() : DateTime", cssStyle "font-style:italic;".
}

// given text representing a method / when method parameter is generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_7() {
    // Input "getTimes(List~T~)": displayText "getTimes(List<T>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_7() {
    // Input "+getTimes(List~T~)": displayText "+getTimes(List<T>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_7() {
    // Input "-getTimes(List~T~)": displayText "-getTimes(List<T>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_7() {
    // Input "#getTimes(List~T~)": displayText "#getTimes(List<T>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_7() {
    // Input "~getTimes(List~T~)": displayText "~getTimes(List<T>)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_7() {
    // Input "getTimes(List~T~)$": displayText "getTimes(List<T>)", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_7() {
    // Input "getTimes(List~T~)*": displayText "getTimes(List<T>)", cssStyle "font-style:italic;".
}

// given text representing a method / when method parameter contains two generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_8() {
    // Input "getTimes(List~T~, List~OT~)": displayText "getTimes(List<T>, List<OT>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_8() {
    // Input "+getTimes(List~T~, List~OT~)": displayText "+getTimes(List<T>, List<OT>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_8() {
    // Input "-getTimes(List~T~, List~OT~)": displayText "-getTimes(List<T>, List<OT>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_8() {
    // Input "#getTimes(List~T~, List~OT~)": displayText "#getTimes(List<T>, List<OT>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_8() {
    // Input "~getTimes(List~T~, List~OT~)": displayText "~getTimes(List<T>, List<OT>)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_8() {
    // Input "getTimes(List~T~, List~OT~)$": displayText "getTimes(List<T>, List<OT>)",
    // cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_8() {
    // Input "getTimes(List~T~, List~OT~)*": displayText "getTimes(List<T>, List<OT>)",
    // cssStyle "font-style:italic;".
}

// given text representing a method / when method parameter is a nested generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_9() {
    // Input "getTimetableList(List~List~T~~)": displayText "getTimetableList(List<List<T>>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_9() {
    // Input "+getTimetableList(List~List~T~~)": displayText "+getTimetableList(List<List<T>>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_9() {
    // Input "-getTimetableList(List~List~T~~)": displayText "-getTimetableList(List<List<T>>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_9() {
    // Input "#getTimetableList(List~List~T~~)": displayText "#getTimetableList(List<List<T>>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_9() {
    // Input "~getTimetableList(List~List~T~~)": displayText "~getTimetableList(List<List<T>>)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_9() {
    // Input "getTimetableList(List~List~T~~)$": displayText "getTimetableList(List<List<T>>)",
    // cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_9() {
    // Input "getTimetableList(List~List~T~~)*": displayText "getTimetableList(List<List<T>>)",
    // cssStyle "font-style:italic;".
}

// given text representing a method / when method parameter is a composite generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_10() {
    // Input "getTimes(List~K, V~)": displayText "getTimes(List<K, V>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_10() {
    // Input "+getTimes(List~K, V~)": displayText "+getTimes(List<K, V>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_10() {
    // Input "-getTimes(List~K, V~)": displayText "-getTimes(List<K, V>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_10() {
    // Input "#getTimes(List~K, V~)": displayText "#getTimes(List<K, V>)".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_10() {
    // Input "~getTimes(List~K, V~)": displayText "~getTimes(List<K, V>)".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_10() {
    // Input "getTimes(List~K, V~)$": displayText "getTimes(List<K, V>)", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_10() {
    // Input "getTimes(List~K, V~)*": displayText "getTimes(List<K, V>)", cssStyle "font-style:italic;".
}

// given text representing a method / when method return type is generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_11() {
    // Input "getTimes() List~T~": displayText "getTimes() : List<T>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_11() {
    // Input "+getTimes() List~T~": displayText "+getTimes() : List<T>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_11() {
    // Input "-getTimes() List~T~": displayText "-getTimes() : List<T>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_11() {
    // Input "#getTimes() List~T~": displayText "#getTimes() : List<T>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_11() {
    // Input "~getTimes() List~T~": displayText "~getTimes() : List<T>".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_11() {
    // Input "getTimes() List~T~$": displayText "getTimes() : List<T>", cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_11() {
    // Input "getTimes() List~T~*": displayText "getTimes() : List<T>", cssStyle "font-style:italic;".
}

// given text representing a method / when method return type is a nested generic
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_parse_correctly_12() {
    // Input "getTimetableList() List~List~T~~": displayText "getTimetableList() : List<List<T>>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_public_visibility_12() {
    // Input "+getTimetableList() List~List~T~~": displayText "+getTimetableList() : List<List<T>>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_private_visibility_12() {
    // Input "-getTimetableList() List~List~T~~": displayText "-getTimetableList() : List<List<T>>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_protected_visibility_12() {
    // Input "#getTimetableList() List~List~T~~": displayText "#getTimetableList() : List<List<T>>".
}

#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn should_handle_internal_visibility_12() {
    // Input "~getTimetableList() List~List~T~~": displayText "~getTimetableList() : List<List<T>>".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_static_classifier_12() {
    // Input "getTimetableList() List~List~T~~$": displayText "getTimetableList() : List<List<T>>",
    // cssStyle "text-decoration:underline;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_return_correct_css_for_abstract_classifier_12() {
    // Input "getTimetableList() List~List~T~~*": displayText "getTimetableList() : List<List<T>>",
    // cssStyle "font-style:italic;".
}

// given text representing a method / --uncategorized tests--
#[test]
#[ignore = "port: displayText is the HTML label form (rendering)"]
fn member_name_should_handle_double_colons() {
    // Attribute "std::map ~int,string~ pMap;": displayText "std::map <int,string> pMap;".
}

#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn member_name_should_handle_generic_type() {
    // Method "getTime~T~(this T, int seconds)$ DateTime": displayText
    // "getTime<T>(this T, int seconds) : DateTime", cssStyle "text-decoration:underline;".
}

// given text representing an attribute / when the attribute has no modifiers
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly() {
    // Input "name String": displayText "name String", cssStyle "".
}

// given text representing an attribute / when the attribute has public "+" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_2() {
    // Input "+name String": displayText "+name String", cssStyle "".
}

// given text representing an attribute / when the attribute has protected "#" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_3() {
    // Input "#name String": displayText "#name String", cssStyle "".
}

// given text representing an attribute / when the attribute has private "-" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_4() {
    // Input "-name String": displayText "-name String", cssStyle "".
}

// given text representing an attribute / when the attribute has internal "~" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_5() {
    // Input "~name String": displayText "~name String", cssStyle "".
}

// given text representing an attribute / when the attribute has static "$" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_and_apply_static_css_style() {
    // Input "name String$": displayText "name String", cssStyle "text-decoration:underline;".
}

// given text representing an attribute / when the attribute has abstract "*" modifier
#[test]
#[ignore = "port: cssStyle is rendering (the model keeps the classifier)"]
fn should_parse_the_display_text_correctly_and_apply_abstract_css_style() {
    // Input "name String*": displayText "name String", cssStyle "font-style:italic;".
}
