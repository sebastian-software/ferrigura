//! The class DB (`classDb.ts`): the state the grammar's actions change, and the setters the
//! reference runs. The rules and the quirks are in `docs-port/class/PLAN.md`, sections 4 to 8.
//!
//! A JavaScript exception of the reference (a `!` lookup that finds nothing, a member that cannot
//! be parsed) is a [`DbError`]: the input is rejected, as the reference rejects it.

use std::collections::HashMap;

use ferrigura_model::class::{
    ClassDiagram, ClassInterface, ClassMember, ClassNamespace, ClassNode, ClassNote, ClassRelation,
    ClassStyle, LineType, MemberType, RelationEnd,
};

use super::member::{Member, parse_member};
use super::{Options, SecurityLevel};
use crate::sanitize::sanitize_text;
use crate::text::{is_js_space, js_trim};
use crate::url::format_url;

/// The prefix the reference puts on an id that starts with a digit (`MERMAID_DOM_ID_PREFIX`).
const CLASS_ID_PREFIX: &str = "classId-";

/// A JavaScript exception of the reference, with the message it would give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub String);

/// A map that keeps its insertion order, with the semantics of JavaScript's `Map`: `set` on a key
/// that exists replaces the value and keeps its place.
#[derive(Debug, Clone)]
pub(crate) struct OrderedMap<V> {
    entries: Vec<(String, V)>,
    index: HashMap<String, usize>,
}

impl<V> OrderedMap<V> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let position = *self.index.get(key)?;
        self.entries.get_mut(position).map(|(_, value)| value)
    }

    fn contains(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    fn set(&mut self, key: String, value: V) {
        match self.index.get(&key) {
            Some(&position) => {
                if let Some(entry) = self.entries.get_mut(position) {
                    entry.1 = value;
                }
            }
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
            }
        }
    }

    fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, value)| value)
    }

    fn into_values(self) -> Vec<V> {
        self.entries.into_iter().map(|(_, value)| value).collect()
    }
}

/// A relation as the grammar gives it: the ends, the line, and the cardinalities and label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawRelation {
    pub id1: String,
    pub id2: String,
    pub type1: RelationEnd,
    pub type2: RelationEnd,
    pub line_type: LineType,
    pub title1: String,
    pub title2: String,
    pub title: Option<String>,
}

/// The class DB: the state of one diagram, as the grammar's actions build it.
#[derive(Debug, Clone)]
pub(crate) struct ClassDb {
    options: Options,
    title: String,
    acc_title: String,
    acc_descr: String,
    direction: String,
    classes: OrderedMap<ClassNode>,
    relations: Vec<ClassRelation>,
    notes: Vec<ClassNote>,
    interfaces: Vec<ClassInterface>,
    namespaces: OrderedMap<ClassNamespace>,
    namespace_stack: Vec<String>,
    style_classes: OrderedMap<ClassStyle>,
}

impl ClassDb {
    /// An empty DB, as `clear()` leaves it, with the sanitising options of the parse.
    pub(crate) fn new(options: Options) -> Self {
        Self {
            options,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: "TB".to_owned(),
            classes: OrderedMap::new(),
            relations: Vec::new(),
            notes: Vec::new(),
            interfaces: Vec::new(),
            namespaces: OrderedMap::new(),
            namespace_stack: Vec::new(),
            style_classes: OrderedMap::new(),
        }
    }

    /// The diagram title from the front matter (`setDiagramTitle`, which sanitises).
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = self.sanitize(text);
    }

    /// `setAccTitle` of `commonDb.ts`: sanitised, then the leading whitespace removed.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = self
            .sanitize(text)
            .trim_start_matches(is_js_space)
            .to_owned();
    }

    /// `setAccDescription` of `commonDb.ts`: sanitised, then each line break and the whitespace
    /// after it are replaced by one line break.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        let sanitized = self.sanitize(text);
        let mut out = String::with_capacity(sanitized.len());
        let mut chars = sanitized.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\n' {
                out.push('\n');
                while chars.peek().is_some_and(|&next| is_js_space(next)) {
                    chars.next();
                }
            } else {
                out.push(c);
            }
        }
        self.acc_descr = out;
    }

    pub(crate) fn set_direction(&mut self, direction: &str) {
        self.direction = direction.to_owned();
    }

    /// `sanitizeText(text, config)` of `common.ts`, with the configuration of the parse.
    fn sanitize(&self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        sanitize_text(&self.sanitize_more(text))
    }

    /// `sanitizeMore` of `common.ts`: only with `htmlLabels`; the strict levels leave the text to
    /// the DOM step that follows, `loose` changes nothing, and any other level turns the markup
    /// into escaped text, with the line breaks kept as `<br/>`.
    fn sanitize_more(&self, text: &str) -> String {
        if !self.options.html_labels {
            return text.to_owned();
        }
        match self.options.security_level {
            SecurityLevel::Strict | SecurityLevel::Antiscript | SecurityLevel::Sandbox => {
                text.to_owned()
            }
            SecurityLevel::Loose => text.to_owned(),
            SecurityLevel::Other => {
                let breaks = break_to_placeholder(text);
                let escaped = breaks.replace('<', "&lt;").replace('>', "&gt;");
                escaped.replace('=', "&equals;").replace("#br#", "<br/>")
            }
        }
    }

    /// `splitClassNameAndType`: the name and the generic type, from the text between the first two
    /// `~`. The id is sanitised first, and so are the two parts.
    fn split(&self, id: &str) -> (String, String) {
        let id = self.sanitize(id);
        if id.find('~').is_some_and(|index| index > 0) {
            let mut parts = id.split('~');
            let first = parts.next().unwrap_or("");
            let second = parts.next().unwrap_or("");
            (self.sanitize(first), self.sanitize(second))
        } else {
            (id, String::new())
        }
    }

    /// `addClass`: adds the class unless its name is known. The class is stored under its
    /// sanitised name.
    pub(crate) fn add_class(&mut self, id: &str) {
        let id = self.sanitize(id);
        let (class_name, class_type) = self.split(&id);
        if self.classes.contains(&class_name) {
            return;
        }
        let name = self.sanitize(&class_name);
        let text = if class_type.is_empty() {
            name.clone()
        } else {
            format!("{name}&lt;{class_type}&gt;")
        };
        let node = ClassNode {
            id: name.clone(),
            class_type,
            label: name.clone(),
            text,
            css_classes: "default".to_owned(),
            annotations: Vec::new(),
            members: Vec::new(),
            methods: Vec::new(),
            styles: Vec::new(),
            parent: None,
            link: None,
            link_target: None,
            tooltip: None,
            have_callback: None,
        };
        self.classes.set(name, node);
    }

    /// `setClassLabel`: the label is sanitised when it is not empty, and the text shows the
    /// generic type in raw angle brackets. A class that is not known is an exception.
    pub(crate) fn set_class_label(&mut self, id: &str, label: &str) -> Result<(), DbError> {
        let id = self.sanitize(id);
        let label = if label.is_empty() {
            String::new()
        } else {
            self.sanitize(label)
        };
        let (class_name, _) = self.split(&id);
        let class = self
            .classes
            .get_mut(&class_name)
            .ok_or_else(|| missing(&class_name))?;
        class.label = label.clone();
        class.text = match class.class_type.as_str() {
            "" => label,
            class_type => format!("{label}<{class_type}>"),
        };
        Ok(())
    }

    /// `addInterface`: a lollipop's interface, in order.
    fn add_interface(&mut self, label: &str, class_id: &str) {
        self.interfaces.push(ClassInterface {
            id: format!("interface{}", self.interfaces.len()),
            label: label.to_owned(),
            class_id: class_id.to_owned(),
        });
    }

    /// `addRelation`: a lollipop end becomes an interface when the other end has no marker; the
    /// classes at both ends are added; the ids are reduced to their names, and the cardinalities
    /// are sanitised and trimmed.
    pub(crate) fn add_relation(&mut self, relation: RawRelation) {
        let mut id1 = relation.id1;
        let mut id2 = relation.id2;
        if relation.type1 == RelationEnd::Lollipop && relation.type2 == RelationEnd::None {
            self.add_class(&id2);
            self.add_interface(&id1, &id2);
            id1 = format!("interface{}", self.interfaces.len().saturating_sub(1));
        } else if relation.type2 == RelationEnd::Lollipop && relation.type1 == RelationEnd::None {
            self.add_class(&id1);
            self.add_interface(&id2, &id1);
            id2 = format!("interface{}", self.interfaces.len().saturating_sub(1));
        } else {
            self.add_class(&id1);
            self.add_class(&id2);
        }
        let id1 = self.split(&id1).0;
        let id2 = self.split(&id2).0;
        let relation_title1 = self.sanitize(js_trim(&relation.title1));
        let relation_title2 = self.sanitize(js_trim(&relation.title2));
        self.relations.push(ClassRelation {
            id1,
            id2,
            relation_title1,
            relation_title2,
            title: relation.title,
            type1: relation.type1,
            type2: relation.type2,
            line_type: relation.line_type,
        });
    }

    /// `addAnnotation`: the annotation is kept as the grammar gives it.
    pub(crate) fn add_annotation(
        &mut self,
        class_name: &str,
        annotation: &str,
    ) -> Result<(), DbError> {
        let (validated, _) = self.split(class_name);
        let class = self
            .classes
            .get_mut(&validated)
            .ok_or_else(|| missing(&validated))?;
        class.annotations.push(annotation.to_owned());
        Ok(())
    }

    /// `addMember`: adds the class if needed, then the member: an annotation when the trimmed text
    /// is `<<…>>`, a method when it has a `)` after its first character, an attribute when it is
    /// not empty, and nothing otherwise.
    pub(crate) fn add_member(&mut self, class_name: &str, member: &str) -> Result<(), DbError> {
        self.add_class(class_name);
        let (validated, _) = self.split(class_name);
        let text = js_trim(member);
        if text.is_empty() {
            return Ok(());
        }
        if text.starts_with("<<") && text.ends_with(">>") && text.len() >= 4 {
            let annotation = self.sanitize(&text[2..text.len() - 2]);
            let class = self
                .classes
                .get_mut(&validated)
                .ok_or_else(|| missing(&validated))?;
            class.annotations.push(annotation);
            return Ok(());
        }
        if text.find(')').is_some_and(|index| index > 0) {
            let parsed = parse_member(&self.sanitize(text), true)
                .ok_or_else(|| DbError(format!("cannot read the method '{text}'")))?;
            let class = self
                .classes
                .get_mut(&validated)
                .ok_or_else(|| missing(&validated))?;
            class.methods.push(model_member(parsed));
        } else {
            let parsed = parse_member(&self.sanitize(text), false)
                .ok_or_else(|| DbError(format!("cannot read the attribute '{text}'")))?;
            let class = self
                .classes
                .get_mut(&validated)
                .ok_or_else(|| missing(&validated))?;
            class.members.push(model_member(parsed));
        }
        Ok(())
    }

    /// `addMembers`: the members in the order they are written (the grammar's reversal and the
    /// DB's reversal cancel out).
    pub(crate) fn add_members(
        &mut self,
        class_name: &str,
        members: &[String],
    ) -> Result<(), DbError> {
        for member in members {
            self.add_member(class_name, member)?;
        }
        Ok(())
    }

    /// `addNote`: a note with the id `note<index>`. The text is kept as written.
    pub(crate) fn add_note(&mut self, text: &str, class: Option<&str>) -> String {
        let index = self.notes.len();
        let id = format!("note{index}");
        self.notes.push(ClassNote {
            id: id.clone(),
            class: class.map(str::to_owned),
            text: text.to_owned(),
            index,
            parent: None,
        });
        id
    }

    /// `cleanupLabel`: one leading colon is dropped, then the text is trimmed and sanitised.
    pub(crate) fn clean_up_label(&self, label: &str) -> String {
        let without_colon = label.strip_prefix(':').unwrap_or(label);
        self.sanitize(js_trim(without_colon))
    }

    /// `setCssClass`: each id (split at commas) gets a space and the class name appended to its
    /// `cssClasses`. An id that starts with a digit is looked up with the `classId-` prefix.
    pub(crate) fn set_css_class(&mut self, ids: &str, class_name: &str) {
        for piece in ids.split(',') {
            let id = prefixed_if_digit(piece);
            let (name, _) = self.split(&id);
            if let Some(class) = self.classes.get_mut(&name) {
                class.css_classes.push(' ');
                class.css_classes.push_str(class_name);
            }
        }
    }

    /// `defineClass`: the style class gets its declarations, and every class whose `cssClasses`
    /// contains the style class's id (as a substring) gets the declarations split at commas.
    pub(crate) fn define_class(&mut self, ids: &[String], styles: &[String]) {
        let split_styles: Vec<String> = styles
            .iter()
            .flat_map(|style| style.split(',').map(str::to_owned))
            .collect();
        for id in ids {
            if !self.style_classes.contains(id) {
                self.style_classes.set(
                    id.clone(),
                    ClassStyle {
                        id: id.clone(),
                        styles: Vec::new(),
                    },
                );
            }
            if let Some(style_class) = self.style_classes.get_mut(id) {
                style_class.styles.extend(styles.iter().cloned());
            }
            for class in self.classes.values_mut() {
                if class.css_classes.contains(id.as_str()) {
                    class.styles.extend(split_styles.iter().cloned());
                }
            }
        }
    }

    /// `setTooltip`: the tooltip of each named class, sanitised.
    pub(crate) fn set_tooltip(&mut self, ids: &str, tooltip: &str) {
        let tooltip = self.sanitize(tooltip);
        for piece in ids.split(',') {
            let (name, _) = self.split(piece);
            if let Some(class) = self.classes.get_mut(&name) {
                class.tooltip = Some(tooltip.clone());
            }
        }
    }

    /// `setLink`: the link of each named class (through `formatUrl`), its target (`_top` under
    /// `sandbox`, else the sanitised target, else `_blank`), and the `clickable` class.
    pub(crate) fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        let loose = self.options.security_level == SecurityLevel::Loose;
        let sandbox = self.options.security_level == SecurityLevel::Sandbox;
        let target = if sandbox {
            "_top".to_owned()
        } else if let Some(target) = target {
            self.sanitize(target)
        } else {
            "_blank".to_owned()
        };
        let formatted = format_url(link, loose);
        for piece in ids.split(',') {
            let id = prefixed_if_digit(piece);
            let (name, _) = self.split(&id);
            if let Some(class) = self.classes.get_mut(&name) {
                class.link = formatted.clone();
                class.link_target = Some(target.clone());
            }
        }
        self.set_css_class(ids, "clickable");
    }

    /// `setClickEvent` without the callback: the class is marked to have one, and it is
    /// `clickable`. The function itself only runs under `loose` in the browser, and is not kept.
    pub(crate) fn set_click_event(&mut self, ids: &str) {
        for piece in ids.split(',') {
            let (name, _) = self.split(piece);
            if let Some(class) = self.classes.get_mut(&name) {
                class.have_callback = Some(true);
            }
        }
        self.set_css_class(ids, "clickable");
    }

    /// `addNamespace`: the qualified name of the namespace is pushed on the stack. A namespace
    /// that is new gets its missing ancestors, the leaf is explicit, and a label replaces the
    /// name when it is not empty.
    pub(crate) fn add_namespace(&mut self, id: &str, label: Option<&str>) -> String {
        let qualified = match self.namespace_stack.last() {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}.{id}"),
            _ => id.to_owned(),
        };
        self.namespace_stack.push(qualified.clone());
        let label = label.filter(|label| !label.is_empty());
        if let Some(existing) = self.namespaces.get_mut(&qualified) {
            existing.explicit = true;
            if let Some(label) = label {
                existing.label = label.to_owned();
            }
            return qualified;
        }
        let parts: Vec<&str> = qualified.split('.').collect();
        let mut ancestors: Vec<String> = Vec::with_capacity(parts.len());
        for (index, part) in parts.iter().enumerate() {
            let ancestor = match ancestors.last() {
                Some(previous) if index > 0 => format!("{previous}.{part}"),
                _ => (*part).to_owned(),
            };
            ancestors.push(ancestor);
        }
        let leaf = ancestors.len().saturating_sub(1);
        for (index, current) in ancestors.iter().enumerate() {
            let parent = (index > 0).then(|| ancestors[index - 1].clone());
            let is_leaf = index == leaf;
            if let Some(existing) = self.namespaces.get_mut(current) {
                if is_leaf {
                    existing.explicit = true;
                }
            } else {
                let node_label = match label {
                    Some(label) if is_leaf => label.to_owned(),
                    _ => parts.get(index).copied().unwrap_or("").to_owned(),
                };
                self.namespaces.set(
                    current.clone(),
                    ClassNamespace {
                        id: current.clone(),
                        label: node_label,
                        parent: parent.clone(),
                        explicit: is_leaf,
                        classes: Vec::new(),
                        notes: Vec::new(),
                    },
                );
            }
            if let Some(parent) = parent
                && let Some(child) = self.namespaces.get_mut(current)
                && child.parent.is_none()
            {
                child.parent = Some(parent);
            }
        }
        qualified
    }

    /// `popNamespace`.
    pub(crate) fn pop_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    /// `addClassesToNamespace`: the classes and notes declared in the namespace get it as their
    /// parent, and are listed in it. A namespace that is not known does nothing.
    pub(crate) fn add_classes_to_namespace(
        &mut self,
        id: &str,
        class_names: &[String],
        note_names: &[String],
    ) -> Result<(), DbError> {
        if !self.namespaces.contains(id) {
            return Ok(());
        }
        for name in class_names {
            let (class_name, _) = self.split(name);
            let class = self
                .classes
                .get_mut(&class_name)
                .ok_or_else(|| missing(&class_name))?;
            class.parent = Some(id.to_owned());
            if let Some(namespace) = self.namespaces.get_mut(id)
                && !namespace.classes.contains(&class_name)
            {
                namespace.classes.push(class_name);
            }
        }
        for name in note_names {
            let note = self
                .notes
                .iter_mut()
                .find(|note| note.id == *name)
                .ok_or_else(|| missing(name))?;
            note.parent = Some(id.to_owned());
            if let Some(namespace) = self.namespaces.get_mut(id)
                && !namespace.notes.contains(name)
            {
                namespace.notes.push(name.clone());
            }
        }
        Ok(())
    }

    /// `setCssStyle`: the declarations of a class (looked up by the id as written), split at
    /// commas where a declaration has one.
    pub(crate) fn set_css_style(&mut self, id: &str, styles: &[String]) {
        let Some(class) = self.classes.get_mut(id) else {
            return;
        };
        for style in styles {
            if style.contains(',') {
                class.styles.extend(style.split(',').map(str::to_owned));
            } else {
                class.styles.push(style.clone());
            }
        }
    }

    /// The model of the DB's state.
    pub(crate) fn into_model(self) -> ClassDiagram {
        let mut diagram = ClassDiagram::new();
        diagram.title = self.title;
        diagram.acc_title = self.acc_title;
        diagram.acc_descr = self.acc_descr;
        diagram.direction = self.direction;
        diagram.classes = self.classes.into_values();
        diagram.relations = self.relations;
        diagram.notes = self.notes;
        diagram.interfaces = self.interfaces;
        diagram.namespaces = self.namespaces.into_values();
        diagram.style_classes = self.style_classes.into_values();
        diagram
    }
}

/// The error of a `!` lookup that finds no class or note: the JavaScript TypeError.
fn missing(name: &str) -> DbError {
    DbError(format!("Cannot read properties of undefined ('{name}')"))
}

/// The id with the `classId-` prefix when it starts with an ASCII digit, as `setCssClass`,
/// `setLink` and `setClickEvent` look it up.
fn prefixed_if_digit(piece: &str) -> String {
    if piece.starts_with(|c: char| c.is_ascii_digit()) {
        format!("{CLASS_ID_PREFIX}{piece}")
    } else {
        piece.to_owned()
    }
}

/// `breakToPlaceholder`: `<br>`, `</br>`, `<br/>` and the like (case-insensitive) become `#br#`.
fn break_to_placeholder(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('<') {
        out.push_str(&rest[..index]);
        let from = &rest[index..];
        match break_len(from) {
            Some(len) => {
                out.push_str("#br#");
                rest = &from[len..];
            }
            None => {
                out.push('<');
                rest = &from[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The length of a `<\/?br\s*\/?>` match at the start of `text`, if there is one.
fn break_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 1;
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    let word = text.get(index..index + 2)?;
    if !word.eq_ignore_ascii_case("br") {
        return None;
    }
    index += 2;
    let spaces = text[index..]
        .chars()
        .take_while(|&c| is_js_space(c))
        .map(char::len_utf8)
        .sum::<usize>();
    index += spaces;
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    (bytes.get(index) == Some(&b'>')).then_some(index + 1)
}

/// `new ClassMember(input, type)` of the reference: the text is sanitised, then parsed as the
/// given member type. `None` where the reference throws.
pub(crate) fn member_model(options: Options, input: &str, is_method: bool) -> Option<ClassMember> {
    let db = ClassDb::new(options);
    let parsed = parse_member(&db.sanitize(input), is_method)?;
    Some(model_member(parsed))
}

/// The model's member for a parsed member.
fn model_member(member: Member) -> ClassMember {
    ClassMember {
        member_type: if member.is_method {
            MemberType::Method
        } else {
            MemberType::Attribute
        },
        visibility: member.visibility,
        classifier: member.classifier,
        id: member.id,
        parameters: member.parameters,
        return_type: member.return_type,
        text: member.text,
    }
}
