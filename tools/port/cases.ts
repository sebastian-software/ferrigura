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
    // The layout data reads the interpolation of the default and of one edge, and the `curve` of the shape data.
    'flowchart TB\nA-->B\nC-->A\nlinkStyle default interpolate stepBefore\nlinkStyle 0 interpolate basis\ne1@{ curve: monotoneX }',
    'flowchart TB\nA e1@-->B\ne1@{ animation: fast, animate: true, curve: stepAfter }',
    // `inheritDir` gives a subgraph without a direction the diagram's direction; an explicit one wins.
    '---\nconfig:\n  flowchart:\n    inheritDir: true\n---\nflowchart LR\nsubgraph one [One]\n  A\nend\nsubgraph two [Two]\n  direction TB\n  B\nend\nA --> B',
    // A padding of 0 is the reference's `|| 8`, and `curve` and `minNodeWidth` are read from the configuration.
    '---\nconfig:\n  flowchart:\n    padding: 0\n    minNodeWidth: 40\n    curve: linear\n---\nflowchart TD\nA-->B',
    // Colour styles of a class, a stroke on one edge and the default stroke.
    'flowchart TD\nclassDef hot color:#fff,fill:#f00\nA-->B\nclass A,B hot\nlinkStyle 0 stroke:#f00,stroke-width:2px\nlinkStyle default stroke:#333',
    // A self-loop next to a collapsed subgraph: the loop on a node that is not collapsed stays, the edge into the subgraph is redirected.
    'flowchart TB\nA-->A\nA-->B\nsubgraph s [S]\nA\nB\nend\ns@{ view: collapsed }\nB-->C',
    // Invisible links and an open arrow have no markers; the edge classes depend on the stroke.
    'flowchart TB\nA ~~~ B\nA --- C\nA --o D\nA -- text --> E',
  ],
  // The quirks of `docs-port/journey/PLAN.md`, section 8, one input each where they differ.
  // cspell:ignore njourneyfoo ntitlefoo Titlex ACCTITLE ACCDESCR
  journey: [
    // J1: keywords match the start of a word, case-insensitively, and need a boundary.
    'journey\njourneyfoo: 5: me',
    'journey\ntitlefoo: 5: me\nsection: 5: me\naccTitlex: 5: me',
    // The detector is case-sensitive; the keywords of the body are not.
    'journey\nTITLE Upper\nSECTION S\nTask: 1: a\nACCTITLE: t\nACCDESCR: d',
    'journey\njourney_x: 5: me',
    'journey-x\nT: 1: a',
    // J2: a comment rule consumes the character before `%%` at the start of a token.
    'journey\nxx%%foo\nT: 1: a',
    'journey\n%%\nT: 1: a',
    "journey\n%% comment\nT: 1: a %% not a comment\n%%{init: {'theme': 'dark'}}%%\nU: 2: b",
    // J3: `accTitle:` and `accDescr:` take the rest of the line, and cross lines after the colon.
    'journey\naccTitle:\n  section S\nT: 1: a',
    'journey\naccTitle:      spaced   out   %% comment\nT: 1: a',
    'journey\naccTitle:\n\n\n',
    'journey\naccDescr: one; two # three\nT: 1: a',
    // J4: an unclosed block, an empty block, and a block after a single-line description.
    'journey\naccDescr {\n  unclosed text',
    'journey\naccDescr {\n  unclosed text\nT: 1: a',
    'journey\naccDescr {}',
    'journey\naccDescr {',
    'journey\naccDescr: first\naccDescr {\nsecond\n}',
    'journey\naccDescr {\n  a\n\n\n    b\n}\nT: 1: a',
    // J5: the title is the text after its first six characters; the last title wins, even when empty.
    'journey\ntitle  x',
    'journey\ntitle  ',
    'journey\ntitle A\ntitle',
    'journey\ntitle A\ntitle <script>x</script>',
    '---\ntitle: From front matter\n---\njourney\ntitle Grammar\nT: 1: a',
    '---\ntitle: Only front matter\n---\njourney\nT: 1: a',
    '---\ntitle: Front matter\n---\njourney\ntitle <script>alert(1)</script>',
    '---\ntitle: "Quoted: title"\n---\njourney\ntitle x',
    // J6: a section keeps its blanks, and its name has no sanitising.
    'journey\nsection S \nT: 1: a\nsection  two  \nU: 2: b',
    'journey\nsection\nT: 1: a',
    'journey\nsection Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5\nT: 1: a',
    'journey\nsection A\nsection A\nsection B\nT: 1: a',
    // J7: a task name keeps its blanks, and its data must follow it at once.
    'journey\nT : 1: a',
    'journey\nT\n: 1: a',
    'journey\nT # c: 1: a',
    'journey\nT: 1 # c: a',
    // J8: scores are read as `Number()` reads them.
    'journey\nT: : me',
    'journey\nT:  : me',
    'journey\nT: 0x10: me\nU: 1e1: me\nV: +3: me\nW: -2: a\nX: .5: a\nY: 5.: a',
    'journey\nT: 0b101: a\nU: 0o17: a\nV: 0X1F: a\nW: 0B11: a',
    'journey\nT: abc: me\nU: 2rem: me\nV: 1_000: me\nW: NaN: me\nX: inf: me',
    'journey\nT: Infinity: me\nU: -Infinity: me\nV: +Infinity: me\nW: infinity: me',
    'journey\nT: 0x: me\nU: 0xg: me\nV: -0x10: me\nW: 1e: me\nX: 1e+: me',
    'journey\nT: 5:\nU: 5\nV: 0x10000000000000001: me\nW: 0x10000000000000801: me',
    // J9 and J10: only the first two pieces of the data are read, and the people are split and trimmed.
    'journey\nT: 5:a:b\nU: 5:a:b,c\nV: 5: x, , y,  z ,\nW: 5: me, me',
    'journey\nT: 5:: me\nU: 5: ,\nV: 5:,',
    // J11: actors sort by UTF-16 code units, and the order of the legend follows it.
    'journey\nT: 1: 2, 10, B, a, A, b, 1, Zed, \u{1d49c}, \uff21, \u{1f600}',
    // J12: entities are decoded for the reader; unknown names and raw ampersands stay as written.
    'journey\ntitle Hearts #9829; x\nT #hearts;: 1: #amp;',
    'journey\nT: 1: #foo;\nU #nope; x: 2: a',
    'journey\nT #0; x #99999999; y #1114112; z: 1: a',
    'journey\nT #NotEqualTilde; #lt;b#gt; #quot;q#apos;: 1: a',
    'journey\nT: 1: &amp; a&b &lt; #ldquo;x#rdquo;',
    'journey\nT: 1: #nbsp;x#nbsp;y#rarr;',
    'journey\nT: 1: a¶ßb ﬂ°°65¶ß ﬂ°amp',
    // J13: a `style` or `classDef` line loses its last semicolon before the entities are encoded.
    'journey\nstyle T:#a; x\nclassDef y:#fff; z\nT: 1: a',
    'journey\nstyle T fill:#f9f;\nT: 1: a',
    // J14: the front matter title is not encoded; the last grammar title line wins.
    '---\ntitle: FM #9829;\n---\njourney\nT: 1: a',
    '---\ntitle: FM\n---\njourney\ntitle Grammar #9829;\ntitle',
    // J15: header texts are sanitised by DOMPurify.
    'journey\ntitle A <b>x</b> & y',
    'journey\ntitle <b>bold</b> text',
    'journey\naccTitle: a < b <i>c</i>\naccDescr: x &amp; <b>y</b>',
    'journey\naccDescr {\n  <img src=x onerror=alert(1)> text &lt; here\n}',
    // J16: a `<br>` in a title or a task name is text.
    'journey\ntitle Make<br/>tea<br>now\nMake<br>tea: 1: a',
    // J18: `journey` is a keyword wherever it stands.
    'journey\njourney\nT: 1: a',
    'journey\nJourney: 1: a',
    'journey\nT: 1: a\nJourney planning: 5: me',
    // J19: the text is trimmed at the start, and the blank lines before the keyword are not part of it.
    '\n\n   journey\nT: 1: a',
    '  %% a comment before the header\n\njourney\nT: 1: a',
    // J20: a title stops at `#` and `;`.
    'journey\ntitle a;b\nT: 1: a',
    'journey\ntitle a#b\nT: 1: a',
    // The plain cases: empty, blank lines, CRLF, indentation, and a missing data colon.
    'journey',
    'journey\n',
    'journey\nT: 1: a\n\n\n',
    'journey\r\nT: 1: a\r\nsection S\r\nU: 2: b\r\n',
    'journey\n  T: 1: a\n   section S\n  U: 2: b',
    'journey\nT: 1:a',
    'journey\nT: 1: a; b',
    'journey\nT: 1: a %% b',
    'journey\nsection S\nT:1:a',
    'journey\n accTitle: x',
    'journey\nT',
    'journey\nT:',
    'journey\n: 5: a',
    'journey\n;',
    'journey title x\nT: 1: a',
    // More of the same paths: tabs, no-break spaces, line separators, directives in the middle.
    'journey\n\tT:\t1:\ta\n\tsection\tS\n\tU:\t2:\tb',
    'journey\nT: 1: \u00a0me\u00a0,\u3000you\u3000',
    'journey\nT\u2028x: 1: a\nsection S\u2029U: 2: b',
    "journey\nT: 1: a\n%%{init: {'theme': 'dark'}}%%\nU: 2: b\n%%{init: {'theme': 'forest'}}%%",
    'journey\nsection  A\nT: 1: a\nsection \tB\nU: 2: b',
    'journey\naccTitle: a # b ; c\naccDescr: x # y\nT: 1: a',
    'journey\naccDescr {\n  a # b\n  c; d\n}\nT: 1: a',
    'journey\ntitle\nT: 1: a',
    'journey\ntitle     \nT: 1: a',
    'journey\ntitle\tTabbed title\nT: 1: a',
    'journey\nT: 1: #frac12; #Ntilde; #+5; #5; #NotAName; #hearts #hearts; x',
    '---\ntitle: [unclosed\n---\njourney\nT: 1: a',
    '---\ntitle: multi\n  line\n---\njourney\nT: 1: a',
    '---\n# no title\n---\njourney\nT: 1: a',
    '---\ntitle: 5\n---\njourney\nT: 1: a',
    '---\ntitle: \n---\njourney\ntitle x',
    'journey\nJourney\nT: 1: a',
    'journey\nT: 5: me\n\n\nsection S\n\n   \n  U: 2: b  \n  \n',
    'journey\nsection A\nsection B\nsection A\nT: 1: a\nU: 2: b\nsection C\nV: 3: c',
    'journey\nT: 1: a,b,c\nU: 1: , \nV: 1:\u00a0\nW: 1:',
  ],
};
