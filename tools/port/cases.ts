import { readFileSync } from 'node:fs';

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
  er: [
    // Value states at the end of the input: the value is empty, so the end marker follows without `EOF`.
    'erDiagram\naccTitle:',
    'erDiagram\naccTitle: foo',
    'erDiagram\naccDescr { foo',
    // A style line with blanks before its line break: the break is swallowed by the style state.
    'erDiagram\nA ||--o{ B : x\nstyle A color:red \nC ||--o{ D : y',
    // A direction takes the rest of its line, and the next line starts a statement.
    'erDiagram\nA ||--o{ B : has direction TB\nC ||--o{ D : x',
    // The members of a subgraph include the names the grammar keeps from other statements.
    'erDiagram\nsubgraph G\nclassDef foo fill:#f9f\naccTitle: T\nA ||--o{ B : x\nend',
    // Nested subgraphs: the inner id is a member of the outer one, and a direction is its own.
    'erDiagram\nsubgraph outer\n  A\n  subgraph inner\n    direction RL\n    B\n  end\nend\ndirection TB\nC',
    // Classes on entities and subgraphs, and attribute blocks with keys, comments and generics.
    'erDiagram\nclass A,B c1,c2\nA:::c3 {\n  string name PK, UK "comment"\n  type~T~ tag\n  `my type` x\n}\nsubgraph A\nB\nend',
    // Alias forms: the alias is kept on the first definition only.
    'erDiagram\nc[First]:::k\nc[Second] {}\nd[Other] ||--o| c : x',
    // Header texts and the front-matter title are sanitised.
    '---\ntitle: A & B\n---\nerDiagram\naccTitle:   AT&T  \naccDescr {\n  line one\n\n  line two\n}\nsubgraph "G & H"\nA\nend',
    // Quoted roles may span lines; a quote inside a name is not allowed.
    'erDiagram\nA ||--o{ B : "multi\nline"\nC ||--o{ "x"y : z',
    // Parent-child and cardinality words, and the `1` in its three readings.
    'erDiagram\nPROJECT u--o{ TEAM : "p"\nA 1--1 B : x\nC 1..one or more D : y\n1 ||--|| 2 : z',
    // Edge cases of the lexer: a direction across a line break, and a non-ASCII blank as a name character.
    'erDiagram\nA ||--o{ B : direction\nTB\nC',
    'erDiagram\nA\u2028B ||--o{ C : x',
    'erDiagram\nA\u00a0B ||--o{ C : x',
    'erDiagram\nA ||--o{ B : x direction LR direction TB',
    // Blocks: a generic type that runs to the last tilde of the line, backticks across line breaks, and an empty pair.
    'erDiagram\nA {\nmap~K,V~ data\n}',
    'erDiagram\nA {\nmap~K,V~ data "x~y"\n}',
    'erDiagram\nA {\nstring `a\nb` PK\n}',
    'erDiagram\nA {\nstring `` x\n}',
    'erDiagram\nA {\nstring `open\n}',
    // Number readings of `1` and the `end` keyword with a hyphen.
    'erDiagram\nA 1a B : x\nA 1-B : y\nA 1 1 B : z\n1-- 1..x 1',
    'erDiagram\nsubgraph G\nA ||--o{ B : x\nend-x\nC',
    // A directive in front of the diagram, a comment in the middle, and a Markdown-like title.
    '%%{init: {"theme": "dark"}}%%\nerDiagram\n%% a comment\nA ||--o{ B : "x"',
    // Style and class lines: a semicolon, a stray comma, and a line with a dot.
    'erDiagram\nA\nclassDef foo fill:#f9f;\nstyle A fill:#f9f,\nB',
    'erDiagram\nA\nstyle A fill:red.5',
    // The subgraph header with a separator other than a line break.
    'erDiagram\nsubgraph G end',
    'erDiagram\nsubgraph G [a b] \nA\nend',
    'erDiagram\nsubgraph G [a [b]]\nA\nend',
    ...erCharacterCases(),
  ],
};

/**
 * The texts of the generated character cases of `erDiagram.spec.js`: the characters of its list,
 * each in the five forms of its loop, and all of them together. They are read from the spec, so the
 * list is the reference's.
 */
function erCharacterCases(): string[] {
  const spec = readFileSync(
    new URL('../../packages/mermaid/src/diagrams/er/parser/erDiagram.spec.js', import.meta.url),
    'utf8'
  );
  const list = /const chars =\s*"([^"]*)";/.exec(spec)?.[1] ?? '';
  const characters = list.split(' ');
  const texts: string[] = [];
  for (const c of characters) {
    const single = `Blo${c}rf`;
    const repeated = `Blo${c}${c}rf`;
    texts.push(
      `erDiagram\n ${single}\n`,
      `erDiagram\n "${single}"\n`,
      `erDiagram\n "${repeated}"\n`
    );
    texts.push(`erDiagram\n "Blorf${c}"\n`);
  }
  texts.push(`erDiagram\n "a${characters.join('')}"\n`);
  return texts;
}
