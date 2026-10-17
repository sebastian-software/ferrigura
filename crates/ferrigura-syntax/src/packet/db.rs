//! The packet DB's rows: `populate` and `getNextFittingBlock` of
//! `packages/mermaid/src/diagrams/packet/parser.ts` (lines 11-53 and 55-91), and `pushWord` of
//! `db.ts` (36-40). The arithmetic is the reference's, in the same order, on doubles, so that the
//! values a degenerate `bitsPerRow` gives are the ones the reference gives.

use ferrigura_model::packet::PacketBlock;
use ferrigura_model::theme::khroma::js_number;
use ferrigura_model::value::Value;

use super::grammar::RawBlock;
use crate::error::{ParseError, Source};
use crate::text::js_number as string_to_number;

/// The most rows a packet keeps (`maxPacketSize`, `parser.ts:9`). Blocks past it are dropped
/// without an error.
const MAX_PACKET_SIZE: f64 = 10_000.0;

/// `bitsPerRow` as the reference uses it: once as the factor of `row * bitsPerRow`, and once in
/// `bitsPerRow + 1`, which is a string when the configuration holds a string.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BitsPerRow {
    /// `bitsPerRow` as a number, in `row * bitsPerRow`.
    pub(crate) times: f64,
    /// `bitsPerRow + 1` as a number, in the test of the word's length.
    pub(crate) plus_one: f64,
}

impl BitsPerRow {
    /// The reading of a configuration value, as JavaScript reads it in `row * bitsPerRow` and in
    /// `bitsPerRow + 1`. A number is used as it is, and `null` and a boolean convert to numbers. A
    /// string, an array or an object is text: `bitsPerRow + 1` then concatenates the `1`.
    pub(crate) fn of(value: &Value) -> Self {
        match value {
            Value::Number(number) => Self {
                times: *number,
                plus_one: number + 1.0,
            },
            Value::Null => Self {
                times: 0.0,
                plus_one: 1.0,
            },
            Value::Bool(flag) => {
                let number = f64::from(u8::from(*flag));
                Self {
                    times: number,
                    plus_one: number + 1.0,
                }
            }
            Value::String(text) => Self::of_text(text),
            Value::Array(_) | Value::Object(_) => Self::of_text(&js_string(value)),
        }
    }

    /// The reading of a string: `"16" * row` and `"16" + 1`, which is `"161"`.
    fn of_text(text: &str) -> Self {
        Self {
            times: string_to_number(text),
            plus_one: string_to_number(&format!("{text}1")),
        }
    }
}

/// `String(value)` of JavaScript, for the values a configuration can hold in an array or object.
/// A `null` element of an array is empty, as `Array.prototype.join` writes it.
fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => js_number(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => js_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// The rows of a packet, from its blocks in source order: the DB calls of `populate`, with the
/// errors the reference throws, at the block that caused them.
pub(crate) fn populate(
    blocks: &[RawBlock],
    bits_per_row: BitsPerRow,
    source: Source<'_>,
) -> Result<Vec<Vec<PacketBlock>>, ParseError> {
    let bits = bits_per_row.times;
    let mut rows: Vec<Vec<PacketBlock>> = Vec::new();
    let mut word: Vec<PacketBlock> = Vec::new();
    let mut last_bit = -1.0_f64;
    let mut row = 1.0_f64;

    for raw in blocks {
        let fail = |message: String| source.error(raw.offset, message);
        if let (Some(start), Some(end)) = (raw.start, raw.end)
            && end < start
        {
            return Err(fail(format!(
                "Packet block {} - {} is invalid. End must be greater than start.",
                js_number(start),
                js_number(end)
            )));
        }
        let start = raw.start.unwrap_or(last_bit + 1.0);
        if start != last_bit + 1.0 {
            let end = raw.end.unwrap_or(start);
            return Err(fail(format!(
                "Packet block {} - {} is not contiguous. It should start from {}.",
                js_number(start),
                js_number(end),
                js_number(last_bit + 1.0)
            )));
        }
        if raw.bits == Some(0.0) {
            return Err(fail(format!(
                "Packet block {} is invalid. Cannot have a zero bit field.",
                js_number(start)
            )));
        }
        let end = raw.end.unwrap_or(start + raw.bits.unwrap_or(1.0) - 1.0);
        let width = raw.bits.unwrap_or(end - start + 1.0);
        last_bit = end;

        let mut current = PacketBlock {
            start,
            end,
            bits: width,
            label: raw.label.clone(),
        };
        while (word.len() as f64) <= bits_per_row.plus_one && (rows.len() as f64) < MAX_PACKET_SIZE
        {
            let (block, next) = fitting_block(&current, row, bits).map_err(fail)?;
            let completes_row = block.end + 1.0 == row * bits;
            word.push(block);
            if completes_row {
                rows.push(std::mem::take(&mut word));
                row += 1.0;
            }
            match next {
                None => break,
                Some(next) => current = next,
            }
        }
    }
    if !word.is_empty() {
        rows.push(word);
    }
    Ok(rows)
}

/// `getNextFittingBlock`: the block as it lies in `row`, and the rest of it when it runs past the
/// row. The first piece's `bits` is one less than its width, and so is the rest's (`parser.ts:82`
/// and `88`); the reference keeps that, see `docs-port/packet/PLAN.md`, quirk 1.
fn fitting_block(
    block: &PacketBlock,
    row: f64,
    bits_per_row: f64,
) -> Result<(PacketBlock, Option<PacketBlock>), String> {
    if block.start > block.end {
        return Err(format!(
            "Block start {} is greater than block end {}.",
            js_number(block.start),
            js_number(block.end)
        ));
    }
    if block.end + 1.0 <= row * bits_per_row {
        return Ok((block.clone(), None));
    }
    let row_end = row * bits_per_row - 1.0;
    let row_start = row * bits_per_row;
    Ok((
        PacketBlock {
            start: block.start,
            end: row_end,
            bits: row_end - block.start,
            label: block.label.clone(),
        },
        Some(PacketBlock {
            start: row_start,
            end: block.end,
            bits: block.end - row_start,
            label: block.label.clone(),
        }),
    ))
}
