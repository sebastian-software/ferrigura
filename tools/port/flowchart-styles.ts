/**
 * The flowchart style sheet of the TypeScript reference, for the differential test of
 * `ferrigura-render`'s `style::style_sheet` (`crates/ferrigura-render/tests/flowchart_styles.rs`).
 *
 * For each case this runs the reference's `createUserStyles` (`mermaidAPI.ts`) with the flowchart
 * styles registered, as `diagrams/flowchart/flowDiagram.ts` registers them. The configuration is the
 * one the differential harness gives the reference: `htmlLabels: false` (SVG text), the top-level
 * `fontFamily` the harness sets, which `initialize` moves into the theme variables, and the theme and
 * look of the case. The classDef cases take their classes from the reference's flowchart DB, which
 * parses the fixture text. The output is the reference's compiled CSS, as stylis writes it, with the
 * classes the DB stores.
 *
 * Usage: `pnpm port:styles-flowchart` (writes `crates/ferrigura-render/tests/data/flowchart-styles.json`).
 */
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import { JSDOM } from 'jsdom';

import { registerDetectors, openReference, type Reference } from './reference.ts';

const dom = new JSDOM('<!doctype html><html><body></body></html>');
Object.assign(globalThis, {
  window: dom.window,
  document: dom.window.document,
  CSSStyleSheet: dom.window.CSSStyleSheet,
});

/** The id of the SVG element the sheet is scoped to. Ferrigura's test uses the same id. */
const svgId = 'flowchart';

/** The font family the harness gives the reference, as the top-level `fontFamily` option. */
const fontFamily = '"Lexica Ultralegible"';

/** The `classDef` statements of the classDef cases. */
const classDefFixture = [
  'flowchart LR',
  '  classDef green fill:#9f6,stroke:#333,stroke-width:2px',
  '  classDef red color:#fff,fill:#f00',
  '  classDef banner color:#fff,background-color:#0f0',
  '  classDef muted color:#333',
  '  a:::green --> b:::red',
  '  c:::banner --> d:::muted',
].join('\n');

interface Case {
  name: string;
  theme: 'redux-color' | 'redux-dark-color';
  look: 'neo' | 'classic';
  /** Overrides of the theme variables that the case adds, as the user's `themeVariables`. */
  overrides?: Record<string, unknown>;
  /** The `classDef` statements of the case, if any. */
  classDefText?: string;
}

const cases: Case[] = [
  { name: 'neo-light', theme: 'redux-color', look: 'neo' },
  { name: 'classic-light', theme: 'redux-color', look: 'classic' },
  { name: 'neo-dark', theme: 'redux-dark-color', look: 'neo' },
  { name: 'classic-dark', theme: 'redux-dark-color', look: 'classic' },
  {
    name: 'neo-light-gradient',
    theme: 'redux-color',
    look: 'neo',
    overrides: { useGradient: true },
  },
  { name: 'classdef-neo-light', theme: 'redux-color', look: 'neo', classDefText: classDefFixture },
];

interface ThemeModule {
  getThemeVariables(userOverrides?: Record<string, unknown>): Record<string, unknown>;
}

interface StylesModule {
  createUserStyles(
    config: Record<string, unknown>,
    graphType: string,
    classDefs: Map<string, unknown> | undefined,
    svgId: `#${string}`
  ): string;
  addStylesForDiagram(type: string, diagramTheme?: unknown): void;
}

interface FlowStylesModule {
  default: unknown;
}

/** A class as the flowchart DB stores it: its styles, and the text styles of its colours. */
interface DbClass {
  id: string;
  styles: string[];
  textStyles: string[];
}

interface FlowDb {
  clear(): void;
  classes: Map<string, DbClass>;
}

interface FlowParser {
  default: { parser: { yy: unknown }; parse(text: string): unknown };
}

interface Modules {
  createUserStyles: StylesModule['createUserStyles'];
  flowchartStyles: unknown;
  themes: Record<Case['theme'], ThemeModule>;
  preprocess: { preprocessDiagram(code: string): { code: { cleaned: string } } };
  encode: { encodeEntities(text: string): string };
  flowParser: FlowParser;
  flowDb: { FlowDB: new () => FlowDb };
}

async function loadModules(reference: Reference): Promise<Modules> {
  await registerDetectors(reference);
  const [mermaidApi, styles, flowchartStyles, redux, dark, preprocess, encode, flowParser, flowDb] =
    await Promise.all([
      reference.load<{ createUserStyles: StylesModule['createUserStyles'] }>(
        '/packages/mermaid/src/mermaidAPI.ts'
      ),
      reference.load<StylesModule>('/packages/mermaid/src/styles.ts'),
      reference.load<FlowStylesModule>('/packages/mermaid/src/diagrams/flowchart/styles.ts'),
      reference.load<ThemeModule>('/packages/mermaid/src/themes/theme-redux-color.js'),
      reference.load<ThemeModule>('/packages/mermaid/src/themes/theme-redux-dark-color.js'),
      reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
      reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
      reference.load<FlowParser>('/packages/mermaid/src/diagrams/flowchart/parser/flowParser.ts'),
      reference.load<Modules['flowDb']>('/packages/mermaid/src/diagrams/flowchart/flowDb.ts'),
    ]);
  styles.addStylesForDiagram('flowchart', flowchartStyles.default);
  return {
    createUserStyles: mermaidApi.createUserStyles,
    flowchartStyles: flowchartStyles.default,
    themes: { 'redux-color': redux, 'redux-dark-color': dark },
    preprocess,
    encode,
    flowParser,
    flowDb,
  };
}

/** The `classDef` classes of the fixture, as the reference's flowchart DB stores them. */
function classesOf(text: string, modules: Modules): DbClass[] {
  const { preprocess, encode, flowParser, flowDb } = modules;
  const { code } = preprocess.preprocessDiagram(text);
  const db = new flowDb.FlowDB();
  db.clear();
  flowParser.default.parser.yy = db;
  flowParser.default.parse(encode.encodeEntities(code.cleaned) + '\n');
  return [...db.classes.values()].map((classDef) => ({
    id: classDef.id,
    styles: [...classDef.styles],
    textStyles: [...classDef.textStyles],
  }));
}

const outputPath =
  process.argv[2] ??
  fileURLToPath(
    new URL('../../crates/ferrigura-render/tests/data/flowchart-styles.json', import.meta.url)
  );

const reference = await openReference();
try {
  const modules = await loadModules(reference);
  const results = cases.map((testCase) => {
    const themeVariables = modules.themes[testCase.theme].getThemeVariables({
      fontFamily,
      ...testCase.overrides,
    });
    const config = {
      theme: testCase.theme,
      look: testCase.look,
      htmlLabels: false,
      fontFamily,
      themeVariables,
    };
    const classes = testCase.classDefText ? classesOf(testCase.classDefText, modules) : [];
    const classDefs = new Map(classes.map((classDef) => [classDef.id, classDef]));
    const css = modules.createUserStyles(config, 'flowchart', classDefs, `#${svgId}`);
    return {
      name: testCase.name,
      theme: testCase.theme,
      look: testCase.look,
      overrides: testCase.overrides ?? {},
      classDefs: classes,
      css,
    };
  });
  const output = {
    generator: 'tools/port/flowchart-styles.ts',
    reference: 'packages/mermaid/src/mermaidAPI.ts createUserStyles',
    svgId,
    fontFamily,
    cases: results,
  };
  await writeFile(outputPath, JSON.stringify(output, null, 2) + '\n');
  console.log(`wrote ${results.length} cases to ${outputPath}`);
} finally {
  await reference.close();
}
