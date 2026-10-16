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
  sequence: [
    // Header texts: the last title wins; a blank title is kept; the body title beats the front matter.
    'sequenceDiagram\ntitle First\ntitle Second\nA->>B: x',
    'sequenceDiagram\ntitle   \nA->>B: x',
    '---\ntitle: Front\n---\nsequenceDiagram\ntitle  \nA->>B: x',
    '---\ntitle: Front\n---\nsequenceDiagram\ntitle Body\nA->>B: x',
    'sequenceDiagram\ntitle  Two spaces\nA->>B: x',
    // A line break after `accTitle:` takes the next line as the value; `;` and `#` are kept.
    'sequenceDiagram\naccTitle:\nOrder\nA->>B: x',
    'sequenceDiagram\naccTitle: Order; #1\nA->>B: x',
    'sequenceDiagram\naccDescr {\n   first line\n\n     second line\n}\nA->>B: x',
    // A repeated declaration with an alias overwrites the actor; an implicit re-reference keeps it.
    'sequenceDiagram\nparticipant A as Alice\nparticipant A as Bob\nA->>A: self',
    'sequenceDiagram\nparticipant A as Alice\nB->>A: hi\nA->>B: back',
    // `create` of an existing id, a message that skips the created actor, a message after `destroy`.
    'sequenceDiagram\nA->>B: hi\ncreate participant B\nB->>A: back',
    'sequenceDiagram\ncreate participant B\nA->>C: hi',
    'sequenceDiagram\nA->>B: hi\ndestroy B\nA->>C: x',
    'sequenceDiagram\ncreate participant B\nA->>B: hi\ndestroy B\nB->>A: bye',
    // Boxes: `#` starts a comment, so the box has no name; a colour name is case-insensitive;
    // CSS-wide keywords are colours; an actor in two boxes is an error.
    'sequenceDiagram\nbox #Purple Alice\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox Purple Alice\nparticipant A\nend\nparticipant B\nA->>B: x',
    'sequenceDiagram\nbox inherit Team\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox rgb(10, 20, 30) Team\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox rgb(1 2 3) Team\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox hsl(120deg, 100%, 50%) Team\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox RGB(1,2,3) Team\nparticipant A\nend\nA->>A: x',
    'sequenceDiagram\nbox Red X\nparticipant A\nend\nbox Blue Y\nparticipant A\nend',
    // Actor metadata: JSON links, an `@`-link with a space, one without (its label loses two
    // characters), properties, and details with no element to read.
    'sequenceDiagram\nparticipant A\nlinks A: {"Dash": "https://dash.test"}\nA->>B: x',
    'sequenceDiagram\nparticipant A\nlink A: Dashboard @ https://dash.test\nA->>B: x',
    'sequenceDiagram\nparticipant A\nlink A: Dash@https://dash.test\nA->>B: x',
    'sequenceDiagram\nparticipant A\nproperties A: {"role": "api", "rank": 2}\nA->>B: x',
    'sequenceDiagram\nparticipant A\ndetails A: missing\nA->>B: x',
    // Autonumber: the bare form, the start and step, and `off`.
    'sequenceDiagram\nautonumber\nA->>B: one',
    'sequenceDiagram\nautonumber 10 2\nA->>B: one\nautonumber off\nA->>B: two',
    // Blocks and notes: `break` drops its end text; a note over two actors; a rect with a colour;
    // an alt with an else.
    'sequenceDiagram\nA->>B: one\nbreak when it fails\nA->>B: retry\nend',
    'sequenceDiagram\nNote over A,B: both',
    'sequenceDiagram\nrect rgb(191, 223, 255)\nA->>B: x\nend',
    'sequenceDiagram\nalt yes\nA->>B: one\nelse no\nA->>B: two\nend',
    // The four central connections, and the activation markers that go with them.
    'sequenceDiagram\nA->>() B: connect',
    'sequenceDiagram\nA () ->> B: reply',
    'sequenceDiagram\nA ()->>() B: both',
    'sequenceDiagram\nactivate A\nA->>B: x\ndeactivate A',
    'sequenceDiagram\nA->>B: x\ndeactivate B',
    // `wrap:` and `nowrap:` after the colon of a message and of a note.
    'sequenceDiagram\nA->>B: wrap: a long text',
    'sequenceDiagram\nparticipant A\nNote over A: wrap: text',
    // Metadata of an actor: an unknown type is kept, `alias` sets the description, a number is kept.
    'sequenceDiagram\nparticipant A@{ type: "bogus" }\nA->>B: x',
    'sequenceDiagram\nparticipant A@{ alias: "Alice" }\nA->>B: x',
    'sequenceDiagram\nparticipant A@{ type: 5 }\nA->>B: x',
    // An actor name with a dash is one actor; `participant A:B` is a parse error, not a dropped line.
    'sequenceDiagram\nA-B->>C: x',
    'sequenceDiagram\nparticipant A:B\nA->>B: x',
  ],
};
