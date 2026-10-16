/**
 * Canonical JSON of gitGraph diagrams, from the TypeScript reference.
 *
 * The steps are those of `Diagram.fromText` after the configuration of the input is set: the DB
 * is cleared, the front matter title is set, and the text (with `encodeEntities` and a line break)
 * goes to the parser, which fills the singleton DB of `gitGraphAst.ts`. The model is read out of
 * that DB. The reference's DOMPurify needs a window, as for the flowchart.
 *
 * The reference draws the seven hex digits of an id that the input does not give from
 * `Math.random`. The DB calls that can draw them are wrapped, and a commit, merge or cherry-pick
 * whose call drew random values is marked `autoId`. Its id, and every parent and branch head that
 * names it, is then replaced by `auto#k`, as `normalise_auto_ids` in
 * `crates/ferrigura-syntax/examples/canonical.rs` does on the ferrigura side. The random values
 * themselves are not compared. See `docs-port/gitgraph/PLAN.md`, section 7.
 *
 * Usage: `tsx tools/port/export-gitgraph.ts inputs.json outputs.json`
 */
import { provideDomForSanitising, registerDetectors, type Reference } from './reference.ts';
import { runExporter } from './exporter.ts';

type CommitTypeName = 'NORMAL' | 'REVERSE' | 'HIGHLIGHT' | 'MERGE' | 'CHERRY_PICK';

/** A commit as the DB stores it (`Commit` in `gitGraphTypes.ts`), with the type as a number. */
interface DbCommit {
  id: string;
  message: string;
  seq: number;
  type: number;
  tags: string[];
  parents: string[];
  branch: string;
  customType?: number;
  customId?: boolean;
}

interface GitGraphDb {
  commitType: Record<CommitTypeName, number>;
  clear: () => void;
  getConfig: () => { mainBranchOrder: number };
  setDiagramTitle: (text: string) => void;
  getDiagramTitle: () => string;
  getAccTitle: () => string;
  getAccDescription: () => string;
  getDirection: () => string;
  getCurrentBranch: () => string;
  getBranches: () => Map<string, string | null>;
  getCommits: () => Map<string, DbCommit>;
  commit: (commitDB: unknown) => void;
  branch: (branchDB: { name: string; order: number }) => void;
  merge: (mergeDB: unknown) => void;
  cherryPick: (cherryPickDB: unknown) => void;
}

/** What the probes saw while the parser filled the DB. Reset for every input. */
interface Probes {
  /** Calls of `Math.random` while the parse runs, counted as they happen. */
  randomCalls: number;
  /** Per successful `commit`, `merge` or `cherryPick` call, in call order: whether it drew digits. */
  generated: boolean[];
  /** The `order` of each successful `branch` call, in call order. */
  branchOrders: number[];
}

interface CanonicalCommit {
  id: string;
  seq: number;
  branch: string;
  type: CommitTypeName;
  customType: CommitTypeName | null;
  customId: boolean | null;
  autoId: boolean;
  message: string;
  tags: string[];
  parents: string[];
}

interface CanonicalModel {
  version: number;
  title: string;
  accTitle: string;
  accDescr: string;
  direction: string;
  currentBranch: string;
  branches: { name: string; order: number; head: string | null }[];
  commits: CanonicalCommit[];
}

interface Modules {
  config: {
    setSiteConfig(conf: Record<string, unknown>): unknown;
    reset(): void;
    getConfig(): Record<string, unknown>;
    setDiagramConfigScope(diagramType?: string): void;
    addDirective(directive: Record<string, unknown>): void;
  };
  preprocess: {
    preprocessDiagram(code: string): {
      code: { cleaned: string };
      title?: string;
      config: Record<string, unknown>;
    };
  };
  detectType: {
    detectType(text: string, config?: unknown): string;
  };
  encode: { encodeEntities(text: string): string };
  parser: { parser: { parse(text: string): Promise<void> } };
  db: GitGraphDb;
  probes: Probes;
}

/** Replaces the DB's `commit`, `merge`, `cherryPick` and `branch` by wrappers that record the probes. */
function installProbes(db: GitGraphDb, probes: Probes): void {
  const record = (call: () => void): void => {
    const before = probes.randomCalls;
    call();
    probes.generated.push(probes.randomCalls > before);
  };
  const { commit, merge, cherryPick, branch } = db;
  db.commit = (commitDB) => record(() => commit(commitDB));
  db.merge = (mergeDB) => record(() => merge(mergeDB));
  db.cherryPick = (cherryPickDB) => record(() => cherryPick(cherryPickDB));
  db.branch = (branchDB) => {
    branch(branchDB);
    probes.branchOrders.push(branchDB.order);
  };
}

/** Runs the parse with `Math.random` counted, and restores it afterwards. */
async function parseCounted(parse: () => Promise<void>, probes: Probes): Promise<void> {
  const random = Math.random;
  Math.random = () => {
    probes.randomCalls += 1;
    return random();
  };
  try {
    await parse();
  } finally {
    Math.random = random;
  }
}

async function loadModules(reference: Reference): Promise<Modules> {
  provideDomForSanitising();
  await registerDetectors(reference);
  const [config, preprocess, detectType, encode, parser, ast] = await Promise.all([
    reference.load<Modules['config']>('/packages/mermaid/src/config.ts'),
    reference.load<Modules['preprocess']>('/packages/mermaid/src/preprocess.ts'),
    reference.load<Modules['detectType']>('/packages/mermaid/src/diagram-api/detectType.ts'),
    reference.load<Modules['encode']>('/packages/mermaid/src/utils.ts'),
    reference.load<Modules['parser']>('/packages/mermaid/src/diagrams/git/gitGraphParser.ts'),
    reference.load<{ db: GitGraphDb }>('/packages/mermaid/src/diagrams/git/gitGraphAst.ts'),
  ]);
  const probes: Probes = { randomCalls: 0, generated: [], branchOrders: [] };
  installProbes(ast.db, probes);
  return { config, preprocess, detectType, encode, parser, db: ast.db, probes };
}

function typeName(db: GitGraphDb, value: number): CommitTypeName {
  const name = (Object.keys(db.commitType) as CommitTypeName[]).find(
    (key) => db.commitType[key] === value
  );
  if (name === undefined) throw new Error(`unknown commit type ${value}`);
  return name;
}

/**
 * The named normalisation of the auto ids, the rule of `normalise_auto_ids` in
 * `crates/ferrigura-syntax/examples/canonical.rs`: the commits with `autoId` get `auto#k` in the
 * order of the commits, a repeated id keeps the placeholder of its first occurrence, and every
 * parent and branch head that names such an id gets the placeholder.
 */
function normaliseAutoIds(model: CanonicalModel): CanonicalModel {
  const placeholders = new Map<string, string>();
  for (const commit of model.commits) {
    if (!commit.autoId) continue;
    const placeholder = placeholders.get(commit.id) ?? `auto#${placeholders.size + 1}`;
    placeholders.set(commit.id, placeholder);
    commit.id = placeholder;
  }
  const replace = (id: string): string => placeholders.get(id) ?? id;
  for (const commit of model.commits) {
    commit.parents = commit.parents.map(replace);
  }
  for (const branch of model.branches) {
    if (branch.head !== null) branch.head = replace(branch.head);
  }
  return model;
}

async function convert(input: { text: string }, modules: Modules): Promise<CanonicalModel> {
  const { config, preprocess, detectType, encode, parser, db, probes } = modules;

  // The configuration of the input alone, as a render gives it: the site settings, the reset,
  // the detected scope, then the front matter and directives (`processAndSetConfigs`).
  config.setSiteConfig({});
  config.reset();
  const processed = preprocess.preprocessDiagram(input.text);
  let diagramType: string | undefined;
  try {
    diagramType = detectType.detectType(processed.code.cleaned, config.getConfig());
  } catch {
    diagramType = undefined;
  }
  config.setDiagramConfigScope(diagramType);
  config.addDirective(processed.config);

  probes.generated = [];
  probes.branchOrders = [];
  db.clear();
  if (processed.title) {
    db.setDiagramTitle(processed.title);
  }
  const text = encode.encodeEntities(processed.code.cleaned) + '\n';
  await parseCounted(() => parser.parser.parse(text), probes);

  const mainBranchOrder = db.getConfig().mainBranchOrder;
  const branches = [...db.getBranches()].map(([name, head], index) => {
    const order = index === 0 ? mainBranchOrder : probes.branchOrders[index - 1];
    if (order === undefined) throw new Error(`no branch call for ${name}`);
    return { name, order, head };
  });

  const commits = [...db.getCommits().values()].map((commit): CanonicalCommit => {
    const generated = probes.generated[commit.seq];
    if (generated === undefined) throw new Error(`no call for commit seq ${commit.seq}`);
    return {
      id: commit.id,
      seq: commit.seq,
      branch: commit.branch,
      type: typeName(db, commit.type),
      customType: commit.customType === undefined ? null : typeName(db, commit.customType),
      customId: commit.customId ?? null,
      autoId: generated,
      message: commit.message,
      tags: commit.tags,
      parents: commit.parents,
    };
  });

  return normaliseAutoIds({
    version: 1,
    title: db.getDiagramTitle(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    direction: db.getDirection(),
    currentBranch: db.getCurrentBranch(),
    branches,
    commits,
  });
}

await runExporter(loadModules, convert);
