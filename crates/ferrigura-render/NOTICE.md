# Notice: sources and licences of ferrigura-render

The crate is MIT-licensed, as Mermaid 12.2.0 is, whose renderers it ports. It also ports part of
marked 16.3.0, the Markdown lexer Mermaid reads Markdown labels with, which is MIT-licensed too.

| Source (marked 16.3.0, `src/`) | Ported in                                                                          |
| ------------------------------ | ---------------------------------------------------------------------------------- |
| `rules.ts`                     | `src/unified/marked.rs`: the block and inline rules of the GFM grammar, verbatim   |
| `Lexer.ts`, `Tokenizer.ts`     | `src/unified/marked.rs`: the lexer and the tokenizers `markdownToLines` depends on |
| `helpers.ts`                   | `src/unified/marked.rs`: `rtrim`, `splitCells` and `findClosingBracket`            |

## marked

Copyright (c) 2018+, MarkedJS (https://github.com/markedjs/)
Copyright (c) 2011-2018, Christopher Jeffrey (https://github.com/chjj/)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
