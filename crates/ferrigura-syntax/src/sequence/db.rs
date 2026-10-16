//! The sequence DB of `sequenceDb.ts`: the state the parser's items build, and `apply`, which
//! runs the items in order and keeps the state as the reference does.
//!
//! The state is the canonical model of `ferrigura-model`. Each method follows the reference method
//! of the same name, and the quirks are kept where the code keeps them (`docs-port/sequence/PLAN.md`,
//! sections 5 and 8). Errors carry the reference's text; the caller adds the position of the item.

use std::collections::{BTreeMap, HashMap};

use ferrigura_model::sequence::{
    Autonumber, CentralConnection, LineType, MessageText, Placement, SequenceActor, SequenceBox,
    SequenceDiagram, SequenceMessage,
};
use ferrigura_model::value::Value;

use super::Options;
use super::grammar::{ActorAction, BoxData, Item, MsgText};
use crate::sanitize::sanitize_text;
use crate::text::{js_trim, newline_then_space_to_newline, strip_leading_space};
use crate::yaml::{self, Yaml};

/// An error of the DB, with the reference's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub(crate) String);

/// The description of an actor, as `addActor` receives it.
#[derive(Debug, Clone, PartialEq)]
struct Description {
    text: Option<String>,
    wrap: Option<bool>,
}

/// The sequence DB state.
pub(crate) struct Db {
    options: Options,
    actors: Vec<SequenceActor>,
    actor_index: HashMap<String, usize>,
    prev_actor: Option<String>,
    created: BTreeMap<String, usize>,
    destroyed: BTreeMap<String, usize>,
    boxes: Vec<SequenceBox>,
    current_box: Option<usize>,
    messages: Vec<SequenceMessage>,
    last_created: Option<String>,
    last_destroyed: Option<String>,
}

impl Db {
    pub(crate) fn new(options: Options) -> Self {
        Self {
            options,
            actors: Vec::new(),
            actor_index: HashMap::new(),
            prev_actor: None,
            created: BTreeMap::new(),
            destroyed: BTreeMap::new(),
            boxes: Vec::new(),
            current_box: None,
            messages: Vec::new(),
            last_created: None,
            last_destroyed: None,
        }
    }

    /// `autoWrap()`: the resolved `wrap` setting, which the caller gives in [`Options`].
    fn auto_wrap(&self) -> bool {
        self.options.wrap
    }

    /// `apply` for one item (`sequenceDb.ts:549-710`).
    pub(crate) fn apply(&mut self, item: &Item) -> Result<(), DbError> {
        match item {
            Item::Participant {
                create,
                actor,
                draw,
                description,
                config,
            } => {
                if *create {
                    if self.actor_index.contains_key(actor) {
                        return Err(DbError(
                            "It is not possible to have actors with the same id, even if one is destroyed before the next is created. Use 'AS' aliases to simulate the behavior".to_owned(),
                        ));
                    }
                    self.last_created = Some(actor.clone());
                    self.add_actor(
                        actor,
                        actor,
                        description.as_ref().map(to_description),
                        *draw,
                        config.as_deref(),
                    )?;
                    self.created.insert(actor.clone(), self.messages.len());
                } else {
                    self.add_actor(
                        actor,
                        actor,
                        description.as_ref().map(to_description),
                        *draw,
                        config.as_deref(),
                    )?;
                }
            }
            Item::Destroy(actor) => {
                self.last_destroyed = Some(actor.clone());
                self.destroyed.insert(actor.clone(), self.messages.len());
            }
            Item::Message {
                from,
                to,
                line_type,
                text,
                activate,
                central,
            } => {
                // The created and destroyed checks of `apply` (`:615-638`). The reference reads
                // `.name` of the id string, so the name is `undefined` (quirk Q6).
                if let Some(created) = self.last_created.clone() {
                    if to != &created {
                        return Err(DbError(
                            "The created participant undefined does not have an associated creating message after its declaration. Please check the sequence diagram.".to_owned(),
                        ));
                    }
                    self.last_created = None;
                } else if let Some(destroyed) = self.last_destroyed.clone() {
                    if to != &destroyed && from != &destroyed {
                        return Err(DbError(
                            "The destroyed participant undefined does not have an associated destroying message after its declaration. Please check the sequence diagram.".to_owned(),
                        ));
                    }
                    self.last_destroyed = None;
                }
                self.add_signal(
                    Some(from.as_str()),
                    Some(to.as_str()),
                    Some(text),
                    *line_type,
                    *activate,
                    *central,
                )?;
            }
            Item::Signal {
                from,
                text,
                line_type,
            } => {
                self.add_signal(
                    from.as_deref(),
                    None,
                    text.as_ref(),
                    *line_type,
                    false,
                    CentralConnection::NoConnection,
                )?;
            }
            Item::Index {
                start,
                step,
                visible,
            } => {
                let id = self.messages.len().to_string();
                self.messages.push(SequenceMessage {
                    id,
                    from: None,
                    to: None,
                    message: MessageText::Autonumber(Autonumber {
                        start: *start,
                        step: *step,
                        visible: *visible,
                    }),
                    wrap: false,
                    line_type: LineType::Autonumber,
                    activate: None,
                    central_connection: None,
                    placement: None,
                });
            }
            Item::Note {
                from,
                to,
                placement,
                text,
            } => self.add_note(from, to, *placement, text),
            Item::Actions {
                action,
                actor,
                text,
            } => self.actor_action(*action, actor, text),
            Item::BoxStart(data) => self.add_box(data),
            Item::BoxEnd => self.current_box = None,
        }
        Ok(())
    }

    /// `addBox` (`:145-153`).
    fn add_box(&mut self, data: &BoxData) {
        self.boxes.push(SequenceBox {
            name: data.name.clone(),
            wrap: data.wrap.unwrap_or_else(|| self.auto_wrap()),
            fill: data.color.clone(),
            actor_keys: Vec::new(),
        });
        self.current_box = Some(self.boxes.len() - 1);
    }

    /// `addActor` (`:155-231`).
    fn add_actor(
        &mut self,
        id: &str,
        name: &str,
        description: Option<Description>,
        draw: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<(), DbError> {
        let doc = match metadata {
            Some(metadata) => {
                // A single line is wrapped in braces; several lines are a block mapping (`:164-173`).
                let text = if metadata.contains('\n') {
                    format!("{metadata}\n")
                } else {
                    format!("{{\n{metadata}\n}}")
                };
                yaml::load(&text).map_err(|error| DbError(error.message))?
            }
            None => None,
        };
        // `type = doc?.type ?? type`: a null `type` falls back to the one the statement gives.
        let mut actor_type = yaml_field(&doc, "type")
            .and_then(js_string)
            .or_else(|| draw.map(str::to_owned));
        let mut description = description;

        // An alias replaces the name when no description is given (or the description is the name).
        if let Some(alias) = yaml_field(&doc, "alias")
            .filter(|value| is_truthy(value))
            .and_then(js_string)
        {
            let replace = description
                .as_ref()
                .is_none_or(|given| given.text.as_deref() == Some(name));
            if replace {
                let wrap = description.as_ref().and_then(|given| given.wrap);
                description = Some(Description {
                    text: Some(alias),
                    wrap,
                });
            }
        }

        let mut assigned_box = self.current_box;
        let old_index = self.actor_index.get(id).copied();
        if let Some(old_index) = old_index {
            let old_box = self.actors[old_index].box_index;
            if let (Some(current), Some(old)) = (self.current_box, old_box)
                && current != old
            {
                let old_name = self.actors[old_index].name.clone();
                return Err(DbError(format!(
                    "A same participant should only be defined in one Box: {old_name} can't be in '{}' and in '{}' at the same time.",
                    box_name(&self.boxes, old),
                    box_name(&self.boxes, current),
                )));
            }
            // A box that the participant has is kept; the first box given is taken otherwise.
            assigned_box = old_box.or(self.current_box);
            self.actors[old_index].box_index = assigned_box;
            // A repeated implicit reference keeps the record (`:195-197`, quirk Q4).
            if name == self.actors[old_index].name && description.is_none() {
                return Ok(());
            }
        }

        // `description?.text == null` and then `type == null`: the name is the text.
        if description
            .as_ref()
            .is_none_or(|given| given.text.is_none())
        {
            description = Some(Description {
                text: Some(name.to_owned()),
                wrap: None,
            });
        }
        if actor_type.is_none() {
            description = Some(Description {
                text: Some(name.to_owned()),
                wrap: None,
            });
        }
        let given = description.unwrap_or(Description {
            text: Some(name.to_owned()),
            wrap: None,
        });
        let record = SequenceActor {
            id: id.to_owned(),
            name: name.to_owned(),
            description: given.text.unwrap_or_default(),
            actor_type: actor_type
                .take()
                .unwrap_or_else(|| "participant".to_owned()),
            wrap: given.wrap.unwrap_or_else(|| self.auto_wrap()),
            box_index: assigned_box,
            prev_actor: self.prev_actor.clone(),
            next_actor: None,
            links: BTreeMap::new(),
            properties: BTreeMap::new(),
        };
        match old_index {
            Some(index) => self.actors[index] = record,
            None => {
                self.actor_index.insert(id.to_owned(), self.actors.len());
                self.actors.push(record);
            }
        }
        // The previous actor points here; for a repeated declaration that is the actor itself, since
        // the record has just been replaced (quirk Q4).
        if let Some(prev) = self.prev_actor.clone()
            && let Some(&prev_index) = self.actor_index.get(&prev)
        {
            self.actors[prev_index].next_actor = Some(id.to_owned());
        }
        if let Some(current) = self.current_box {
            self.boxes[current].actor_keys.push(id.to_owned());
        }
        self.prev_actor = Some(id.to_owned());
        Ok(())
    }

    /// `activationCount` (`:233-254`): the activations of `part` minus its deactivations.
    fn activation_count(&self, part: &str) -> i64 {
        if part.is_empty() {
            return 0;
        }
        self.messages
            .iter()
            .filter(|message| message.from.as_deref() == Some(part))
            .map(|message| match message.line_type {
                LineType::ActiveStart => 1,
                LineType::ActiveEnd => -1,
                _ => 0,
            })
            .sum()
    }

    /// `addSignal` (`:272-308`).
    fn add_signal(
        &mut self,
        from: Option<&str>,
        to: Option<&str>,
        message: Option<&MsgText>,
        line_type: LineType,
        activate: bool,
        central: CentralConnection,
    ) -> Result<(), DbError> {
        if line_type == LineType::ActiveEnd {
            let from_name = from.unwrap_or("");
            if self.activation_count(from_name) < 1 {
                return Err(DbError(format!(
                    "Trying to inactivate an inactive participant ({})",
                    from.unwrap_or("undefined")
                )));
            }
        }
        let text = message.map_or_else(String::new, |message| message.text.clone());
        let wrap = message
            .and_then(|message| message.wrap)
            .unwrap_or_else(|| self.auto_wrap());
        let id = self.messages.len().to_string();
        self.messages.push(SequenceMessage {
            id,
            from: from.map(str::to_owned),
            to: to.map(str::to_owned),
            message: MessageText::Text(text),
            wrap,
            line_type,
            activate: Some(activate),
            central_connection: Some(central),
            placement: None,
        });
        Ok(())
    }

    /// `addNote` (`:425-450`). The notes list is not in the model; the message is.
    fn add_note(&mut self, from: &str, to: &str, placement: Placement, text: &MsgText) {
        let wrap = text.wrap.unwrap_or_else(|| self.auto_wrap());
        let id = self.messages.len().to_string();
        self.messages.push(SequenceMessage {
            id,
            from: Some(from.to_owned()),
            to: Some(to.to_owned()),
            message: MessageText::Text(text.text.clone()),
            wrap,
            line_type: LineType::Note,
            activate: None,
            central_connection: None,
            placement: Some(placement),
        });
    }

    /// `addLinks`, `addALink`, `addProperties` and `addDetails` (`:452-546`).
    fn actor_action(&mut self, action: ActorAction, actor: &str, text: &MsgText) {
        let Some(&index) = self.actor_index.get(actor) else {
            return;
        };
        match action {
            ActorAction::Links => {
                let sanitised = sanitise_link_text(&text.text);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&sanitised) {
                    insert_all(&mut self.actors[index].links, Value::from(json));
                }
            }
            ActorAction::ALink => {
                // `sep` is taken before the entities are replaced, and the slices use it as it is.
                let sanitised = sanitize_text(&text.text);
                let units: Vec<u16> = sanitised.encode_utf16().collect();
                let separator = units.iter().position(|&unit| unit == u16::from(b'@'));
                let replaced = replace_entities(&sanitised);
                let replaced_units: Vec<u16> = replaced.encode_utf16().collect();
                let label_end = separator.map_or(-2, |sep| sep as i64 - 1);
                let label = js_slice(&replaced_units, 0, label_end);
                let link = js_slice(
                    &replaced_units,
                    separator.map_or(0, |sep| sep as i64 + 1),
                    i64::MAX,
                );
                // `insertLinks` of `{label: link}`: one key, overwritten if it is there.
                self.actors[index].links.insert(
                    js_trim(&label).to_owned(),
                    Value::String(js_trim(&link).to_owned()),
                );
            }
            ActorAction::Properties => {
                let sanitised = sanitize_text(&text.text);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&sanitised) {
                    insert_all(&mut self.actors[index].properties, Value::from(json));
                }
            }
            // `document.getElementById` reads an element of the page, which the port does not have.
            // The reference's lookup finds nothing in the text of a diagram, so nothing is stored.
            ActorAction::Details => {}
        }
    }

    /// The state as the model: the header texts and the lists, in the DB's order.
    pub(crate) fn into_model(
        self,
        title: Option<&str>,
        acc_title: Option<&str>,
        acc_descr: Option<&str>,
    ) -> SequenceDiagram {
        let mut diagram = SequenceDiagram::new();
        diagram.title = title.map(sanitize_text).unwrap_or_default();
        diagram.acc_title = acc_title
            .map(|text| strip_leading_space(&sanitize_text(text)).to_owned())
            .unwrap_or_default();
        diagram.acc_descr = acc_descr
            .map(|text| newline_then_space_to_newline(&sanitize_text(text)))
            .unwrap_or_default();
        diagram.actors = self.actors;
        diagram.boxes = self.boxes;
        diagram.messages = self.messages;
        diagram.created_actors = self.created;
        diagram.destroyed_actors = self.destroyed;
        diagram
    }
}

fn to_description(text: &MsgText) -> Description {
    Description {
        text: Some(text.text.clone()),
        wrap: text.wrap,
    }
}

fn box_name(boxes: &[SequenceBox], index: usize) -> &str {
    boxes
        .get(index)
        .and_then(|found| found.name.as_deref())
        .unwrap_or("undefined")
}

/// The sanitised text of a links or properties statement (`sanitizeText`, then the entities that
/// the reference replaces after sanitising, which never match its placeholders, quirk Q10).
fn sanitise_link_text(text: &str) -> String {
    replace_entities(&sanitize_text(text))
}

/// `replace(/&equals;/g, '=')` and then `replace(/&amp;/g, '&')`.
fn replace_entities(text: &str) -> String {
    text.replace("&equals;", "=").replace("&amp;", "&")
}

/// `insertLinks` and `insertProperties`: `actor.x[key] = value` for each key of a JSON value. A
/// non-object value has the keys `for…in` gives it (an array's indices, a string's characters).
fn insert_all(target: &mut BTreeMap<String, Value>, value: Value) {
    match value {
        Value::Object(map) => target.extend(map),
        Value::Array(items) => {
            for (index, item) in items.into_iter().enumerate() {
                target.insert(index.to_string(), item);
            }
        }
        Value::String(text) => {
            for (index, character) in text.chars().enumerate() {
                target.insert(index.to_string(), Value::String(character.to_string()));
            }
        }
        _ => {}
    }
}

/// `String.prototype.slice` on UTF-16 units, with negative indices counted from the end.
fn js_slice(units: &[u16], start: i64, end: i64) -> String {
    let len = units.len() as i64;
    let clamp = |index: i64| -> usize {
        let index = if index < 0 {
            (len + index).max(0)
        } else {
            index.min(len)
        };
        index as usize
    };
    let (from, to) = (clamp(start), clamp(end));
    if from >= to {
        return String::new();
    }
    String::from_utf16_lossy(&units[from..to])
}

/// A field of the YAML mapping, `None` when the document is not a mapping or has no such key.
fn yaml_field<'y>(doc: &'y Option<Yaml>, key: &str) -> Option<&'y Yaml> {
    match doc {
        Some(Yaml::Mapping(pairs)) => pairs
            .iter()
            .find(|(name, _)| name.as_str() == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

/// The string a scalar YAML value reads as in JavaScript, `None` for null and for collections.
fn js_string(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(text) => Some(text.clone()),
        Yaml::Number(number) => Some(number.to_string()),
        Yaml::Bool(flag) => Some(flag.to_string()),
        Yaml::Null | Yaml::Sequence(_) | Yaml::Mapping(_) => None,
    }
}

/// JavaScript truthiness of a YAML scalar.
fn is_truthy(value: &Yaml) -> bool {
    match value {
        Yaml::Null => false,
        Yaml::Bool(flag) => *flag,
        Yaml::Number(number) => *number != 0.0 && !number.is_nan(),
        Yaml::String(text) => !text.is_empty(),
        Yaml::Sequence(_) | Yaml::Mapping(_) => true,
    }
}
