/**
 * The header of a pie model as the reference's pie DB holds it after parsing.
 *
 * The DB sets the front matter title first, then the grammar's title when it is non-empty
 * (`Diagram.fromText` and `populateCommonDb`). It also applies two whitespace rules to the
 * accessibility texts. The setters sanitise with DOMPurify, so these rules are kept here, and
 * `check-header.ts` compares them with the setters on inputs where sanitising changes nothing.
 * See `docs-port/canonical-model.md`.
 */

/** The header that the pie parser of `@mermaid-js/parser` gives. */
export interface PieHeaderAst {
  title?: string;
  accTitle?: string;
  accDescr?: string;
}

/** The header texts of a pie model. */
export interface PieHeader {
  title: string;
  accTitle: string;
  accDescr: string;
}

export function pieHeader(ast: PieHeaderAst, frontMatterTitle?: string): PieHeader {
  const grammarTitle = ast.title ?? '';
  return {
    // An empty grammar title falls through to the front matter title, as in `Diagram.fromText`.
    title: grammarTitle !== '' ? grammarTitle : (frontMatterTitle ?? ''),
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : '',
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : '',
  };
}
