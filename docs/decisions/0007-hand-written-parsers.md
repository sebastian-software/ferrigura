# 0007 Hand-written parsers

**Scope:** the parsers of all diagram types in `ferrigura-syntax`, and their lexers. It does not cover the YAML front matter, the Markdown in labels or the configuration, which have readers of their own.

## Decision

The parsers are written by hand on one shared base: recursive descent for the rules, with shared rules for what every type has (`title`, `accTitle`, `accDescr`, front matter, directives). Lexers follow the grammar they port:

- where Mermaid's grammar is regular, the lexer uses `logos`;
- where a Jison grammar has start conditions and takes the first rule in its written order rather than the longest match, a state-stack scanner runs that rule table in order (`ferrigura-syntax/src/scan.rs`), keeping Jison's implicit word boundary after keyword literals.

Mermaid's Jison, Langium and Chevrotain grammars are the specification; the reference's parser decides what a text means.

## Why

The grammars carry only half of the behavior. Lexer states, conditional lexing, string slicing in actions and calls into the diagram DB are JavaScript, and Langium adds token builders and value converters in TypeScript, so a parser generator would still leave the other half to port by hand. Mermaid's syntax is line-oriented and context-sensitive (keywords allowed as node names, Markdown strings, `@{ … }` shape data), which recursive descent handles directly and with exact error positions.

Weighed and not chosen: a parser generator per grammar style (LALRPOP for Jison, pest for Langium), which keeps two styles and still needs the actions ported by hand; and a combinator library (`chumsky`, `winnow`), which adds a dependency and an idiom for little gain.

## Consequences

- Error messages are ferrigura's own. Ported specs that assert Mermaid's exact error text assert that parsing fails, and where, instead.
- Every parser is checked against the reference's parser and DB on every input by the differential parse ([0015](./0015-differential-testing.md)).

## Related

[0008](./0008-canonical-model-and-typed-views.md), [0015](./0015-differential-testing.md).
