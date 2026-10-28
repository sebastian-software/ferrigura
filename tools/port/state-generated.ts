/**
 * The placeholders of the trailing concurrency regions of a state document.
 *
 * The reference names a trailing region with `generateId()` (`Math.random`, `utils.ts`), so its id
 * and every id built from it (`<id>_start` of a `[*]` inside the region, the DOM ids) differ from run
 * to run. The port numbers the regions instead: `<generated:k>` for the k-th one, counted in the order
 * the reference creates them. `docTranslator` splits a document before it translates the document's
 * children, so a composite's trailing region is numbered before the regions and states inside it.
 * `ferrigura-syntax` (`state/db.rs`, `generated_id`) numbers them the same way, so the outputs of
 * both sides agree once each random id is replaced by its number.
 */

/** The id a trailing region has in the reference's output: a divider that the parser does not number. */
function isTrailingRegion(node: unknown): node is { id: string } {
  if (typeof node !== 'object' || node === null) return false;
  const record = node as Record<string, unknown>;
  return (
    record.stmt === 'state' &&
    record.type === 'divider' &&
    typeof record.id === 'string' &&
    !/^divider-id-\d+$/.test(record.id)
  );
}

/** The random id of each trailing region, with its placeholder. */
export function generatedPlaceholders(document: unknown[]): Map<string, string> {
  const found = new Map<string, string>();
  const visitDocument = (doc: unknown[]): void => {
    // The trailing region of a split document is its last statement.
    const last = doc[doc.length - 1];
    if (isTrailingRegion(last) && !found.has(last.id)) {
      found.set(last.id, `<generated:${found.size + 1}>`);
    }
    for (const statement of doc) visitStatement(statement);
  };
  const visitStatement = (node: unknown): void => {
    if (typeof node !== 'object' || node === null) return;
    const record = node as Record<string, unknown>;
    if (record.stmt === 'relation') {
      visitStatement(record.state1);
      visitStatement(record.state2);
    } else if (record.stmt === 'state' && Array.isArray(record.doc)) {
      visitDocument(record.doc);
    }
  };
  visitDocument(document);
  return found;
}

/** The value with every random id replaced by its placeholder, in strings of any depth. */
export function replaceGenerated<T>(value: T, placeholders: Map<string, string>): T {
  let json = JSON.stringify(value);
  for (const [id, placeholder] of placeholders) json = json.split(id).join(placeholder);
  return JSON.parse(json) as T;
}
