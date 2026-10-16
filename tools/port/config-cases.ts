/**
 * Explicit inputs for the configuration differential run: the precedence of front matter and
 * directives, the key gating and the sanitising, the directive detection and its error cases, and
 * the `initialize` settings. The pie inputs of `inputs.ts` are run as well.
 */

export interface ConfigCase {
  source: string;
  text: string;
  /** The `initialize` settings the text runs with. */
  site?: Record<string, unknown>;
}

export const configCases: ConfigCase[] = [
  { source: 'config case: no configuration', text: 'pie\n"a": 1' },
  {
    source: 'config case: front matter config',
    text: '---\nconfig:\n  theme: dark\n  look: neo\n---\npie\n"a": 1',
  },
  {
    source: 'config case: init directive',
    text: '%%{init: {"theme": "forest"}}%%\npie\n"a": 1',
  },
  {
    source: 'config case: initialize directive with single quotes',
    text: "%%{initialize: {'theme': 'neutral'}}%%\npie",
  },
  {
    source: 'config case: directive overrides front matter',
    text: '---\nconfig:\n  theme: dark\n  look: neo\n---\n%%{init: {"theme": "forest"}}%%\npie',
  },
  {
    source: 'config case: several init directives',
    text: '%%{init: {"theme": "dark"}}%%\n%%{init: {"theme": "forest", "look": "neo"}}%%\npie',
  },
  {
    source: 'config case: directive inside the diagram',
    text: 'pie\n%%{init: {"theme": "dark"}}%%\n"a": 1',
  },
  {
    source: 'config case: config key moves under the pie section',
    text: '%%{init: {"config": {"donutHole": 0.3}}}%%\npie',
  },
  {
    source: 'config case: pie section of the directive',
    text: '%%{init: {"theme": "dark", "pie": {"theme": "forest", "textPosition": 0.5}}}%%\npie',
  },
  {
    source: 'config case: securityLevel from a directive is dropped',
    text: '%%{init: {"securityLevel": "loose", "theme": "forest"}}%%\npie',
  },
  {
    source: 'config case: securityLevel from a directive with initialize',
    text: '%%{init: {"securityLevel": "loose"}}%%\npie',
    site: { securityLevel: 'antiscript' },
  },
  {
    source: 'config case: unknown directive key is dropped',
    text: '%%{init: {"nonsense": 1, "theme": "dark"}}%%\npie',
  },
  {
    source: 'config case: invalid JSON drops every directive',
    text: '%%{init: {"theme": "forest"}}%%\n%%{init: {theme: dark}}%%\npie',
  },
  {
    source: 'config case: wrap directive',
    text: '%%{wrap}%%\npie',
  },
  {
    source: 'config case: two wrap directives',
    text: '%%{wrap}%%\n%%{wrap}%%\npie',
  },
  {
    source: 'config case: wrapper is not wrap',
    text: '%%{wrapper}%%\npie',
  },
  {
    source: 'config case: init without arguments',
    text: '%%{init}%%\npie',
  },
  {
    source: 'config case: null values are removed',
    text: '%%{init: {"theme": null, "look": "neo"}}%%\npie',
  },
  {
    source: 'config case: array-valued key is dropped',
    text: '%%{init: {"secure": ["theme"], "theme": "dark"}}%%\npie',
  },
  {
    source: 'config case: prototype key is dropped',
    text: '%%{init: {"__proto__": {"x": 1}, "theme": "dark"}}%%\npie',
  },
  {
    source: 'config case: markup in a string is dropped',
    text: '%%{init: {"themeCSS": "<b>x</b>", "look": "neo"}}%%\npie',
  },
  {
    source: 'config case: unbalanced fontFamily',
    text: '%%{init: {"fontFamily": "a { b"}}%%\npie',
  },
  {
    source: 'config case: odd theme variable value',
    text: '%%{init: {"themeVariables": {"primaryColor": "#fff", "lineColor": "<x>"}}}%%\npie',
  },
  {
    source: 'config case: unknown look is ignored',
    text: '%%{init: {"look": "bogus"}}%%\npie',
  },
  {
    source: 'config case: unknown theme falls through to initialize',
    text: '%%{init: {"theme": "nope"}}%%\npie',
    site: { theme: 'neutral' },
  },
  {
    source: 'config case: initialize theme with a directive look',
    text: '%%{init: {"look": "neo"}}%%\npie',
    site: { theme: 'forest' },
  },
  {
    source: 'config case: initialize keeps unknown keys',
    text: 'pie',
    site: { unknownKey: 1, fontSize: 20 },
  },
  {
    source: 'config case: initialize pie section merges with directive',
    text: '%%{init: {"pie": {"donutHole": 0.4}}}%%\npie',
    site: { pie: { textPosition: 0.5 } },
  },
  {
    source: 'config case: legacy displayMode goes to gantt',
    text: '---\ndisplayMode: compact\n---\npie',
  },
  {
    source: 'config case: invalid front matter YAML',
    text: '---\nconfig: [\n---\npie',
  },
  {
    source: 'config case: number directive value',
    text: '%%{init: {"fontSize": 20, "maxEdges": 100}}%%\npie',
  },
  {
    source: 'config case: comment line and directive closer',
    text: '%%}%%ignored\n%% a comment\n%%{init: {"theme": "dark"}}%%\npie',
  },
  {
    source: 'config case: apostrophes in labels',
    text: 'pie\n"it\'s": 1',
  },
];
