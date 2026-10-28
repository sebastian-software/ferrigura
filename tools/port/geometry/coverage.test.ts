/**
 * The geometry harness's exclusion by coverage: a fixture is excluded only when its drawn text has a
 * character that no bundled face covers, and the reason names it. The faces are the bundled files
 * (`tools/port/fonts/bundled.ts`), so the rule follows what ferrigura can draw.
 */
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { faceCoverage, labelCoverage, unicodeRange } from '../fonts/bundled.ts';
import { exclusionReason, uncoveredCharacters, uncoveredScriptReason } from './common.ts';

/** A flowchart with the given node text. */
function flowchart(text: string): string {
  return `flowchart LR\n  A[${text}] --> B\n`;
}

describe('the coverage of the bundled faces', () => {
  it('covers what the text faces, the arrows, the text symbols and the emoji draw', () => {
    const covered = labelCoverage();
    for (const code of [0x41, 0xe9, 0x2192, 0x2665, 0x26a0, 0x2713, 0x1f600, 0x2615, 0x2795]) {
      assert.equal(covered.has(code), true, `U+${code.toString(16)}`);
    }
  });

  it('does not cover CJK, kana, box drawing, or emoji that Twemoji lacks', () => {
    const covered = labelCoverage();
    for (const code of [0x63d0, 0x307c, 0x3315, 0x2500, 0x1f6d8]) {
      assert.equal(covered.has(code), false, `U+${code.toString(16)}`);
    }
  });

  it('keeps the characters of Atkinson Mono out of a label, as no label falls back to it', () => {
    assert.equal(faceCoverage('AtkinsonHyperlegibleMono-Regular.ttf').has(0x266a), true);
    assert.equal(labelCoverage().has(0x266a), false);
  });

  it('writes a range list of exactly the code points it is given', () => {
    assert.equal(
      unicodeRange([0x2600, 0x2601, 0x2602, 0x2605, 0x2665]),
      'U+2600-2602, U+2605, U+2665'
    );
    assert.equal(unicodeRange([]), '');
  });
});

describe('the exclusion by coverage', () => {
  it('compares a diagram whose text has symbols and emoji the faces draw', () => {
    assert.equal(exclusionReason(flowchart('a ♥ ✓ → 😀 ☕')), null);
    assert.deepEqual(uncoveredCharacters(flowchart('a ♥ ✓ → 😀')), []);
  });

  it('excludes a fixture with a character no face covers, and names it', () => {
    const reason = exclusionReason(flowchart('提交'));
    assert.match(reason ?? '', /^text no bundled face covers \(U\+63D0 提, U\+4EA4 交\)/);
    assert.match(uncoveredScriptReason(flowchart('a 🛘')) ?? '', /U\+1F6D8/);
  });

  it('excludes emoji with emoji presentation that Twemoji lacks', () => {
    assert.deepEqual(uncoveredCharacters(flowchart('🛘')), ['U+1F6D8 🛘']);
  });

  it('does not count a character of the front matter that the diagram does not draw', () => {
    const source = `---\ntitle: a plain title\ndescription: 提交 and 😀 in the description\n---\nflowchart LR\n  A --> B\n`;
    assert.equal(exclusionReason(source), null);
  });

  it('counts the title of the front matter', () => {
    const source = `---\ntitle: 提交\n---\nflowchart LR\n  A --> B\n`;
    assert.match(exclusionReason(source) ?? '', /U\+63D0/);
  });

  it('does not count the line breaks, which the reference collapses', () => {
    assert.deepEqual(uncoveredCharacters('flowchart LR\r\n\tA --> B\n'), []);
  });

  it('treats the box-drawing characters of a tree view as its syntax, not drawn text', () => {
    const tree = 'treeView-beta\n  project/\n  ├── src/\n  └── README.md\n';
    assert.deepEqual(uncoveredCharacters(tree), []);
    assert.deepEqual(uncoveredCharacters('flowchart LR\n  A["├── x"] --> B\n'), [
      'U+251C ├',
      'U+2500 ─',
    ]);
  });

  it('keeps the other exclusions as they were', () => {
    assert.match(
      exclusionReason('---\nconfig:\n  look: handDrawn\n---\nflowchart LR\n  A --> B\n') ?? '',
      /^look handDrawn/
    );
  });
});
