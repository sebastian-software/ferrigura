//! The source checks of `event-modeling-validator.ts`: which entity types may feed which.
//!
//! The reference registers these checks with Langium's validation registry, but its `parse()`
//! reads only the lexer and parser errors, so the checks never run on the render path
//! (`docs-port/eventmodeling/NOTES.md`, section 2.5). The port keeps the checks as functions of
//! their own, tested on their own, and [`parse`](super::parse) does not call them.

use super::ast::EventModel;

/// A source that cannot feed its frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The index of the frame in the text order.
    pub frame: usize,
    /// The message of the reference's check.
    pub message: String,
}

/// Checks every frame that has sources. A source number that names no frame is skipped, as
/// Langium's linker leaves an unresolved reference without a target.
#[must_use]
pub fn check_frames(ast: &EventModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (index, frame) in ast.frames.iter().enumerate() {
        if frame.source_frames.is_empty() {
            continue;
        }
        let sources: Vec<&str> = frame
            .source_frames
            .iter()
            .filter_map(|name| {
                ast.frames
                    .iter()
                    .find(|candidate| candidate.name == *name)
                    .map(|candidate| candidate.model_entity_type.as_str())
            })
            .collect();
        for message in check_source_frame_types(&frame.model_entity_type, &sources) {
            diagnostics.push(Diagnostic {
                frame: index,
                message,
            });
        }
    }
    diagnostics
}

/// The messages for the sources of a frame of type `target`, one per source that is not allowed,
/// in the order of the sources. A target type with no rule has no messages.
#[must_use]
pub fn check_source_frame_types(target: &str, sources: &[&str]) -> Vec<String> {
    let (target_label, expected_label, allowed): (&str, &str, &[&str]) = match target {
        "cmd" | "command" => ("command", "ui or processor", &["ui", "pcr", "processor"]),
        "evt" | "event" => ("event", "command", &["cmd", "command"]),
        "rmo" | "readmodel" => ("read model", "event", &["evt", "event"]),
        "pcr" | "processor" => ("processor", "read model", &["rmo", "readmodel"]),
        "ui" => ("ui", "read model", &["rmo", "readmodel"]),
        _ => return Vec::new(),
    };
    sources
        .iter()
        .filter(|source| !allowed.contains(source))
        .map(|source| {
            format!(
                "A {target_label} can only receive input from a {expected_label}, not from '{source}'."
            )
        })
        .collect()
}
