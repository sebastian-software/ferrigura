/**
 * The Unicode coverage of a TrueType or OpenType font: the code points its `cmap` maps to a glyph.
 *
 * The coverage is read from every Unicode subtable (platform 0, and platform 3 with encodings 0, 1
 * and 10), so it agrees with the font's `cmap` as a whole. A code point mapped to glyph 0 is not
 * covered, as `Face::covers` in `ferrigura-text` reads it. Formats 0, 4, 6 and 12 are read; a font
 * with another format is an error, not an empty coverage.
 */

/** The sfnt table directory of a font: each table's offset and length, by its four-letter tag. */
function tableRecords(data: DataView): Map<string, { offset: number; length: number }> {
  const count = data.getUint16(4);
  const tables = new Map<string, { offset: number; length: number }>();
  for (let index = 0; index < count; index++) {
    const record = 12 + 16 * index;
    const tag = String.fromCharCode(
      data.getUint8(record),
      data.getUint8(record + 1),
      data.getUint8(record + 2),
      data.getUint8(record + 3)
    );
    tables.set(tag, { offset: data.getUint32(record + 8), length: data.getUint32(record + 12) });
  }
  return tables;
}

/** The code points of one `cmap` subtable that map to a glyph other than 0. */
function subtableCoverage(data: DataView, offset: number, into: Set<number>): void {
  const format = data.getUint16(offset);
  if (format === 0) {
    for (let code = 0; code < 256; code++) {
      if (data.getUint8(offset + 6 + code) !== 0) {
        into.add(code);
      }
    }
  } else if (format === 4) {
    const segmentCount = data.getUint16(offset + 6) / 2;
    const endCodes = offset + 14;
    const startCodes = endCodes + 2 + segmentCount * 2;
    const idDeltas = startCodes + segmentCount * 2;
    const idRangeOffsets = idDeltas + segmentCount * 2;
    for (let segment = 0; segment < segmentCount; segment++) {
      const start = data.getUint16(startCodes + segment * 2);
      const end = data.getUint16(endCodes + segment * 2);
      const delta = data.getUint16(idDeltas + segment * 2);
      const rangeOffset = data.getUint16(idRangeOffsets + segment * 2);
      for (let code = start; code <= end && code !== 0xffff; code++) {
        let glyph: number;
        if (rangeOffset === 0) {
          glyph = (code + delta) & 0xffff;
        } else {
          const address = idRangeOffsets + segment * 2 + rangeOffset + 2 * (code - start);
          glyph = data.getUint16(address);
          if (glyph !== 0) {
            glyph = (glyph + delta) & 0xffff;
          }
        }
        if (glyph !== 0) {
          into.add(code);
        }
      }
    }
  } else if (format === 6) {
    const first = data.getUint16(offset + 6);
    const entries = data.getUint16(offset + 8);
    for (let index = 0; index < entries; index++) {
      if (data.getUint16(offset + 10 + 2 * index) !== 0) {
        into.add(first + index);
      }
    }
  } else if (format === 12) {
    const groups = data.getUint32(offset + 12);
    for (let group = 0; group < groups; group++) {
      const record = offset + 16 + 12 * group;
      const start = data.getUint32(record);
      const end = data.getUint32(record + 4);
      for (let code = start; code <= end; code++) {
        into.add(code);
      }
    }
  } else {
    throw new Error(`cmap format ${format} is not read here`);
  }
}

/** The Unicode code points a font covers, from its `cmap` subtables. */
export function readCmap(bytes: Uint8Array): Set<number> {
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const cmap = tableRecords(data).get('cmap');
  if (!cmap) {
    throw new Error('the font has no cmap table');
  }
  const covered = new Set<number>();
  const subtableCount = data.getUint16(cmap.offset + 2);
  for (let index = 0; index < subtableCount; index++) {
    const record = cmap.offset + 4 + 8 * index;
    const platform = data.getUint16(record);
    const encoding = data.getUint16(record + 2);
    const unicode = platform === 0 || (platform === 3 && [0, 1, 10].includes(encoding));
    if (unicode) {
      subtableCoverage(data, cmap.offset + data.getUint32(record + 4), covered);
    }
  }
  return covered;
}

/** The tags of the tables of a font, in the order of its table directory. */
export function readTableTags(bytes: Uint8Array): string[] {
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return [...tableRecords(data).keys()];
}
