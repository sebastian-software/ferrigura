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
