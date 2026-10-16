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
};
