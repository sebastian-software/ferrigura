//! `BasicProgressMonitor` as elkjs runs it, with the logging tree of `collectLogs`.
//!
//! Java semantics kept: `begin` names a monitor only once, `subTask` creates a
//! child monitor that is kept in order, `done` closes a monitor, and `log`
//! records messages only when logging is on. A monitor that is only created
//! by `subTask` and never begun has no name and shows as `null` in the tree.
//!
//! Source: `org.eclipse.elk.core.util.BasicProgressMonitor` (elkjs 0.9.3
//! `lib/elk-worker.js`, `$begin`, `$doSubTask`, `$done_0`, `collectLogs`).

use serde_json::{Map, Value};

/// A handle to one monitor in a [`ProgressMonitor`] tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monitor(usize);

#[derive(Debug)]
struct Node {
    task_name: Option<String>,
    closed: bool,
    logs: Vec<String>,
    children: Vec<usize>,
}

/// The tree of monitors for one layout run.
#[derive(Debug)]
pub struct ProgressMonitor {
    nodes: Vec<Node>,
    recording_logs: bool,
}

/// The logging tree, shaped like elkjs's `logging` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceNode {
    pub name: Option<String>,
    pub logs: Vec<String>,
    pub children: Vec<TraceNode>,
}

impl ProgressMonitor {
    /// A new tree with a root monitor. `logging` mirrors `withLogging`.
    pub fn new(logging: bool) -> (Self, Monitor) {
        let mut monitor = ProgressMonitor {
            nodes: Vec::new(),
            recording_logs: logging,
        };
        let root = monitor.push_node();
        (monitor, root)
    }

    fn push_node(&mut self) -> Monitor {
        self.nodes.push(Node {
            task_name: None,
            closed: false,
            logs: Vec::new(),
            children: Vec::new(),
        });
        Monitor(self.nodes.len() - 1)
    }

    /// `isLoggingEnabled`.
    pub fn is_logging_enabled(&self) -> bool {
        self.recording_logs
    }

    /// `begin(name, work)`: names the monitor if it has no name yet.
    ///
    /// Returns `false` when the monitor was already named; that is not an error
    /// in Java and is not one here.
    pub fn begin(&mut self, monitor: Monitor, name: &str, _total_work: f64) -> bool {
        let node = &mut self.nodes[monitor.0];
        if node.closed {
            panic!("The task is already done.");
        }
        if node.task_name.is_some() {
            return false;
        }
        node.task_name = Some(name.to_string());
        true
    }

    /// `subTask(work)`: a new child monitor, or `None` once `monitor` is done.
    pub fn sub_task(&mut self, monitor: Monitor, _work: f64) -> Option<Monitor> {
        if self.nodes[monitor.0].closed {
            return None;
        }
        let child = self.push_node();
        self.nodes[monitor.0].children.push(child.0);
        Some(child)
    }

    /// `done()`: closes the monitor. Panics if it was never begun, as Java does.
    pub fn done(&mut self, monitor: Monitor) {
        let node = &mut self.nodes[monitor.0];
        if node.task_name.is_none() {
            panic!("The task has not begun yet.");
        }
        node.closed = true;
    }

    /// `log(message)`: kept only when logging is on.
    pub fn log(&mut self, monitor: Monitor, message: impl Into<String>) {
        if self.recording_logs {
            self.nodes[monitor.0].logs.push(message.into());
        }
    }

    /// The tree below `root`, as `collectLogs` writes it.
    pub fn trace(&self, root: Monitor) -> TraceNode {
        self.trace_of(root.0)
    }

    fn trace_of(&self, index: usize) -> TraceNode {
        let node = &self.nodes[index];
        TraceNode {
            name: node.task_name.clone(),
            logs: if self.recording_logs {
                node.logs.clone()
            } else {
                Vec::new()
            },
            children: node
                .children
                .iter()
                .map(|&child| self.trace_of(child))
                .collect(),
        }
    }
}

impl TraceNode {
    /// The JSON shape of elkjs's `logging` object: `name`, then `logs` and
    /// `children` only when they are non-empty.
    pub fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "name".to_string(),
            self.name.clone().map_or(Value::Null, Value::String),
        );
        if !self.logs.is_empty() {
            object.insert(
                "logs".to_string(),
                Value::Array(self.logs.iter().cloned().map(Value::String).collect()),
            );
        }
        if !self.children.is_empty() {
            object.insert(
                "children".to_string(),
                Value::Array(self.children.iter().map(TraceNode::to_json).collect()),
            );
        }
        Value::Object(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtasks_nest_under_their_parent_in_order() {
        let (mut pm, root) = ProgressMonitor::new(true);
        assert!(pm.begin(root, "Recursive Graph Layout", 1.0));
        let first = pm.sub_task(root, 1.0).unwrap();
        pm.begin(first, "Layered layout", 1.0);
        let second = pm.sub_task(root, 1.0).unwrap();
        pm.begin(second, "Other", 1.0);
        pm.log(first, "hello");
        let trace = pm.trace(root);
        assert_eq!(trace.name.as_deref(), Some("Recursive Graph Layout"));
        assert_eq!(trace.children[0].name.as_deref(), Some("Layered layout"));
        assert_eq!(trace.children[0].logs, vec!["hello".to_string()]);
        assert_eq!(trace.children[1].name.as_deref(), Some("Other"));
    }

    #[test]
    fn logs_are_dropped_when_logging_is_off() {
        let (mut pm, root) = ProgressMonitor::new(false);
        pm.begin(root, "x", 1.0);
        pm.log(root, "ignored");
        assert!(pm.trace(root).logs.is_empty());
        assert!(!pm.is_logging_enabled());
    }

    #[test]
    fn json_omits_empty_fields() {
        let (mut pm, root) = ProgressMonitor::new(true);
        pm.begin(root, "a", 1.0);
        let child = pm.sub_task(root, 1.0).unwrap();
        pm.begin(child, "c", 1.0);
        pm.done(child);
        assert_eq!(
            pm.trace(root).to_json(),
            serde_json::json!({ "name": "a", "children": [{ "name": "c" }] })
        );
        // A subtask that is never begun has no name, which shows as null.
        let unnamed = pm.sub_task(root, 1.0).unwrap();
        assert_eq!(
            pm.trace(unnamed).to_json(),
            serde_json::json!({ "name": null })
        );
    }
}
