# Hand-written parsers; grammars stay as the specification

The Rust parsers are written by hand on one shared base: `logos` for lexing and recursive descent for the rules. They are not generated from the existing Jison (19) and Langium (18) grammars. Those grammars carry only half of the behaviour: lexer states, conditional lexing, string slicing in actions and calls into the diagram DB are JavaScript, and Langium adds TypeScript token builders and value converters, so a generator would still leave the other half to port by hand. Mermaid's syntax is line-oriented and context-sensitive (keywords allowed as node names, Markdown strings, `@{ … }` shape data), which recursive descent handles more directly and with better error positions.

The grammar files stay in the repository as the specification until the JS reference is removed.

Lexers follow the grammar. Where the grammar is regular, the shared base uses `logos`, as pie and info do. Where the Jison grammar has start conditions and takes the first rule in its written order rather than the longest match, a state-stack scanner runs that rule table in order (`ferrigura-syntax/src/scan.rs` holds the shared helpers). The flowchart is the first user; the other Jison grammars with start conditions (sequence, class, state, ER, gantt) will use the same helpers. Its rules need lookahead and state-dependent matches that `logos` cannot express, and the implicit word boundary of keyword literals is a Jison rule that the state-stack scanner keeps.

## Considered Options

- **Translate the grammars to a Rust parser generator** (LALRPOP for the LALR Jison grammars, pest for the Langium ones): keeps two styles and still needs the actions and lexer states ported by hand.
- **A parser-combinator library** (`chumsky`, `winnow`): workable, but adds a dependency and an idiom for little gain over plain recursive descent.

## Consequences

Error messages are our own. Specs that assert the exact Jison error text are ported to assert that parsing fails and at which position.
