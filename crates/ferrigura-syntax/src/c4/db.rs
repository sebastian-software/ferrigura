//! The C4 DB, `c4Db.ts`: the shapes, boundaries and relations the grammar declares, with the
//! reference's rules for a repeated alias, the named attributes and the defaults. See
//! `docs-port/c4/PLAN.md`, section 5, for the calls and what they store.
//!
//! The DB holds the model of `ferrigura-model::c4`, and a flag for each text field that a bare
//! string overwrote. The reference's `??=` keeps a bare string (it is not `null`), and the model
//! shows it as absent, so the flag is what tells the two apart.

use std::collections::BTreeMap;

use ferrigura_model::c4::{C4Boundary, C4Diagram, C4Relation, C4Shape, C4Text};
use serde_json::{Value, json};

use crate::sanitize::sanitize_text;
use crate::text::{is_js_space, newline_then_space_to_newline};

/// One argument of a call: a positional text, or a `$key="value"` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Arg {
    /// A positional argument, trimmed by the grammar.
    Text(String),
    /// A named argument: its key and value, both trimmed by the grammar.
    Named(String, String),
}

/// A call the DB does not make the way the reference does: the argument would store a JavaScript
/// object where the model holds text, or it would replace a structural field. Not ported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub(crate) String);

type DbResult = Result<(), DbError>;

/// Whether a named argument is stored the way a declaration stores it (`Construct`, which wraps
/// the text fields in `{ text }`) or the way a style statement does (`Update`, a bare string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Construct,
    Update,
}

/// The description and the technology, the two text fields that have a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextSlot {
    Techn,
    Descr,
}

/// A DB object with the flags of the text fields that a bare string overwrote.
#[derive(Debug, Clone)]
struct Entry<T> {
    value: T,
    /// The description is a bare string (not `{ text }`), so `??=` keeps it.
    bare_descr: bool,
    /// The technology is a bare string, so `??=` keeps it.
    bare_techn: bool,
}

impl<T> Entry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            bare_descr: false,
            bare_techn: false,
        }
    }

    fn is_bare(&self, slot: TextSlot) -> bool {
        match slot {
            TextSlot::Techn => self.bare_techn,
            TextSlot::Descr => self.bare_descr,
        }
    }

    fn set_bare(&mut self, slot: TextSlot, bare: bool) {
        match slot {
            TextSlot::Techn => self.bare_techn = bare,
            TextSlot::Descr => self.bare_descr = bare,
        }
    }
}

/// The text fields of a DB object, by slot.
trait Text {
    fn text_mut(&mut self, slot: TextSlot) -> &mut Option<String>;
}

impl Text for C4Shape {
    fn text_mut(&mut self, slot: TextSlot) -> &mut Option<String> {
        match slot {
            TextSlot::Techn => &mut self.techn,
            TextSlot::Descr => &mut self.descr,
        }
    }
}

impl Text for C4Boundary {
    fn text_mut(&mut self, slot: TextSlot) -> &mut Option<String> {
        match slot {
            TextSlot::Techn => &mut self.techn,
            TextSlot::Descr => &mut self.descr,
        }
    }
}

impl Text for C4Relation {
    fn text_mut(&mut self, slot: TextSlot) -> &mut Option<String> {
        match slot {
            TextSlot::Techn => &mut self.techn,
            TextSlot::Descr => &mut self.descr,
        }
    }
}

/// The named-attribute rule of one kind of DB object: `obj[key] = …` as the reference writes it.
type Named<T> = fn(&mut Entry<T>, &str, &str, Mode) -> DbResult;

/// The C4 DB state of one parse.
#[derive(Debug)]
pub(crate) struct Db {
    /// `autoWrap()`: the top-level `wrap` configuration, stored on each element.
    wrap: bool,
    title: String,
    c4_type: String,
    acc_descr: String,
    shape_in_row: f64,
    boundary_in_row: f64,
    shapes: Vec<Entry<C4Shape>>,
    boundaries: Vec<Entry<C4Boundary>>,
    rels: Vec<Entry<C4Relation>>,
    /// `currentBoundaryParse`: the alias of the boundary being declared.
    current_boundary: String,
    /// `parentBoundaryParse`.
    parent_boundary: String,
    /// `boundaryParseStack`.
    boundary_stack: Vec<String>,
}

impl Db {
    /// The DB after `clear()` and `setWrap(wrap)`, as `Diagram.fromText` leaves it.
    pub(crate) fn new(wrap: bool) -> Self {
        Self {
            wrap,
            title: String::new(),
            c4_type: String::new(),
            acc_descr: String::new(),
            shape_in_row: 4.0,
            boundary_in_row: 2.0,
            shapes: Vec::new(),
            boundaries: vec![Entry::new(C4Boundary::global())],
            rels: Vec::new(),
            current_boundary: "global".to_owned(),
            parent_boundary: String::new(),
            boundary_stack: vec![String::new()],
        }
    }

    /// `setC4Type`: the header keyword, sanitised.
    pub(crate) fn set_c4_type(&mut self, text: &str) {
        self.c4_type = sanitize_text(text);
    }

    /// `setTitle`: the title, sanitised. The grammar's `accTitle` calls it too (P26).
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccDescription` (the common DB): sanitised, with the blanks after each line break
    /// removed.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addPersonOrSystem(kind, alias, label, descr?, sprite?, tags?, link?)`.
    pub(crate) fn add_person_or_system(&mut self, kind: &str, args: &[Arg]) -> DbResult {
        let alias = identity(args.first(), "alias")?;
        let label = label_slot(args.get(1));
        let index = self.shape_position(&alias);
        let current = self.current_boundary.clone();
        let wrap = self.wrap;
        let entry = &mut self.shapes[index];
        entry.value.label = Some(label);
        text_slot(entry, TextSlot::Descr, args.get(2), named_shape)?;
        raw_slots(entry, &[args.get(3), args.get(4), args.get(5)], named_shape)?;
        entry.value.type_c4_shape = kind.to_owned();
        entry.value.parent_boundary = current;
        entry.value.wrap = wrap;
        Ok(())
    }

    /// `addContainer` and `addComponent`: `addPersonOrSystem` with the technology before the
    /// description.
    pub(crate) fn add_container(&mut self, kind: &str, args: &[Arg]) -> DbResult {
        let alias = identity(args.first(), "alias")?;
        let label = label_slot(args.get(1));
        let index = self.shape_position(&alias);
        let current = self.current_boundary.clone();
        let wrap = self.wrap;
        let entry = &mut self.shapes[index];
        entry.value.label = Some(label);
        text_slot(entry, TextSlot::Techn, args.get(2), named_shape)?;
        text_slot(entry, TextSlot::Descr, args.get(3), named_shape)?;
        raw_slots(entry, &[args.get(4), args.get(5), args.get(6)], named_shape)?;
        entry.value.wrap = wrap;
        entry.value.type_c4_shape = kind.to_owned();
        entry.value.parent_boundary = current;
        Ok(())
    }

    /// `addPersonOrSystemBoundary(alias, label, type?, tags?, link?)`: the target of `Boundary`,
    /// `Enterprise_Boundary` and `System_Boundary` (the grammar splices the kind in).
    pub(crate) fn add_person_or_system_boundary(&mut self, args: &[Arg]) -> DbResult {
        self.add_boundary(args, "system")
    }

    /// `addContainerBoundary`: the target of `Container_Boundary`, with the `container` default.
    pub(crate) fn add_container_boundary(&mut self, args: &[Arg]) -> DbResult {
        self.add_boundary(args, "container")
    }

    /// The body of both boundary declarations: a type that defaults to `default_type`, the tags
    /// and the link, then the boundary becomes the current one.
    fn add_boundary(&mut self, args: &[Arg], default_type: &str) -> DbResult {
        let alias = identity(args.first(), "alias")?;
        let label = label_slot(args.get(1));
        let index = self.boundary_position(&alias);
        let current = self.current_boundary.clone();
        let wrap = self.wrap;
        let entry = &mut self.boundaries[index];
        entry.value.label = Some(label);
        boundary_type(entry, args.get(2), default_type)?;
        boundary_tags_and_link(entry, args.get(3), args.get(4))?;
        entry.value.parent_boundary = current.clone();
        entry.value.wrap = wrap;
        self.open_boundary(alias, current);
        Ok(())
    }

    /// `addDeploymentNode(nodeType, alias, label, type?, descr?, sprite?, tags?, link?)`. The
    /// reference reads the sprite argument and never assigns it, so it is skipped here too.
    pub(crate) fn add_deployment_node(&mut self, node_type: &str, args: &[Arg]) -> DbResult {
        let alias = identity(args.first(), "alias")?;
        let label = label_slot(args.get(1));
        let index = self.boundary_position(&alias);
        let current = self.current_boundary.clone();
        let wrap = self.wrap;
        let entry = &mut self.boundaries[index];
        entry.value.label = Some(label);
        boundary_type(entry, args.get(2), "node")?;
        text_slot(entry, TextSlot::Descr, args.get(3), named_boundary)?;
        boundary_tags_and_link(entry, args.get(5), args.get(6))?;
        entry.value.node_type = Some(node_type.to_owned());
        entry.value.parent_boundary = current.clone();
        entry.value.wrap = wrap;
        self.open_boundary(alias, current);
        Ok(())
    }

    /// `popBoundaryParseStack`: the parent of the boundary being closed becomes the current one.
    pub(crate) fn pop_boundary(&mut self) {
        self.current_boundary = std::mem::take(&mut self.parent_boundary);
        self.boundary_stack.pop();
        self.parent_boundary = self.boundary_stack.pop().unwrap_or_default();
        self.boundary_stack.push(self.parent_boundary.clone());
    }

    /// The `parentBoundaryParse = current; current = alias; stack.push(parent)` that every boundary
    /// declaration ends with.
    fn open_boundary(&mut self, alias: String, current: String) {
        self.parent_boundary = current;
        self.current_boundary = alias;
        self.boundary_stack.push(self.parent_boundary.clone());
    }

    /// `addRel(type, from, to, label, techn?, descr?, sprite?, tags?, link?)`. A relation without
    /// its label is dropped, which `Rel(a, "b")` gets (P32).
    pub(crate) fn add_rel(&mut self, kind: &str, args: &[Arg]) -> DbResult {
        let (Some(from), Some(to), Some(label)) = (args.first(), args.get(1), args.get(2)) else {
            return Ok(());
        };
        let from = identity_arg(from, "from")?;
        let to = identity_arg(to, "to")?;
        let label = label_arg(label);
        let index = self.rel_position(&from, &to);
        let wrap = self.wrap;
        let entry = &mut self.rels[index];
        entry.value.relation_type = kind.to_owned();
        entry.value.from = from;
        entry.value.to = to;
        entry.value.label = Some(label);
        text_slot(entry, TextSlot::Techn, args.get(3), named_rel)?;
        text_slot(entry, TextSlot::Descr, args.get(4), named_rel)?;
        raw_slots(entry, &[args.get(5), args.get(6), args.get(7)], named_rel)?;
        entry.value.wrap = wrap;
        Ok(())
    }

    /// `updateElStyle(alias, bgColor?, fontColor?, borderColor?, shadowing?, shape?, sprite?,
    /// techn?, legendText?, legendSprite?)`. An alias that no shape or boundary has is ignored.
    pub(crate) fn update_el_style(&mut self, args: &[Arg]) -> DbResult {
        let Some(Arg::Text(alias)) = args.first() else {
            return Ok(());
        };
        let rest = args.get(1..).unwrap_or(&[]);
        if let Some(index) = self.shapes.iter().position(|e| e.value.alias == *alias) {
            return style_element(&mut self.shapes[index], rest, named_shape);
        }
        if let Some(index) = self.boundaries.iter().position(|e| e.value.alias == *alias) {
            return style_element(&mut self.boundaries[index], rest, named_boundary);
        }
        Ok(())
    }

    /// `updateRelStyle(from, to, textColor?, lineColor?, offsetX?, offsetY?)`. The first relation
    /// with the two endpoints is updated; none matching is ignored.
    pub(crate) fn update_rel_style(&mut self, args: &[Arg]) -> DbResult {
        let (Some(Arg::Text(from)), Some(Arg::Text(to))) = (args.first(), args.get(1)) else {
            return Ok(());
        };
        let Some(index) = self
            .rels
            .iter()
            .position(|e| e.value.from == *from && e.value.to == *to)
        else {
            return Ok(());
        };
        rel_style(
            &mut self.rels[index],
            args.get(2),
            args.get(3),
            args.get(4),
            args.get(5),
        )
    }

    /// `updateLayoutConfig(shapeRow, boundaryRow)`: a value below one is ignored.
    pub(crate) fn update_layout_config(&mut self, args: &[Arg]) {
        let shape_row = parse_int_arg(args.first());
        let boundary_row = parse_int_arg(args.get(1));
        if shape_row >= 1.0 {
            self.shape_in_row = shape_row;
        }
        if boundary_row >= 1.0 {
            self.boundary_in_row = boundary_row;
        }
    }

    /// The model of the parse, with the DB's objects in the order it holds them.
    pub(crate) fn into_model(self) -> C4Diagram {
        let mut model = C4Diagram::new();
        model.c4_type = self.c4_type;
        model.title = self.title;
        model.acc_descr = self.acc_descr;
        model.c4_shape_in_row = self.shape_in_row;
        model.c4_boundary_in_row = self.boundary_in_row;
        model.shapes = self.shapes.into_iter().map(|e| e.value).collect();
        model.boundaries = self.boundaries.into_iter().map(|e| e.value).collect();
        model.rels = self.rels.into_iter().map(|e| e.value).collect();
        model
    }

    /// The position of the shape with `alias`, or a new one at the end.
    fn shape_position(&mut self, alias: &str) -> usize {
        match self.shapes.iter().position(|e| e.value.alias == alias) {
            Some(index) => index,
            None => {
                self.shapes.push(Entry::new(blank_shape(alias)));
                self.shapes.len() - 1
            }
        }
    }

    /// The position of the boundary with `alias`, or a new one at the end.
    fn boundary_position(&mut self, alias: &str) -> usize {
        match self.boundaries.iter().position(|e| e.value.alias == alias) {
            Some(index) => index,
            None => {
                self.boundaries.push(Entry::new(blank_boundary(alias)));
                self.boundaries.len() - 1
            }
        }
    }

    /// The position of the relation between `from` and `to`, or a new one at the end.
    fn rel_position(&mut self, from: &str, to: &str) -> usize {
        match self
            .rels
            .iter()
            .position(|e| e.value.from == from && e.value.to == to)
        {
            Some(index) => index,
            None => {
                self.rels.push(Entry::new(blank_rel(from, to)));
                self.rels.len() - 1
            }
        }
    }
}

fn blank_shape(alias: &str) -> C4Shape {
    C4Shape {
        alias: alias.to_owned(),
        type_c4_shape: String::new(),
        label: None,
        descr: None,
        techn: None,
        sprite: None,
        tags: None,
        link: None,
        parent_boundary: String::new(),
        wrap: false,
        bg_color: None,
        font_color: None,
        border_color: None,
        shadowing: None,
        shape: None,
        legend_text: None,
        legend_sprite: None,
        extra: BTreeMap::new(),
    }
}

fn blank_boundary(alias: &str) -> C4Boundary {
    C4Boundary {
        alias: alias.to_owned(),
        label: None,
        boundary_type: None,
        node_type: None,
        descr: None,
        techn: None,
        tags: None,
        link: None,
        sprite: None,
        parent_boundary: String::new(),
        wrap: false,
        bg_color: None,
        font_color: None,
        border_color: None,
        shadowing: None,
        shape: None,
        legend_text: None,
        legend_sprite: None,
        extra: BTreeMap::new(),
    }
}

fn blank_rel(from: &str, to: &str) -> C4Relation {
    C4Relation {
        relation_type: String::new(),
        from: from.to_owned(),
        to: to.to_owned(),
        label: None,
        techn: None,
        descr: None,
        sprite: None,
        tags: None,
        link: None,
        wrap: false,
        text_color: None,
        line_color: None,
        offset_x: None,
        offset_y: None,
        extra: BTreeMap::new(),
    }
}

/// The text of an argument that names an object: the alias, the label or an endpoint. A named
/// argument there would store a JavaScript object in the DB, which is not ported.
fn identity_arg(arg: &Arg, position: &str) -> Result<String, DbError> {
    match arg {
        Arg::Text(text) => Ok(text.clone()),
        Arg::Named(key, _) => Err(not_ported(&format!(
            "a named argument (`${key}`) in the {position} position"
        ))),
    }
}

/// The alias of a declaration: its first argument, which is always present.
fn identity(arg: Option<&Arg>, position: &str) -> Result<String, DbError> {
    match arg {
        Some(arg) => identity_arg(arg, position),
        None => Err(not_ported(&format!("a declaration without its {position}"))),
    }
}

/// The label of a declaration: a named argument in the label position is stored as its object, and
/// an absent label is `""`.
fn label_slot(arg: Option<&Arg>) -> C4Text {
    match arg {
        Some(Arg::Text(text)) => C4Text::Plain(text.clone()),
        Some(Arg::Named(key, value)) => {
            C4Text::Named(BTreeMap::from([(key.clone(), value.clone())]))
        }
        None => C4Text::Plain(String::new()),
    }
}

/// The label of a relation, which is positional or named as the label slot of a declaration is.
fn label_arg(arg: &Arg) -> C4Text {
    match arg {
        Arg::Text(text) => C4Text::Plain(text.clone()),
        Arg::Named(key, value) => C4Text::Named(BTreeMap::from([(key.clone(), value.clone())])),
    }
}

fn not_ported(detail: &str) -> DbError {
    DbError(format!(
        "{detail} is not ported: the reference stores an object there"
    ))
}

/// `obj[key] = value` for a text slot: `{ text }` in a declaration, a bare string in a style
/// statement.
fn bare_or_text(mode: Mode, value: &str) -> Option<String> {
    match mode {
        Mode::Construct => Some(value.to_owned()),
        Mode::Update => None,
    }
}

/// A text slot of a declaration. An absent argument gives the default with `??=`, which keeps a
/// bare string; a positional argument is the text; a named one is stored by `named`.
fn text_slot<T: Text>(
    entry: &mut Entry<T>,
    slot: TextSlot,
    arg: Option<&Arg>,
    named: Named<T>,
) -> DbResult {
    match arg {
        None => {
            if entry.value.text_mut(slot).is_none() && !entry.is_bare(slot) {
                *entry.value.text_mut(slot) = Some(String::new());
            }
            Ok(())
        }
        Some(Arg::Text(text)) => {
            *entry.value.text_mut(slot) = Some(text.clone());
            entry.set_bare(slot, false);
            Ok(())
        }
        Some(Arg::Named(key, value)) => named(entry, key, value, Mode::Construct),
    }
}

/// The sprite, tags and link slots of a declaration: a positional value is a plain string, a named
/// one goes through `named`. An absent argument leaves the field alone.
fn raw_slots<T: Raw>(entry: &mut Entry<T>, slots: &[Option<&Arg>; 3], named: Named<T>) -> DbResult {
    for (field, arg) in T::RAW_FIELDS.iter().zip(slots) {
        match arg {
            None => {}
            Some(Arg::Text(text)) => entry.value.set_raw(field, text),
            Some(Arg::Named(key, value)) => named(entry, key, value, Mode::Construct)?,
        }
    }
    Ok(())
}

/// The plain string fields that a declaration takes positionally: the sprite, the tags and the
/// link, in that order.
trait Raw {
    const RAW_FIELDS: [&'static str; 3];
    fn set_raw(&mut self, field: &str, value: &str);
}

impl Raw for C4Shape {
    const RAW_FIELDS: [&'static str; 3] = ["sprite", "tags", "link"];

    fn set_raw(&mut self, field: &str, value: &str) {
        let value = Some(value.to_owned());
        match field {
            "sprite" => self.sprite = value,
            "tags" => self.tags = value,
            "link" => self.link = value,
            _ => {}
        }
    }
}

impl Raw for C4Relation {
    const RAW_FIELDS: [&'static str; 3] = ["sprite", "tags", "link"];

    fn set_raw(&mut self, field: &str, value: &str) {
        let value = Some(value.to_owned());
        match field {
            "sprite" => self.sprite = value,
            "tags" => self.tags = value,
            "link" => self.link = value,
            _ => {}
        }
    }
}

impl Raw for C4Boundary {
    // A boundary takes its tags and link in the same slots as a shape takes its tags and link,
    // but the grammar passes them through `boundary_tags_and_link` instead.
    const RAW_FIELDS: [&'static str; 3] = ["sprite", "tags", "link"];

    fn set_raw(&mut self, field: &str, value: &str) {
        match field {
            "tags" => self.tags = Some(Some(value.to_owned())),
            "link" => self.link = Some(Some(value.to_owned())),
            "sprite" => self.sprite = Some(value.to_owned()),
            _ => {}
        }
    }
}

/// The type slot of a boundary: positional or named, with the default when absent.
fn boundary_type(entry: &mut Entry<C4Boundary>, arg: Option<&Arg>, default_type: &str) -> DbResult {
    match arg {
        None => {
            entry.value.boundary_type = Some(default_type.to_owned());
            Ok(())
        }
        Some(Arg::Text(text)) => {
            entry.value.boundary_type = Some(text.clone());
            Ok(())
        }
        Some(Arg::Named(key, value)) => named_boundary(entry, key, value, Mode::Construct),
    }
}

/// The tags and link of a boundary. An absent argument leaves the field alone.
fn boundary_tags_and_link(
    entry: &mut Entry<C4Boundary>,
    tags: Option<&Arg>,
    link: Option<&Arg>,
) -> DbResult {
    for (field, arg) in [("tags", tags), ("link", link)] {
        match arg {
            None => {}
            Some(Arg::Text(text)) => entry.value.set_raw(field, text),
            Some(Arg::Named(key, value)) => named_boundary(entry, key, value, Mode::Construct)?,
        }
    }
    Ok(())
}

/// `UpdateElementStyle` on a shape or a boundary: the nine style values, positional or named.
fn style_element<T: Styled>(entry: &mut Entry<T>, args: &[Arg], named: Named<T>) -> DbResult {
    for (position, arg) in args.iter().enumerate() {
        let Some(name) = STYLE_POSITIONS.get(position) else {
            break;
        };
        match arg {
            Arg::Text(text) if *name == "techn" => {
                // The reference assigns the positional technology as a bare string.
                *entry.value.text_mut(TextSlot::Techn) = None;
                entry.set_bare(TextSlot::Techn, true);
            }
            Arg::Text(text) => entry.value.set_style(name, text),
            Arg::Named(key, value) => named(entry, key, value, Mode::Update)?,
        }
    }
    Ok(())
}

/// The positional arguments of `UpdateElementStyle` after the alias, in order.
const STYLE_POSITIONS: [&str; 9] = [
    "bgColor",
    "fontColor",
    "borderColor",
    "shadowing",
    "shape",
    "sprite",
    "techn",
    "legendText",
    "legendSprite",
];

/// The style fields that a shape and a boundary share, as plain strings.
trait Styled: Text {
    fn set_style(&mut self, field: &str, value: &str);
}

impl Styled for C4Shape {
    fn set_style(&mut self, field: &str, value: &str) {
        let value = Some(value.to_owned());
        match field {
            "bgColor" => self.bg_color = value,
            "fontColor" => self.font_color = value,
            "borderColor" => self.border_color = value,
            "shadowing" => self.shadowing = value,
            "shape" => self.shape = value,
            "sprite" => self.sprite = value,
            "legendText" => self.legend_text = value,
            "legendSprite" => self.legend_sprite = value,
            _ => {}
        }
    }
}

impl Styled for C4Boundary {
    fn set_style(&mut self, field: &str, value: &str) {
        let value = Some(value.to_owned());
        match field {
            "bgColor" => self.bg_color = value,
            "fontColor" => self.font_color = value,
            "borderColor" => self.border_color = value,
            "shadowing" => self.shadowing = value,
            "shape" => self.shape = value,
            "sprite" => self.sprite = value,
            "legendText" => self.legend_text = value,
            "legendSprite" => self.legend_sprite = value,
            _ => {}
        }
    }
}

/// `UpdateRelStyle`: the text colour, the line colour, and the two offsets. A positional offset,
/// or one in the offset slot, is read with `parseInt`; a named one in a colour slot is stored as
/// it is.
fn rel_style(
    entry: &mut Entry<C4Relation>,
    text_color: Option<&Arg>,
    line_color: Option<&Arg>,
    offset_x: Option<&Arg>,
    offset_y: Option<&Arg>,
) -> DbResult {
    for (field, arg) in [("textColor", text_color), ("lineColor", line_color)] {
        match arg {
            None => {}
            Some(Arg::Text(text)) => set_rel_field(&mut entry.value, field, text.clone()),
            Some(Arg::Named(key, value)) => named_rel(entry, key, value, Mode::Update)?,
        }
    }
    for (field, arg) in [("offsetX", offset_x), ("offsetY", offset_y)] {
        match arg {
            None => {}
            Some(Arg::Text(text)) => {
                set_rel_field(
                    &mut entry.value,
                    field,
                    js_number_string(js_parse_int(text)),
                );
            }
            Some(Arg::Named(key, value)) => {
                if key != "offsetX" && key != "offsetY" {
                    return Err(not_ported(&format!(
                        "a named argument (`${key}`) in an offset position"
                    )));
                }
                set_rel_field(&mut entry.value, key, js_number_string(js_parse_int(value)));
            }
        }
    }
    Ok(())
}

/// A plain string field of a relation that `UpdateRelStyle` sets.
fn set_rel_field(rel: &mut C4Relation, field: &str, value: String) {
    let value = Some(value);
    match field {
        "textColor" => rel.text_color = value,
        "lineColor" => rel.line_color = value,
        "offsetX" => rel.offset_x = value,
        "offsetY" => rel.offset_y = value,
        _ => {}
    }
}

/// `obj[key] = TEXT_FIELDS.has(key) ? { text: value } : value` on a shape (`Construct`), and the
/// raw assignment `obj[key] = value` of a style statement (`Update`).
fn named_shape(entry: &mut Entry<C4Shape>, key: &str, value: &str, mode: Mode) -> DbResult {
    let shape = &mut entry.value;
    match key {
        "alias" | "parentBoundary" | "wrap" | "typeC4Shape" => {
            return Err(not_ported(&format!(
                "a named argument (`${key}`) that replaces a structural field"
            )));
        }
        "label" => shape.label = bare_or_text(mode, value).map(C4Text::Plain),
        "descr" => {
            shape.descr = bare_or_text(mode, value);
            entry.set_bare(TextSlot::Descr, mode == Mode::Update);
        }
        "techn" => {
            shape.techn = bare_or_text(mode, value);
            entry.set_bare(TextSlot::Techn, mode == Mode::Update);
        }
        // A shape has no type field: the reference stores the value under `type` in the object.
        "type" => {
            let stored = match mode {
                Mode::Construct => json!({ "text": value }),
                Mode::Update => Value::String(value.to_owned()),
            };
            shape.extra.insert("type".to_owned(), stored);
        }
        "sprite" | "tags" | "link" | "bgColor" | "fontColor" | "borderColor" | "shadowing"
        | "shape" | "legendText" | "legendSprite" => match key {
            "tags" => shape.tags = Some(value.to_owned()),
            "link" => shape.link = Some(value.to_owned()),
            other => shape.set_style(other, value),
        },
        other => {
            shape
                .extra
                .insert(other.to_owned(), Value::String(value.to_owned()));
        }
    }
    Ok(())
}

/// The named attribute of a boundary: the same rules, on the boundary's fields.
fn named_boundary(entry: &mut Entry<C4Boundary>, key: &str, value: &str, mode: Mode) -> DbResult {
    let boundary = &mut entry.value;
    match key {
        "alias" | "parentBoundary" | "wrap" => {
            return Err(not_ported(&format!(
                "a named argument (`${key}`) that replaces a structural field"
            )));
        }
        "label" => boundary.label = bare_or_text(mode, value).map(C4Text::Plain),
        "descr" => {
            boundary.descr = bare_or_text(mode, value);
            entry.set_bare(TextSlot::Descr, mode == Mode::Update);
        }
        "techn" => boundary.techn = bare_or_text(mode, value),
        "type" => boundary.boundary_type = bare_or_text(mode, value),
        "nodeType" => boundary.node_type = Some(value.to_owned()),
        "tags" | "link" => boundary.set_raw(key, value),
        "sprite" => boundary.sprite = Some(value.to_owned()),
        "bgColor" | "fontColor" | "borderColor" | "shadowing" | "shape" | "legendText"
        | "legendSprite" => boundary.set_style(key, value),
        other => {
            boundary
                .extra
                .insert(other.to_owned(), Value::String(value.to_owned()));
        }
    }
    Ok(())
}

/// The named attribute of a relation. `type`, `from` and `to` are plain strings on a relation, and
/// a declaration cannot store `{ text }` over `type`, so it is not ported there.
fn named_rel(entry: &mut Entry<C4Relation>, key: &str, value: &str, mode: Mode) -> DbResult {
    let rel = &mut entry.value;
    match key {
        "wrap" => {
            return Err(not_ported("a named argument (`$wrap`) on a relation"));
        }
        "type" if mode == Mode::Construct => {
            return Err(not_ported(
                "a named argument (`$type`) in a relationship declaration",
            ));
        }
        "type" => rel.relation_type = value.to_owned(),
        "from" => rel.from = value.to_owned(),
        "to" => rel.to = value.to_owned(),
        "label" => rel.label = bare_or_text(mode, value).map(C4Text::Plain),
        "descr" => {
            rel.descr = bare_or_text(mode, value);
            entry.set_bare(TextSlot::Descr, mode == Mode::Update);
        }
        "techn" => {
            rel.techn = bare_or_text(mode, value);
            entry.set_bare(TextSlot::Techn, mode == Mode::Update);
        }
        "sprite" => rel.sprite = Some(value.to_owned()),
        "tags" => rel.tags = Some(value.to_owned()),
        "link" => rel.link = Some(value.to_owned()),
        "textColor" | "lineColor" | "offsetX" | "offsetY" => {
            set_rel_field(rel, key, value.to_owned());
        }
        other => {
            rel.extra
                .insert(other.to_owned(), Value::String(value.to_owned()));
        }
    }
    Ok(())
}

/// `parseInt` of a layout argument, with NaN for an absent argument.
fn parse_int_arg(arg: Option<&Arg>) -> f64 {
    match arg {
        Some(Arg::Text(text)) => js_parse_int(text),
        Some(Arg::Named(_, value)) => js_parse_int(value),
        None => f64::NAN,
    }
}

/// `parseInt(text)` without a radix: the white space and a sign, then hexadecimal for `0x`, or
/// decimal digits, up to the first character that is not one. NaN when there are no digits.
fn js_parse_int(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(is_js_space);
    let (negative, unsigned) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let (radix, digits) = match unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        Some(hex) => (16, hex),
        None => (10, unsigned),
    };
    let valid = digits
        .find(|c: char| !c.is_digit(radix))
        .map_or(digits, |end| &digits[..end]);
    if valid.is_empty() {
        return f64::NAN;
    }
    let magnitude = if radix == 10 {
        valid.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        valid.chars().fold(0.0, |value, c| {
            value * 16.0 + f64::from(c.to_digit(16).unwrap_or(0))
        })
    };
    if negative { -magnitude } else { magnitude }
}

/// `String(number)`: the text JavaScript gives a number. Zero of either sign is `0`, and a number of
/// magnitude 10^21 or more is written with an exponent and a sign.
fn js_number_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value.abs() >= 1e21 {
        let scientific = format!("{value:e}");
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let exponent: i32 = exponent.parse().unwrap_or(0);
        let sign = if exponent < 0 { "-" } else { "+" };
        return format!("{mantissa}e{sign}{}", exponent.unsigned_abs());
    }
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_reads_the_leading_digits() {
        assert_eq!(js_parse_int("5px"), 5.0);
        assert_eq!(js_parse_int("  -12"), -12.0);
        assert_eq!(js_parse_int("0x1F"), 31.0);
        assert!(js_parse_int("abc").is_nan());
        assert!(js_parse_int("").is_nan());
    }

    #[test]
    fn js_number_string_matches_string_of_a_number() {
        assert_eq!(js_number_string(-0.0), "0");
        assert_eq!(js_number_string(5.0), "5");
        assert_eq!(js_number_string(f64::NAN), "NaN");
        assert_eq!(js_number_string(1e21), "1e+21");
        assert_eq!(js_number_string(1e20), "100000000000000000000");
    }
}
