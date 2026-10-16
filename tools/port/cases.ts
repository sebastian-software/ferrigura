/**
 * Explicit inputs of the differential run, added to the ones the reference already tests.
 *
 * Title precedence (`docs-port/canonical-model.md`): the grammar's title wins when it is
 * non-empty, and the front matter title fills in otherwise. The info DB has no title setter, so
 * the front matter title does not reach the info model. The accessibility cases pin the blank
 * handling of block and single-line values, which the DB whitespace rules must leave alone.
 */
export const explicitCases: Record<string, string[]> = {
  pie: [
    '---\ntitle: From front matter\n---\npie title From grammar\n"a": 1',
    '---\ntitle: Only front matter\n---\npie\n"a": 1',
    '---\ntitle: Empty grammar title\n---\npie title\n"a": 1',
    '---\ntitle: "Quoted: title"\n---\npie\n"a": 1',
    'pie title Grammar only\n"a": 1',
    'pie\n"a": 1',
    'pie accDescr {\n   one\t\t two\n\n\n   three  \n \t\n}\n"a": 1',
    'pie accTitle:      spaced   out   %% comment\n"a": 1',
  ],
  info: [
    '---\ntitle: From front matter\n---\ninfo title From grammar',
    '---\ntitle: Only front matter\n---\ninfo',
  ],
  flowchart: [
    // A subgraph without an id or a title: the reference reads the text of an undefined id.
    'graph TD\nsubgraph\nA\nend',
    // A node named like an edge id: the DB returns before it makes the vertex.
    'graph TD\nA e1@-->B\ne1 --> C',
    // A link that is not a link: a head that does not match the start, which the DB keeps as INVALID.
    'graph TD\nA x-- text --o B',
    // Keywords as ids, and the direction rule that takes the rest of the line.
    'graph TD\nA-->B direction TB\nC-->D',
    'graph TD;styleA-->B;',
    // Accessibility lines inside a subgraph are strings of its list.
    'graph TD\nsubgraph X\naccTitle: foo\nA\nend',
    // Entities and colour semicolons are encoded before the parser sees the text.
    'graph TD\nA[#35;x]-->B\nstyle A fill:#f9f;',
  ],
};
