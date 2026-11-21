//! Retains legacy kern-v0 pair tables, which the subsetter does not serialize itself.

use std::collections::BTreeMap;

use read_fonts::tables::kern::{Kern, Subtable, SubtableKind};
use read_fonts::types::Tag;
use read_fonts::{FontRef, TableProvider};
use skera::Plan;
use write_fonts::FontBuilder;

use crate::TextError;

/// Reject unsupported kern formats rather than producing differently measured embedded text.
pub(crate) fn validate(font: &FontRef<'_>) -> Result<(), TextError> {
    if font.table_data(Tag::new(b"kern")).is_none() {
        return Ok(());
    }
    let kern = font.kern().map_err(|_| TextError::InvalidFont)?;
    if !matches!(kern, Kern::Ot(_)) {
        return Err(unsupported());
    }
    for subtable in kern.subtables() {
        let subtable = subtable.map_err(|_| TextError::InvalidFont)?;
        if !matches!(subtable.kind(), Ok(SubtableKind::Format0(_))) {
            return Err(unsupported());
        }
    }
    Ok(())
}

fn unsupported() -> TextError {
    TextError::Registration("only OpenType format-0 legacy kern tables are supported".into())
}

pub(crate) fn restore(
    font: &FontRef<'_>,
    subset: Vec<u8>,
    plan: &Plan,
) -> Result<Vec<u8>, TextError> {
    if font.table_data(Tag::new(b"kern")).is_none() {
        return Ok(subset);
    }
    validate(font)?;
    let mapping: BTreeMap<_, _> = plan.old_to_new_glyph_mapping().collect();
    let kern = font.kern().map_err(|_| TextError::InvalidFont)?;
    let mut tables = Vec::new();
    for subtable in kern.subtables() {
        let subtable = subtable.map_err(|_| TextError::InvalidFont)?;
        let Subtable::Ot(ref table) = subtable else {
            return Err(unsupported());
        };
        let SubtableKind::Format0(format) = subtable.kind().map_err(|_| TextError::InvalidFont)?
        else {
            return Err(unsupported());
        };
        let mut pairs = Vec::new();
        for pair in format.pairs() {
            if let (Some(left), Some(right)) = (
                mapping.get(&pair.left().into()),
                mapping.get(&pair.right().into()),
            ) {
                let left = u16::try_from(left.to_u32()).map_err(|_| unsupported())?;
                let right = u16::try_from(right.to_u32()).map_err(|_| unsupported())?;
                pairs.push((left, right, pair.value()));
            }
        }
        if pairs.is_empty() {
            continue;
        }
        pairs.sort_unstable();
        let count = u16::try_from(pairs.len()).map_err(|_| unsupported())?;
        let length = u16::try_from(14 + pairs.len() * 6).map_err(|_| unsupported())?;
        let power = 1_u16 << count.ilog2();
        let search_range = power * 6;
        let mut data = Vec::new();
        for value in [
            0,
            length,
            table.coverage(),
            count,
            search_range,
            count.ilog2() as u16,
            count * 6 - search_range,
        ] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        for (left, right, value) in pairs {
            data.extend_from_slice(&left.to_be_bytes());
            data.extend_from_slice(&right.to_be_bytes());
            data.extend_from_slice(&value.to_be_bytes());
        }
        tables.push(data);
    }
    if tables.is_empty() {
        return Ok(subset);
    }
    let count = u16::try_from(tables.len()).map_err(|_| unsupported())?;
    let mut data = vec![0, 0];
    data.extend_from_slice(&count.to_be_bytes());
    for table in tables {
        data.extend(table);
    }
    let subset_font = FontRef::new(&subset).map_err(|_| TextError::InvalidFont)?;
    let mut builder = FontBuilder::new();
    builder
        .add_raw(Tag::new(b"kern"), data)
        .copy_missing_tables(subset_font);
    Ok(builder.build())
}
