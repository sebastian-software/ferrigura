/**
 * Collects the inputs of the differential run for one diagram type from the places the reference
 * already tests: the e2e fixtures, the string literals of the spec files and the mermaid blocks of
 * the docs. Identical texts are kept once, with the first place they were found in. The fixtures and
 * the cases come from this repository; the specs and the docs from the reference's checkout
 * (`reference-root.ts`), named by their path in it.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

import ts from 'typescript';

import { explicitCases, undetectedCases } from './cases.ts';
import type { PortInput } from './reference.ts';
import { referenceRoot, repositoryRoot } from './reference-root.ts';

/** The spec files whose literals are collected, where a type's specs are JavaScript. */
const specPatterns: Record<string, RegExp> = { er: /\.(spec|test)\.[jt]s$/ };

/**
 * The fixture folders of a type, where it is not `e2e/diagrams/<type>/`. The state folder names are
 * the reference's two headers.
 */
const fixtureFolders: Record<string, string[]> = {
  er: ['e2e/diagrams/er-diagram/'],
  class: ['e2e/diagrams/class-diagram/'],
  quadrantChart: ['e2e/diagrams/quadrant-chart/'],
  state: [
    'e2e/diagrams/state-diagram/',
    'e2e/diagrams/state-diagram-v2/',
    'e2e/platform/dev-diagrams/',
  ],
};

/**
 * The detectors of the reference, as its files write them (`packages/mermaid/src/diagrams/`). The XY
 * chart takes its `-beta` keyword too, as upstream Mermaid and ferrigura do, and ends at a word
 * boundary, as patch 08 of the reference makes it do.
 */
const detectors: Record<string, RegExp> = {
  pie: /^\s*pie/,
  flowchart: /^\s*(graph|flowchart)/,
  er: /^\s*erDiagram/,
  class: /^\s*classDiagram/,
  sequence: /^\s*sequenceDiagram/,
  state: /^\s*stateDiagram/,
  gantt: /^\s*gantt/,
  mindmap: /^\s*mindmap/,
  xychart: /^\s*xychart(?:-beta)?(?![\w-])/,
  quadrantChart: /^\s*quadrantChart/,
};

/** Front matter and directive lines in front of the keyword do not count for detection. */
export function looksLikeType(type: string, text: string): boolean {
  let body = text.replace(/^---[\t ]*\r?\n[\S\s]*?\r?\n---(?:\r?\n|$)/, '');
  body = body.replace(/^\s*%%{[\S\s]*?}%%\s*/, '');
  return detectors[type].test(body);
}

/**
 * The files under `dir` of the checkout at `root` that pass `filter`, relative to `root`.
 *
 * Dependency and build folders are not entered: the reference checkout links its workspace packages
 * into each other's `node_modules`, and following those links never ends. Links are not followed
 * anywhere else either, so every file is found once, under its own path.
 */
async function walk(
  root: string,
  dir: string,
  filter: (path: string) => boolean
): Promise<string[]> {
  const found: string[] = [];
  const pending = [join(root, dir)];
  for (let folder = pending.pop(); folder !== undefined; folder = pending.pop()) {
    for (const entry of await readdir(folder, { withFileTypes: true })) {
      const path = join(folder, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== 'node_modules' && entry.name !== 'dist') pending.push(path);
        continue;
      }
      if (!entry.isFile()) continue;
      const repoRelative = relative(root, path);
      if (filter(repoRelative)) found.push(repoRelative);
    }
  }
  return found.sort();
}

/** The string literals of a TypeScript file, with the line each starts on. */
function stringLiterals(file: string, source: string): { line: number; text: string }[] {
  const sourceFile = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true);
  const literals: { line: number; text: string }[] = [];
  const visit = (node: ts.Node): void => {
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart());
      literals.push({ line: line + 1, text: node.text });
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return literals;
}

/** The mermaid and mermaid-example blocks of a Markdown file. */
function mermaidBlocks(source: string): string[] {
  return [...source.matchAll(/```mermaid(?:-example)?\r?\n([\S\s]*?)```/g)].map(
    (match) => match[1] ?? ''
  );
}

export async function collectInputs(type: string): Promise<PortInput[]> {
  const candidates: { source: string; text: string }[] = [];

  // The state fixtures are in two folders, and some state inputs sit in the platform's dev folder.
  // Those are kept by detector.
  for (const folder of fixtureFolders[type] ?? [`e2e/diagrams/${type}/`]) {
    const fixtures = await walk(repositoryRoot, folder, (path) => path.endsWith('.mmd'));
    for (const fixture of fixtures) {
      const text = await readFile(join(repositoryRoot, fixture), 'utf8');
      if (type === 'state' && !looksLikeType(type, text)) continue;
      candidates.push({ source: fixture, text });
    }
  }

  const specPattern = specPatterns[type] ?? /\.(spec|test)\.(ts|js)$/;
  const specs = await walk(referenceRoot, 'packages/', (path) => specPattern.test(path));
  for (const spec of specs) {
    const source = await readFile(join(referenceRoot, spec), 'utf8');
    for (const literal of stringLiterals(spec, source)) {
      if (looksLikeType(type, literal.text)) {
        candidates.push({ source: `${spec}:${literal.line}`, text: literal.text });
      }
    }
  }

  const docs = await walk(referenceRoot, 'packages/mermaid/src/docs/', (path) =>
    path.endsWith('.md')
  );
  for (const doc of docs) {
    const source = await readFile(join(referenceRoot, doc), 'utf8');
    for (const block of mermaidBlocks(source)) {
      if (looksLikeType(type, block)) {
        candidates.push({ source: doc, text: block });
      }
    }
  }

  for (const text of [...(explicitCases[type] ?? []), ...(undetectedCases[type] ?? [])]) {
    candidates.push({ source: 'tools/port/cases.ts', text });
  }

  const seen = new Set<string>();
  const inputs: PortInput[] = [];
  for (const candidate of candidates) {
    if (seen.has(candidate.text)) continue;
    seen.add(candidate.text);
    inputs.push({ id: inputs.length, source: candidate.source, text: candidate.text });
  }
  return inputs;
}
