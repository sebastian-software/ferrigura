//! The gitGraph DB, `packages/mermaid/src/diagrams/git/gitGraphAst.ts`: the state that the
//! statements change, with the reference's checks and messages. The method order and the quirks
//! are in `docs-port/gitgraph/PLAN.md` sections 5 and 7.
//!
//! The reference's maps keep insertion order, so the branches and commits are vectors with an
//! index. A repeated commit id replaces the commit at its first position, as `Map.set` does.
//! `head` is not stored: it is always the head of the current branch (each commit, merge and
//! cherry-pick moves both, and `checkout` reads it from the branch), so it is `branches[current]`.

use std::collections::HashMap;

use ferrigura_model::CANONICAL_VERSION;
use ferrigura_model::git_graph::{
    CommitType, Direction, GitGraphBranch, GitGraphCommit, GitGraphDiagram,
};

use super::Options;
use crate::sanitize::{sanitize_section, sanitize_text};
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// A DB error: the message the reference throws, which is kept exactly.
pub(crate) type DbResult = Result<(), String>;

/// A branch: its name, its `order` and the id of its head commit.
struct BranchState {
    name: String,
    order: f64,
    head: Option<String>,
}

/// The generator of the page the reference is rendered in: its `Math.random` is mulberry32 from a
/// fixed seed (`renderReference` in `tools/port/geometry/common.ts`). A seeded parse draws the
/// generated ids from it, in the order the DB makes them, so they are the reference's ids.
#[derive(Debug, Clone)]
pub(crate) struct Mulberry32 {
    state: i32,
}

impl Mulberry32 {
    /// The generator of a seed, as `clock.seed` of the harness.
    pub(crate) fn new(seed: u32) -> Self {
        Self { state: seed as i32 }
    }

    /// The next hex digit of `makeRandomHex`: `Math.floor(Math.random() * 16)`, which is the top four
    /// bits of the 32-bit value the generator returns.
    fn next_digit(&mut self) -> usize {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let state = self.state;
        let mut t = (state ^ ((state as u32) >> 15) as i32).wrapping_mul(1 | state);
        t = t.wrapping_add((t ^ ((t as u32) >> 7) as i32).wrapping_mul(61 | t)) ^ t;
        let value = (t ^ ((t as u32) >> 14) as i32) as u32;
        (value >> 28) as usize
    }

    /// The id part of `random({ length })`: `length` hex digits, each drawn from the generator.
    fn hex_digits(&mut self, length: usize) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        (0..length)
            .map(|_| char::from(DIGITS[self.next_digit()]))
            .collect()
    }
}

/// The state of a gitGraph while its statements are applied.
pub(crate) struct Db {
    title: String,
    acc_title: String,
    acc_descr: String,
    direction: Direction,
    branches: Vec<BranchState>,
    branch_index: HashMap<String, usize>,
    current: String,
    commits: Vec<GitGraphCommit>,
    commit_index: HashMap<String, usize>,
    seq: u64,
    warnings: Vec<String>,
    random: Option<Mulberry32>,
}

/// The id a commit gets when the input gives none. The reference draws seven hex digits from
/// `Math.random`; this port derives them from the sequence number, as documented in
/// `docs-port/gitgraph/PLAN.md` section 7: `<seq>-` and seven hex digits of an FNV-1a hash.
#[must_use]
pub(crate) fn auto_id(seq: u64) -> String {
    let hash = fnv1a(format!("gitgraph-auto-id:{seq}").as_bytes());
    format!("{seq}-{:07x}", hash & 0x0fff_ffff)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl Db {
    /// A cleared DB: the main branch alone, as `clear()` leaves it.
    pub(crate) fn new(options: &Options) -> Self {
        let main = options.main_branch_name.clone();
        let mut branch_index = HashMap::new();
        branch_index.insert(main.clone(), 0);
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: Direction::LR,
            branches: vec![BranchState {
                name: main.clone(),
                order: options.main_branch_order,
                head: None,
            }],
            branch_index,
            current: main,
            commits: Vec::new(),
            commit_index: HashMap::new(),
            seq: 0,
            warnings: Vec::new(),
            random: options.auto_id_seed.map(Mulberry32::new),
        }
    }

    /// The id of a commit that has none: `<seq>-` and seven hex digits. Seeded, the digits are the
    /// reference's `getID()`; otherwise they come from `auto_id`.
    fn generated_id(&mut self, seq: u64) -> String {
        match &mut self.random {
            Some(random) => format!("{seq}-{}", random.hex_digits(7)),
            None => auto_id(seq),
        }
    }

    /// `setDiagramTitle`: the text is sanitised with the top-level configuration.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, then the leading whitespace is removed.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: sanitised, then the whitespace after each line break is removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setDirection`.
    pub(crate) fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /// Warnings the reference would log, such as a repeated commit id.
    pub(crate) fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The model of the DB, after the statements.
    pub(crate) fn into_model(self) -> GitGraphDiagram {
        GitGraphDiagram {
            version: CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            direction: self.direction,
            current_branch: self.current,
            branches: self
                .branches
                .into_iter()
                .map(|branch| GitGraphBranch {
                    name: branch.name,
                    order: branch.order,
                    head: branch.head,
                })
                .collect(),
            commits: self.commits,
        }
    }

    /// `commit`: `id` is sanitised and an empty id is generated; `msg` and the tags are sanitised.
    pub(crate) fn commit(
        &mut self,
        id: Option<&str>,
        message: &str,
        kind: Option<CommitType>,
        tags: &[String],
    ) -> DbResult {
        let given = id.map(sanitize_section).filter(|id| !id.is_empty());
        let message = sanitize_section(message);
        let tags: Vec<String> = tags.iter().map(|tag| sanitize_section(tag)).collect();
        // `seq` is read before its increment, so the first id is `0-…`.
        let seq = self.next_seq();
        let auto_id_used = given.is_none();
        let id = match given {
            Some(id) => id,
            None => self.generated_id(seq),
        };
        let commit = GitGraphCommit {
            id: id.clone(),
            seq,
            branch: self.current.clone(),
            kind: kind.unwrap_or(CommitType::Normal),
            custom_type: None,
            custom_id: None,
            auto_id: auto_id_used,
            message,
            tags,
            parents: self.head_parents(),
        };
        self.set_current_head(id.clone());
        if self.commit_index.contains_key(&id) {
            self.warnings.push(format!("Commit ID {id} already exists"));
        }
        self.store_commit(commit);
        Ok(())
    }

    /// `branch`: a new branch at the current head, then a checkout of it.
    pub(crate) fn branch(&mut self, raw_name: &str, order: f64) -> DbResult {
        let name = sanitize_section(raw_name);
        if self.branch_index.contains_key(&name) {
            return Err(format!(
                "Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using \"checkout {name}\")"
            ));
        }
        let head = self.head_id();
        self.branch_index.insert(name.clone(), self.branches.len());
        self.branches.push(BranchState {
            name: name.clone(),
            order,
            head,
        });
        self.checkout(&name)
    }

    /// `merge`: the checks in the reference's order, then a merge commit with the two heads as
    /// parents. The merge's tags are not sanitised, as in the reference.
    pub(crate) fn merge(
        &mut self,
        raw_branch: &str,
        raw_id: Option<&str>,
        kind: Option<CommitType>,
        tags: &[String],
    ) -> DbResult {
        let other = sanitize_section(raw_branch);
        // `customId` is sanitised only when it is truthy, so an empty id stays empty.
        let custom = match raw_id {
            Some(id) if !id.is_empty() => sanitize_section(id),
            _ => String::new(),
        };
        let current_head = self.head_id();
        let current_commit = current_head.as_deref().and_then(|id| self.commit_by_id(id));
        let other_head = self.branch_head(&other).flatten();
        let other_commit = other_head.as_deref().and_then(|id| self.commit_by_id(id));
        if let (Some(current), Some(_)) = (current_commit, other_commit)
            && current.branch == other
        {
            return Err(format!("Cannot merge branch '{other}' into itself."));
        }
        if self.current == other {
            return Err("Incorrect usage of \"merge\". Cannot merge a branch to itself".to_owned());
        }
        let Some(current_id) = current_head.filter(|id| self.commit_index.contains_key(id)) else {
            return Err(format!(
                "Incorrect usage of \"merge\". Current branch ({})has no commits",
                self.current
            ));
        };
        if !self.branch_index.contains_key(&other) {
            return Err(format!(
                "Incorrect usage of \"merge\". Branch to be merged ({other}) does not exist"
            ));
        }
        let Some(other_id) = other_head.filter(|id| self.commit_index.contains_key(id)) else {
            return Err(format!(
                "Incorrect usage of \"merge\". Branch to be merged ({other}) has no commits"
            ));
        };
        if current_id == other_id {
            return Err("Incorrect usage of \"merge\". Both branches have same head".to_owned());
        }
        if !custom.is_empty() && self.commit_index.contains_key(&custom) {
            return Err(format!(
                "Incorrect usage of \"merge\". Commit with id:{custom} already exists, use different custom id"
            ));
        }

        let seq = self.next_seq();
        let auto_id_used = custom.is_empty();
        let id = if auto_id_used {
            self.generated_id(seq)
        } else {
            custom.clone()
        };
        let commit = GitGraphCommit {
            id: id.clone(),
            seq,
            branch: self.current.clone(),
            kind: CommitType::Merge,
            custom_type: kind,
            custom_id: Some(!auto_id_used),
            auto_id: auto_id_used,
            message: format!("merged branch {other} into {}", self.current),
            tags: tags.to_vec(),
            parents: vec![current_id, other_id],
        };
        self.set_current_head(id);
        self.store_commit(commit);
        Ok(())
    }

    /// `checkout`: the branch must exist. The name is sanitised first.
    pub(crate) fn checkout(&mut self, raw_name: &str) -> DbResult {
        let name = sanitize_section(raw_name);
        if !self.branch_index.contains_key(&name) {
            return Err(format!(
                "Trying to checkout branch which is not yet created. (Help try using \"branch {name}\")"
            ));
        }
        self.current = name;
        Ok(())
    }

    /// `cherryPick` with the current branch as the target, which is the only target the grammar
    /// reaches. `tags` is `None` when the statement gave none, and then the default tag is used.
    pub(crate) fn cherry_pick(
        &mut self,
        raw_source: Option<&str>,
        tags: Option<&[String]>,
        raw_parent: Option<&str>,
    ) -> DbResult {
        let source = raw_source.map(sanitize_section).unwrap_or_default();
        let tags = tags.map(|tags| {
            tags.iter()
                .map(|tag| sanitize_section(tag))
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<String>>()
        });
        let parent = raw_parent.map(sanitize_section).unwrap_or_default();

        let source_commit = if source.is_empty() {
            None
        } else {
            self.commit_by_id(&source)
        };
        let Some(source_commit) = source_commit.cloned() else {
            return Err(
                "Incorrect usage of \"cherryPick\". Source commit id should exist and provided"
                    .to_owned(),
            );
        };
        if !parent.is_empty() && !source_commit.parents.contains(&parent) {
            return Err(
                "Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit."
                    .to_owned(),
            );
        }
        if source_commit.kind == CommitType::Merge && parent.is_empty() {
            return Err(
                "Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified."
                    .to_owned(),
            );
        }
        if source_commit.branch == self.current {
            return Err(
                "Incorrect usage of \"cherryPick\". Source commit is already on current branch"
                    .to_owned(),
            );
        }
        let current_id = self
            .head_id()
            .filter(|id| self.commit_index.contains_key(id));
        let Some(current_id) = current_id else {
            return Err(format!(
                "Incorrect usage of \"cherry-pick\". Current branch ({})has no commits",
                self.current
            ));
        };

        let seq = self.next_seq();
        let message = format!(
            "cherry-picked {} into {}",
            source_commit.message, self.current
        );
        let tags = tags.unwrap_or_else(|| {
            let merge_suffix = if source_commit.kind == CommitType::Merge {
                format!("|parent:{parent}")
            } else {
                String::new()
            };
            vec![format!("cherry-pick:{}{merge_suffix}", source_commit.id)]
        });
        let parents = vec![current_id, source_commit.id.clone()];
        let id = self.generated_id(seq);
        let commit = GitGraphCommit {
            id: id.clone(),
            seq,
            branch: self.current.clone(),
            kind: CommitType::CherryPick,
            custom_type: None,
            custom_id: None,
            auto_id: true,
            message,
            tags,
            parents,
        };
        self.set_current_head(id);
        self.store_commit(commit);
        Ok(())
    }

    /// The sequence number of the next commit, which the counter then moves past.
    fn next_seq(&mut self) -> u64 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    /// The head id of the current branch, `None` while it has no commit.
    fn head_id(&self) -> Option<String> {
        self.branch_head(&self.current).flatten()
    }

    /// The head of a branch: `None` if there is no such branch, else its head (possibly none).
    fn branch_head(&self, name: &str) -> Option<Option<String>> {
        self.branch_index
            .get(name)
            .and_then(|&index| self.branches.get(index))
            .map(|branch| branch.head.clone())
    }

    /// The parents of a commit made now: the current head, if there is one.
    fn head_parents(&self) -> Vec<String> {
        self.head_id().into_iter().collect()
    }

    /// Moves the head of the current branch to `id`.
    fn set_current_head(&mut self, id: String) {
        let index = self.branch_index.get(&self.current).copied();
        if let Some(branch) = index.and_then(|index| self.branches.get_mut(index)) {
            branch.head = Some(id);
        }
    }

    fn commit_by_id(&self, id: &str) -> Option<&GitGraphCommit> {
        self.commit_index
            .get(id)
            .and_then(|&index| self.commits.get(index))
    }

    /// `commits.set(id, commit)`: a repeated id keeps its first position and gets the new value.
    fn store_commit(&mut self, commit: GitGraphCommit) {
        match self.commit_index.get(&commit.id).copied() {
            Some(index) => {
                if let Some(slot) = self.commits.get_mut(index) {
                    *slot = commit;
                }
            }
            None => {
                self.commit_index
                    .insert(commit.id.clone(), self.commits.len());
                self.commits.push(commit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_ids_have_the_reference_shape() {
        for seq in [0, 1, 9, 10, 123] {
            let id = auto_id(seq);
            let (number, hex) = id.split_once('-').expect("a dash");
            assert_eq!(number, seq.to_string());
            assert_eq!(hex.len(), 7);
            assert!(
                hex.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            );
        }
        assert_ne!(auto_id(0), auto_id(1));
    }
}

#[cfg(test)]
mod seeded_id_tests {
    use crate::git_graph::{Options, parse_with};

    /// A seeded parse draws the generated ids from the reference page's generator: the merge without
    /// an `id:` in fixture `e2e/diagrams/git/nightly-check/20-…` is `4-a08ff49` in the reference
    /// rendered with `clock.seed` 1 (`tools/port/geometry/common.ts`), and so here.
    #[test]
    fn a_seeded_parse_gives_the_reference_ids() {
        let text = "gitGraph TB:\n  commit id: \"1\"\n  commit id: \"2\"\n  branch develop\n  checkout develop\n  commit id: \"3\"\n  commit id: \"4\"\n  checkout main\n  merge develop\n  commit id: \"5\"\n  commit id: \"6\"\n";
        let seeded = parse_with(
            text,
            &Options {
                auto_id_seed: Some(1),
                ..Options::default()
            },
        )
        .unwrap();
        let merge = seeded.commits.iter().find(|commit| commit.auto_id).unwrap();
        assert_eq!(merge.id, "4-a08ff49");
        let unseeded = parse_with(text, &Options::default()).unwrap();
        assert!(
            unseeded
                .commits
                .iter()
                .any(|commit| commit.id.starts_with("4-") && commit.auto_id)
        );
        assert_ne!(
            unseeded
                .commits
                .iter()
                .find(|commit| commit.auto_id)
                .unwrap()
                .id,
            "4-a08ff49"
        );
    }
}
