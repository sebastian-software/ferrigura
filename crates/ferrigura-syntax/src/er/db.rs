//! The ER DB of `erDb.ts`, with the header setters of `commonDb.ts`. The DB's state is the model
//! (`ferrigura_model::er::ErDiagram`); the method names follow the DB's, in snake case.
//!
//! Subgraph ids and entity names share one namespace, as in the reference: a relationship end is
//! the subgraph id when a subgraph of that id exists at the time, and the entity id otherwise.

use std::collections::HashSet;

use ferrigura_model::er::{
    ErAttribute, ErClass, ErDiagram, ErEntity, ErRelationship, ErSubgraph, RelSpec,
};

use crate::sanitize::sanitize_text;
use crate::text::{js_trim, newline_then_space_to_newline, strip_leading_space};

/// One member of a subgraph, as the grammar's values give it: a name, or the `dir` object of a
/// `direction` statement inside the subgraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    /// A string: an entity name, a nested subgraph id, or a value the grammar keeps (see the plan).
    Name(String),
    /// The direction of the subgraph, from a `direction` statement inside it.
    Direction(String),
}

impl From<&str> for Member {
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

/// The DB of one diagram. Create one per parse; [`ErDb::into_model`] gives the state.
#[derive(Debug, Clone, Default)]
pub struct ErDb {
    model: ErDiagram,
    /// The number of subgraphs open at the current statement (`subgraphDepth`).
    subgraph_depth: usize,
}

impl ErDb {
    /// An empty DB with the direction `TB`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            model: ErDiagram::new(),
            subgraph_depth: 0,
        }
    }

    /// `clear()`: removes the entities, relationships, classes and subgraphs, and the header
    /// texts. The direction stays, as the reference's `clear` leaves it.
    pub fn clear(&mut self) {
        let direction = std::mem::take(&mut self.model.direction);
        self.model = ErDiagram::new();
        self.model.direction = direction;
        self.subgraph_depth = 0;
    }

    /// `setDiagramTitle`: the title from the front matter, sanitised.
    pub fn set_diagram_title(&mut self, text: &str) {
        self.model.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, with the leading whitespace removed.
    pub fn set_acc_title(&mut self, text: &str) {
        self.model.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: sanitised, with the whitespace after each line break removed.
    pub fn set_acc_description(&mut self, text: &str) {
        self.model.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setDirection`: the last top-level direction wins.
    pub fn set_direction(&mut self, direction: &str) {
        self.model.direction = direction.to_owned();
    }

    /// `getDirection`.
    #[must_use]
    pub fn get_direction(&self) -> &str {
        &self.model.direction
    }

    /// `addEntity(name, alias)`: creates the entity if it is new, and gives it the alias if it has
    /// none yet. Returns the index of the entity.
    pub fn add_entity(&mut self, name: &str, alias: &str) -> usize {
        if let Some(index) = self.entity_index(name) {
            let entity = &mut self.model.entities[index];
            if entity.alias.is_empty() && !alias.is_empty() {
                entity.alias = alias.to_owned();
            }
            return index;
        }
        let index = self.model.entities.len();
        self.model.entities.push(ErEntity {
            id: format!("entity-{name}-{index}"),
            name: name.to_owned(),
            alias: alias.to_owned(),
            attributes: Vec::new(),
            classes: Vec::new(),
            styles: Vec::new(),
        });
        index
    }

    /// `getEntity(name)`.
    #[must_use]
    pub fn get_entity(&self, name: &str) -> Option<&ErEntity> {
        self.entity_index(name)
            .and_then(|i| self.model.entities.get(i))
    }

    /// `addAttributes(name, list)`: the entity is added first if it is new, and the attributes are
    /// appended in source order.
    pub fn add_attributes(&mut self, name: &str, attributes: Vec<ErAttribute>) {
        let index = self.add_entity(name, "");
        self.model.entities[index].attributes.extend(attributes);
    }

    /// `addRelationship(a, role, b, spec)`. Each end is the subgraph id when that subgraph exists,
    /// and the entity id otherwise (the entity is added in that case).
    pub fn add_relationship(&mut self, a: &str, role: &str, b: &str, spec: RelSpec) {
        let entity_a = self.relationship_end(a);
        let entity_b = self.relationship_end(b);
        self.model.relationships.push(ErRelationship {
            entity_a,
            role_a: role.to_owned(),
            entity_b,
            rel_spec: spec,
        });
    }

    /// `getRelationships`.
    #[must_use]
    pub fn get_relationships(&self) -> &[ErRelationship] {
        &self.model.relationships
    }

    /// `addCssStyles(ids, styles)`: each style goes to the entity and to the subgraph of each id.
    pub fn add_css_styles(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            if let Some(index) = self.entity_index(id) {
                self.model.entities[index]
                    .styles
                    .extend(styles.iter().cloned());
            }
            if let Some(index) = self.subgraph_index(id) {
                self.model.subgraphs[index]
                    .styles
                    .extend(styles.iter().cloned());
            }
        }
    }

    /// `addClass(ids, styles)`: each id gets a class (created if new) with the styles. A style
    /// that names a colour is also a text style, with its first `fill` read as `bgFill`.
    pub fn add_class(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            let index = match self.model.classes.iter().position(|class| class.id == *id) {
                Some(index) => index,
                None => {
                    self.model.classes.push(ErClass {
                        id: id.clone(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    });
                    self.model.classes.len() - 1
                }
            };
            let class = &mut self.model.classes[index];
            for style in styles {
                if style.contains("color") {
                    class.text_styles.push(style.replacen("fill", "bgFill", 1));
                }
                class.styles.push(style.clone());
            }
        }
    }

    /// `getClasses`.
    #[must_use]
    pub fn get_classes(&self) -> &[ErClass] {
        &self.model.classes
    }

    /// `setClass(ids, classNames)`: an entity gets the names in its class list, and a subgraph
    /// gets them in its own.
    pub fn set_class(&mut self, ids: &[String], class_names: &[String]) {
        for id in ids {
            if let Some(index) = self.entity_index(id) {
                self.model.entities[index]
                    .classes
                    .extend(class_names.iter().cloned());
            }
            if let Some(index) = self.subgraph_index(id) {
                self.model.subgraphs[index]
                    .classes
                    .extend(class_names.iter().cloned());
            }
        }
    }

    /// `addSubGraph(id, list, title)`: the members are the list without the empty strings, the
    /// repeated strings and the direction objects, which set the direction (the last one wins). A
    /// member that an earlier subgraph holds is dropped. Returns the trimmed id.
    pub fn add_subgraph(&mut self, id: &str, list: &[Member], title: &str) -> String {
        let id = js_trim(id).to_owned();
        let mut direction = None;
        let mut seen = HashSet::new();
        let mut nodes = Vec::new();
        for member in list {
            match member {
                Member::Direction(value) => direction = Some(value.clone()),
                Member::Name(name) => {
                    let trimmed = js_trim(name);
                    if !trimmed.is_empty() && seen.insert(trimmed.to_owned()) {
                        nodes.push(name.clone());
                    }
                }
            }
        }
        let earlier: HashSet<&String> = self
            .model
            .subgraphs
            .iter()
            .flat_map(|subgraph| subgraph.nodes.iter())
            .collect();
        nodes.retain(|node| !earlier.contains(node));
        let title = js_trim(&sanitize_text(title)).to_owned();
        self.model.subgraphs.push(ErSubgraph {
            id: id.clone(),
            title,
            nodes,
            classes: Vec::new(),
            styles: Vec::new(),
            dir: direction,
        });
        id
    }

    /// `getSubGraphs`.
    #[must_use]
    pub fn get_subgraphs(&self) -> &[ErSubgraph] {
        &self.model.subgraphs
    }

    /// The `subgraphDepth` of the DB: the subgraphs whose header has been read and whose `end` has
    /// not.
    #[must_use]
    pub fn subgraph_depth(&self) -> usize {
        self.subgraph_depth
    }

    /// Opens a subgraph: `subgraphDepth += 1`, when its header has been read.
    pub fn open_subgraph(&mut self) {
        self.subgraph_depth += 1;
    }

    /// Closes a subgraph at its `end`: `subgraphDepth -= 1`, and never below zero.
    pub fn close_subgraph(&mut self) {
        self.subgraph_depth = self.subgraph_depth.saturating_sub(1);
    }

    /// The model: the state of the DB after parsing.
    #[must_use]
    pub fn into_model(self) -> ErDiagram {
        self.model
    }

    fn entity_index(&self, name: &str) -> Option<usize> {
        self.model
            .entities
            .iter()
            .position(|entity| entity.name == name)
    }

    /// The last subgraph of the id: a repeated id replaces the lookup entry (`subGraphLookup.set`).
    fn subgraph_index(&self, id: &str) -> Option<usize> {
        self.model
            .subgraphs
            .iter()
            .rposition(|subgraph| subgraph.id == id)
    }

    fn relationship_end(&mut self, name: &str) -> String {
        if self.subgraph_index(name).is_some() {
            return name.to_owned();
        }
        let index = self.add_entity(name, "");
        self.model.entities[index].id.clone()
    }
}
