import { readFileSync } from 'node:fs';

/**
 * Explicit inputs of the differential run, added to the ones the reference already tests.
 *
 * Title precedence (`docs-port/canonical-model.md`): the grammar's title wins when it is
 * non-empty, and the front matter title fills in otherwise. The info DB has no title setter, so
 * the front matter title does not reach the info model. The accessibility cases pin the blank
 * handling of block and single-line values, which the DB whitespace rules must leave alone.
 */
/**
 * Explicit inputs that the detector of their type must not take: the reference and ferrigura both
 * leave them undetected, so the configuration run expects no diagram scope for them.
 */
export const undetectedCases: Record<string, string[]> = {
  // Detection is case-sensitive: a capitalised keyword is not a timeline.
  timeline: ['Timeline\nsection A\n  t: e'],
};

export const explicitCases: Record<string, string[]> = {
  // State inputs that the reference's specs do not cover: the quirks of the grammar and the DB
  // (`docs-port/state/PLAN.md`, section 7), and the error paths.
  state: [
    'stateDiagram-v2\n        state invalid syntax { Y }',
    'stateDiagram\nclassDef default fill:#f00\nA --> B',
    'stateDiagram\nA --> B; B --> C',
    'stateDiagram\ntitle Foo\nA --> B',
    'stateDiagram\ndirection LR\nA --> B\ndirection TB',
    'stateDiagram\nA --> B : foo direction TB\nC --> D',
    'stateDiagram\nA --> B\nstate A <<fork>>',
    'stateDiagram\nstate A <<fork>>\nA --> B',
    'stateDiagram\nstate foo\nA --> B',
    'stateDiagram\nstate foo : desc\nA --> B',
    'stateDiagram\nnote "text" as N1\nA --> B',
    'stateDiagram\nnote "a" as N1',
    'stateDiagram\nstate X {\n  A --> B\n  class A foo\n  note right of A : n\n}\nclassDef foo fill:#f00',
    'stateDiagram\nstate X {\n  A:::foo\n  class A bar\n  style A fill:#f00\n}',
    'stateDiagram\nA:::cls\nB --> A',
    'stateDiagram\nA:::cls --> B:::other',
    'stateDiagram\nclassDef foo fill:#f00\nclass A,B foo\nclass A foo2\nA --> B',
    'stateDiagram\nstyle A fill:#f00\nstyle A stroke:#0f0\nA --> B',
    'stateDiagram\nclassDef x fill:#f00;stroke:#333;\nstyle A fill:#f00;stroke:#333;\nA --> B',
    'stateDiagram\nclassDef hot color:#fff,fill:#f00\nclass A hot\nA --> B',
    'stateDiagram\nclassDef foo fill:#f00;color:#fff;x\nA --> B',
    'stateDiagram\nA --> B\nclick A "https://example.com" "tip"\nclick B href "https://b.example"',
    'stateDiagram\nclick A "https://example.com" "tip"  \nB --> A',
    'stateDiagram\nclick A "https://example.com" "tip"',
    'stateDiagram\nstate X {\n  click A "x" "y"\n}',
    'stateDiagram\nA --> B : label:with colon',
    'stateDiagram\nA --> B : info: key: value',
    'stateDiagram\nA --> B : \nC --> D',
    'stateDiagram\nmyState : status: active',
    'stateDiagram\nA : one\nA : two\nA --> B',
    'stateDiagram\nstate X {\n  a\n}\nX : one\nX : two',
    'stateDiagram\nstate "Long" as S {\n  a\n}\nS : extra',
    'stateDiagram\nstate "x" as S1:alt\nS1 --> B',
    'stateDiagram\nstate "x" as S1:alt:more\nS1 --> B',
    'stateDiagram\nstate "x" as S1:alt {\n  a\n}',
    'stateDiagram\nstate "  spaced  " as S\nS --> T',
    'stateDiagram\nstate "  spaced  " as S {\n  a\n}',
    'stateDiagram\nstate X {\n  [*] --> A\n  A --> [*]\n}\nX --> [*]',
    'stateDiagram\n[*]\n[*] --> A',
    'stateDiagram\n[*] : start desc\nA --> [*]',
    'stateDiagram\nstate X {\n  a --> b\n  --\n}',
    'stateDiagram\nstate X {\n  a --> b\n  --\n  c --> d\n  --\n  e\n}',
    'stateDiagram\nstate X {\n  a --> b\n  --\n  [*] --> c\n}',
    'stateDiagram\nstate X {\n  --\n  c --> d\n}',
    'stateDiagram\nstate X {\n  direction LR\n  a --> b\n  direction TB\n}',
    'stateDiagram\nstate X {\n  state Y {\n    a --> b\n    --\n    c\n  }\n  --\n  d\n}',
    'stateDiagram\nroot --> A\nA --> root',
    'stateDiagram\nnote right of A : hello\nA --> B\ndirection BT',
    'stateDiagram\nA --> B\nnote right of A\n  line one\n   line two\nend note',
    'stateDiagram\nA --> B\nnote right of A  : text',
    'stateDiagram\nA --> B\nNOTE LEFT OF A : up',
    'stateDiagram\nState1\nnote right of State1\n  this sentence contains send note inside\nend note\nState1 --> State2',
    'stateDiagram\nState1\nnote right of State1\n  this sentence contains end note as part\nend note\nState1 --> State2',
    'stateDiagram\nstate X {\n  a\n  note right of a : in\n}',
    'stateDiagram\nnote right of A\nA --> B',
    'stateDiagram\naccTitle:\nA --> B',
    'stateDiagram\naccTitle: T  \nA --> B',
    'stateDiagram\naccDescr {\n    first line\n    second line\n}\nA --> B',
    'stateDiagram\naccDescr {\n  text',
    'stateDiagram\nstate X {\n  accTitle: nope\n}',
    'stateDiagram\naccTitle: first\naccTitle: second\nA --> B',
    'stateDiagram\nhide empty description\nA --> B',
    'stateDiagram\nscale 300 width\nA --> B',
    'stateDiagram\nstate X {\n  scale 2 width\n}',
    'stateDiagram\n# hash\n%% percent\nA --> B %% inline\nB:::x %% tail\n',
    'stateDiagram\nA%%inline\nB%%{x\nC',
    '%%{init: {"theme": "dark"}}%%\nstateDiagram\nA --> B',
    '---\ntitle: My title\n---\nstateDiagram\nA --> B',
    '---\ntitle: "A & <b>B</b>"\n---\nstateDiagram\nA --> B',
    'stateDiagram\nmy-state --> B',
    'stateDiagram\nstate X {\n  a-b\n}',
    'stateDiagram\nA,B:::cls',
    'stateDiagram-v2\n[*] --> __proto__\n__proto__ --> [*]\n[*] --> constructor',
    'stateDiagram\nstate X {\n}',
    'stateDiagram\nstate "Big" as B {\n  state "inner" as I\n  I2: inner two\n  I --> I2\n}',
    'stateDiagram\nstate X {\n  a\n}\nclass X foo\nclassDef foo fill:#f00',
    'stateDiagram\nstate X {\n  a\n}\nstyle X fill:#f00',
    'stateDiagram\nstate F <<fork>>\nstate J <<join>>\nstate C <<choice>>\nF --> A\nA --> J\nC --> B',
    'stateDiagram\nstate F [[fork]]\nstate G [[choice]]\nF --> G',
    'stateDiagram\nstate <<fork>>',
    'stateDiagram\nstate A B {\n a\n}',
    'stateDiagram\nstate A  {\n a\n}',
    'stateDiagram\nstate "as" as as\nassemble\nassemblies\nstate assemble\nstate assemblies',
    'stateDiagram\ndefault --> A',
    'stateDiagram\nclick --> A',
    'stateDiagram\nA --> B -',
    'stateDiagram\nA -> B',
    'stateDiagram\nA - B',
    'stateDiagram\n--> A',
    'stateDiagram\nA :',
    'stateDiagram',
    'stateDiagram\n',
    'A --> B',
    'stateDiagram-v2A --> B',
    'STATEDIAGRAM-V2\naccTitle: T\nCLASSDEF foo FILL:#f00\nCLASS A foo\nNOTE RIGHT OF A : x\nDIRECTION LR\nA --> B',
    'stateDiagram\nNOTE LEFT OF A : n\nA --> B',
    'stateDiagram\nSTATE X {\n  a\n}',
    'stateDiagram\nA B C',
    'stateDiagram\nA --> B  C --> D',
    'stateDiagram\nA --> B : x; y',
    'stateDiagram\nnote right of A : a;b\nA --> B',
    'stateDiagram\nA --> B : &amp; <b>x</b>',
    'stateDiagram\nA : <script>bad()</script>ok\nA --> B',
    'stateDiagram\nnote right of A : <b>bold</b> & x\nA --> B',
    'stateDiagram\nstate "A<b>" as S {\n  x\n}',
    'stateDiagram\nclassDef foo\nA --> B',
    'stateDiagram\nclass A\nA --> B',
    'stateDiagram\nstyle A\nA --> B',
    'stateDiagram\nclass Z foo\nA --> B',
    'stateDiagram\nstyle A,B fill:#f00,stroke:#0f0\nA --> B',
    'stateDiagram\nclass A,B, foo\nA --> B',
  ],
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
  timeline: [
    // The front matter title is dropped; the grammar's title counts even when it is blank.
    '---\ntitle: From front matter\n---\ntimeline\ntitle From grammar\ntitle  ',
    '---\ntitle: Only front matter\n---\ntimeline\nsection A',
    // The keyword `timeline` needs a word boundary; `TD` and `LR` need one too.
    'timeline TDx\n  t1',
    'timelineX\n  t1',
    // An event needs a blank after its colon; a colon followed by a blank starts a new event.
    'timeline\n2004 : a:b\n2004:x',
    'timeline\n2004 : a: b',
    // An event before any period is an error.
    'timeline\n  : orphan',
    // The value of accTitle may be on the next line; at the end of the input it is an error.
    'timeline\naccTitle:\n  2004',
    'timeline\naccTitle:',
    'timeline\naccDescr: ',
    // An accessibility block: comment marks are text, the indentation goes, and the block must close.
    'timeline\naccDescr {\n  a # b\n  c %% d\n}',
    'timeline\naccDescr {\n  never closed',
    'timeline\naccDescr {}',
    // Comments: one character before `%%`, a whole line starting with `%`, and a `#` that ends a period.
    'timeline\nx%%c\nabc%%c\n%abc\n2004 # hidden',
    // Blank lines and blanks have no effect on the names.
    'timeline\n\n  section A \n\n  t1 \n',
    // A section name that repeats keeps both occurrences; each occurrence has its own periods.
    'timeline\nsection A\n  t1\nsection A\n  t2',
    // Entities are encoded; the header texts are sanitised, and the entity in a section is kept encoded.
    'timeline\ntitle AT&T <b>bold</b>\naccTitle: AT&T\nsection #quot;A#quot;\n  t: #9829;',
    'timeline\ntitle AT&T\n  accTitle:   spaced   out  ',
    // A second header is an error, and so is a header that is not the first token.
    'timeline\ntimeline TD',
    'timeline LR\nsection A\n  t: e',
    'timeline TD\ntitle x\n  t: e',
    // Statements need no line break between them.
    'timeline section A',
    // A directive is removed before the parse and does not change the grammar.
    '%%{init: {"theme": "forest"}}%%\ntimeline\nsection A\n  t: e',
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
  gitgraph: [
    // The DB's refusals: the head of the current branch belongs to the other branch, the same
    // branch, no commit yet, a duplicate merge id.
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\nmerge main',
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\nmerge b',
    'gitGraph\nmerge x',
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\ncommit id:"B"\ncheckout main\nmerge b id:"B"',
    // A duplicate commit id only warns: the entry is replaced and the commit is its own parent.
    'gitGraph\ncommit id:"a"\ncommit id:"a"',
    // Cherry-picks with a tag, without one (the default tag), with an empty one, and a merge source.
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\ncommit id:"B"\ncheckout main\ncherry-pick id:"B" tag:"picked"',
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\ncommit id:"B"\ncheckout main\ncherry-pick id:"B"',
    'gitGraph\ncommit id:"A"\nbranch b\ncheckout b\ncommit id:"B"\ncheckout main\ncherry-pick id:"B" tag:""',
    'gitGraph\ncommit id:"A"\nbranch b\ncommit id:"B"\ncheckout main\nmerge b id:"M"\ncheckout b\ncherry-pick id:"M"',
    'gitGraph\ncommit id:"A"\nbranch b\ncommit id:"B"\ncheckout main\nmerge b id:"M"\ncheckout b\ncherry-pick id:"M" parent:"B"',
    // Branch names that the lexer reads as numbers, as commands, or as quoted strings.
    'gitGraph\nbranch 1',
    'gitGraph\nbranch 1.0\ncommit',
    'gitGraph\nbranch 1-0',
    'gitGraph\nbranch commit',
    'gitGraph\nbranch "commit"\ncommit',
    'gitGraph\ncommitX',
    'gitGraph\ncommit\nbranch commitX\ncheckout commitX\ncommit',
    // A keyword followed by word characters is one REFERENCE; `checkout 02` is an INT, not a name.
    'gitGraph\ncommit\nbranch checkout02\ncheckout checkout02\ncommit',
    'gitGraph\ncommit\nbranch "02"\ncheckout 02\ncommit',
    // Titles and accessibility lines, and the order of a branch.
    'gitGraph\ntitle Git title\ncommit',
    'gitGraph\ntitle X %% comment\ncommit',
    'gitGraph\naccDescr {\n  one\n  two\n}\ncommit',
    'gitGraph\naccTitle: Spaced title  \ncommit',
    'gitGraph\ncommit\nbranch b order: 2\nbranch c order: 1',
    // The header: a direction with a colon, and one without (an error).
    'gitGraph TB:\ncommit',
    'gitGraph BT:\ncommit',
    'gitGraph LR\ncommit',
    'gitGraph commit',
    // The commit types on commits and merges, with and without the option.
    'gitGraph\ncommit type: REVERSE\ncommit type: HIGHLIGHT\ncommit type: NORMAL',
    'gitGraph\ncommit id:"A"\nbranch b\ncommit id:"B"\ncheckout main\nmerge b id:"M" type: REVERSE\nmerge b type: HIGHLIGHT\nmerge b tag:"x"',
    // Messages: the `msg:` option and a bare string.
    'gitGraph\ncommit msg:"hello there"\ncommit "bare message"',
    // Ids and tags with characters the sanitiser escapes.
    'gitGraph\ncommit id:"a=b" msg:"x<y"\ncommit id:"x<y" tag:"<b>t</b>"\ncommit id:"AT&T"',
    // Empty values: a commit with an empty tag, and empty ids, which are generated.
    'gitGraph\ncommit tag:""',
    'gitGraph\ncommit id:""\ncommit id:""',
    // The title from front matter, and from the grammar, which wins.
    '---\ntitle: From front matter\n---\ngitGraph\ntitle From grammar\ncommit',
    '---\ntitle: Only front matter\n---\ngitGraph\ncommit',
    // The main branch's name and order from front matter, and from a directive.
    '---\nconfig:\n  gitGraph:\n    mainBranchName: trunk\n    mainBranchOrder: 3\n---\ngitGraph\ncommit\nbranch dev\ncommit',
    '%%{init: {"gitGraph": {"mainBranchName": "trunk"}}}%%\ngitGraph\ncommit',
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
  gantt: [
    'gantt\nsection\nA :a1, 2014-01-01, 1d',
    'gantt\nsection X\ntitle\nA :a1, 2014-01-01, 1d',
    'gantt\ndateFormat x\nA :a1, 9999999999999999, 1d',
    'gantt\nA :active',
    'gantt\nclick A call f\nA :a1, 2014-01-01, 1d',
    'gantt\nA :a1, 2014-01-01, 1d\nB :call',
    'gantt\nA :a1, 2014-01-01, 1d\naccDescr {\nhello',
    'gantt\naccTitle:\nsection X',
    'gantt\nsection A\nB :a1, 2014-01-01, 1d\nexcludes weekends\nC :a2, after a1, 5d',
    'gantt\nclick a1 href "https://example.org/x y"\nA :a1, 2014-01-01, 1d',
    'gantt\nclick a1 href "javascript:alert(1)"\nA :a1, 2014-01-01, 1d',
    'gantt\nclick a1 call f()\nA :a1, 2014-01-01, 1d',
    'gantt\nclick a1 href "https://x.org" call f(a)\nA :a1, 2014-01-01, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nA :a1, 2014-01-01, 1d\nB :after x, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nA :a1, 2014-01-01, 1d\nB :after a1, 1d\nC :until a1, 1d',
    'gantt\nA :vert, v1, 2014-01-01, 1d\nB :milestone, m1, 2014-01-02, 0d',
    'gantt\ntitle  Foo  \nsection S\nA :a1, 2014-01-01, 2d',
    'gantt\ntitle <script>alert(1)</script>\nA :a1, 2014-01-01, 1d',
    'gantt\nexcludes 2014-01-02, WEEKENDS\nincludes 2014-01-04\nA :a1, 2014-01-01, 5d',
    'gantt\nweekday monday\nweekend friday\ntickInterval 1day \nA :a1, 2014-01-01, 1d',
    'gantt\ndateFormat HH:mm\nA :a1, 10:30, 1h',
    'gantt\ndateFormat D\nA :a1, 1, 1d',
    'gantt\ndateFormat MMM D\nA :a1, Jan 2, 1d',
    'gantt\ndateFormat X\nA :a1, 120, 1d',
    'gantt\nA :a1, 2014-02-30, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nA :a1, 2014-01-06, 2014-01-08\nB :2014-01-02, 3d',
    'gantt\nA :a1, 2014/01/06, 1d',
    'gantt\nA :a1, 2014-01-06T10:00, 1d',
    'gantt\nA :a1, 2014-01-06, 1d\nB :after a1 b2, 2d',
    'gantt\n%% comment\nA :a1, 2014-01-06, 1d',
    'gantt\nsection A\n\n\nB :a2, 2014-01-06, 1d',
    'gantt\nA :milestone, m1, 2014-01-06, 0d',
    'gantt\nA :crit, active, a1, 2014-01-06, 1d',
    'gantt\nA :done, crit, 2014-01-06, 1d',
    'gantt\nA :a1, 2014-01-01, 1d, 2d',
    'gantt\naccDescr: hello\n   world  \naccTitle: T',
    'gantt\naccDescr {\n  line one\n\n  line two\n}',
    'gantt\ndateFormat ss\ntask1 :0, 20',
    'gantt\ndateFormat  D\nA :a1, 1, 1d',
    'gantt\ntodayMarker off\ntickInterval 1week\nA :a1, 2014-01-01, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nA :a1, 2014-01-01, 1d\nB :after zz, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nA :a1, 2014-01-01, 1d\nexcludes wednesday\nB :a2, after a1, 3d',
    'gantt\nsection A\nB :a1, 2014-01-01, 1d\nC :after a1, 1d\nD :a2, 2014-01-01, until a1',
    'gantt\nclick A href "https://a.b"\nA :a1, 2014-01-01, 1d',
    'gantt\ndateFormat YYYY-MM-DD\nt12 :after t11, 1d\nt11 :after t10, 1d\nt10 :after t9, 1d\nt9 :after t8, 1d\nt8 :after t7, 1d\nt7 :after t6, 1d\nt6 :after t5, 1d\nt5 :after t4, 1d\nt4 :after t3, 1d\nt3 :after t2, 1d\nt2 :after t1, 1d\nt1 :a1, 2014-01-01, 1d',
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
