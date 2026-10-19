//! The venn DB, `vennDB.ts`, as the grammar's actions call it. The state is the model: the DB's
//! arrays are the model's, and the sets known for a union and the current sets of an indented
//! `text` are the parser's own state.

use std::collections::HashSet;

use ferrigura_model::venn::{StyleEntry, Subset, TextNode, VennDiagram};

use crate::sanitize::sanitize_text;
use crate::text::js_trim;

/// The state the venn grammar builds.
#[derive(Debug, Default)]
pub(super) struct VennDb {
    diagram: VennDiagram,
    /// `knownSets`: the ids of the `set` statements so far, which a union must name.
    known_sets: HashSet<String>,
    /// `currentSets`: the sets of the last `set` or `union`, which an indented `text` joins.
    current_sets: Option<Vec<String>>,
}

impl VennDb {
    /// `setDiagramTitle`: sanitised, and the last one wins, even when it sanitises to nothing.
    pub(super) fn set_diagram_title(&mut self, text: &str) {
        self.diagram.title = sanitize_text(text);
    }

    /// `addSubsetData`: the ids are normalised and sorted, the size is the statement's or `10 / n²`
    /// for `n` identifiers as written, and a single id becomes a known set.
    pub(super) fn add_subset(
        &mut self,
        identifiers: &[String],
        label: Option<String>,
        size: Option<f64>,
    ) {
        let sets = sorted_ids(identifiers);
        let count = identifiers.len() as f64;
        let size = size.unwrap_or(10.0 / (count * count));
        if sets.len() == 1 {
            self.known_sets.insert(sets[0].clone());
        }
        self.current_sets = Some(sets.clone());
        self.diagram.subsets.push(Subset {
            sets,
            size,
            label: label.map_or_else(String::new, |label| normalize_text(&label)),
        });
    }

    /// `validateUnionIdentifiers`: every id of a union must be a known set. The error names all the
    /// unknown ids.
    pub(super) fn validate_union(&self, identifiers: &[String]) -> Result<(), String> {
        let unknown: Vec<String> = identifiers
            .iter()
            .map(|identifier| normalize_text(identifier))
            .filter(|identifier| !self.known_sets.contains(identifier))
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(format!("unknown set identifier: {}", unknown.join(", ")))
        }
    }

    /// `addTextData`: the ids are normalised and sorted, and the id and the label are normalised.
    pub(super) fn add_text(&mut self, identifiers: &[String], id: &str, label: Option<String>) {
        self.diagram.text_nodes.push(TextNode {
            sets: sorted_ids(identifiers),
            id: normalize_text(id),
            label: label.map_or_else(String::new, |label| normalize_text(&label)),
        });
    }

    /// `addStyleData`: the targets are normalised and sorted. The properties go into an object in
    /// the reference, so a key given twice keeps its first place and takes the last value, and the
    /// key `__proto__` is dropped, since assigning to it on an ordinary object does nothing.
    pub(super) fn add_style(&mut self, identifiers: &[String], properties: Vec<(String, String)>) {
        let mut styles: Vec<(String, String)> = Vec::new();
        for (key, value) in properties {
            if key == "__proto__" {
                continue;
            }
            let value = normalize_text(&value);
            match styles.iter_mut().find(|(name, _)| *name == key) {
                Some(entry) => entry.1 = value,
                None => styles.push((key, value)),
            }
        }
        self.diagram.styles.push(StyleEntry {
            targets: sorted_ids(identifiers),
            styles,
        });
    }

    /// `getCurrentSets`: the sets of the last `set` or `union`, if there is one.
    pub(super) fn current_sets(&self) -> Option<Vec<String>> {
        self.current_sets.clone()
    }

    pub(super) fn into_model(self) -> VennDiagram {
        self.diagram
    }
}

/// `normalizeText`: trimmed, and without one pair of double quotes around it.
pub(super) fn normalize_text(text: &str) -> String {
    let trimmed = js_trim(text);
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `normalizeIdentifierList(list).sort()`: the normalised ids in JavaScript's default order, which
/// compares UTF-16 code units.
fn sorted_ids(identifiers: &[String]) -> Vec<String> {
    let mut sets: Vec<String> = identifiers
        .iter()
        .map(|identifier| normalize_text(identifier))
        .collect();
    sets.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    sets
}
