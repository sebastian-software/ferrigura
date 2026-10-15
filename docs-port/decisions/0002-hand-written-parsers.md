# Hand-written parsers; grammars stay as the specification

The Rust parsers are written by hand on one shared base: `logos` for lexing and recursive descent for the rules. They are not generated from the existing Jison (19) and Langium (18) grammars. Those grammars carry only half of the behaviour: lexer states, conditional lexing, string slicing in actions and calls into the diagram DB are JavaScript, and Langium adds TypeScript token builders and value converters, so a generator would still leave the other half to port by hand. Mermaid's syntax is line-oriented and context-sensitive (keywords allowed as node names, Markdown strings, `@{ … }` shape data), which recursive descent handles more directly and with better error positions.

The grammar files stay in the repository as the specification until the JS reference is removed.

## Considered Options

- **Translate the grammars to a Rust parser generator** (LALRPOP for the LALR Jison grammars, pest for the Langium ones): keeps two styles and still needs the actions and lexer states ported by hand.
- **A parser-combinator library** (`chumsky`, `winnow`): workable, but adds a dependency and an idiom for little gain over plain recursive descent.

## Consequences

Error messages are our own. Specs that assert the exact Jison error text are ported to assert that parsing fails and at which position.
