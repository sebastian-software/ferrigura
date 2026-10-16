/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Random compound graphs, laid out by Node elkjs: the evidence behind the port's compound layout
 * (`hierarchyHandling: INCLUDE_CHILDREN`) beyond the corpus. The shape follows Mermaid's subgraphs: a
 * root whose children are leaves or compound nodes, nested up to three levels, with labels on the
 * compound nodes and edges between any two nodes, at any level. Every edge is written at the root,
 * as Mermaid writes its edges. Each graph is seeded, so the run is reproducible, and each case is
 * written in the corpus format (`version`, `name`, `input`, `output`, `trace`) to a directory outside
 * the repository's tracked files.
 *
 * The graphs vary the edge direction (all four), the model order strategy, the cycle breaker, the
 * random seed, merged hierarchy edges, edge labels, and inside labels on the compound and leaf nodes.
 *
 * Usage, from the repository root: run the tool with the count, seed and output directory options
 * (defaults: 900 graphs, seed 20261017, target/ferrigura-compound-graphs). Then run the ignored test
 * of the port's crate with the environment variable FERRIGURA_COMPOUND_GRAPHS set to that directory,
 * and the --ignored and --show-output options of cargo test.
 */

import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { CORPUS_VERSION, jsonRoundTrip, stripGwtHashKeys } from './corpus-model.js';

const REPO_ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)), '../../..');
const mermaidRequire = createRequire(join(REPO_ROOT, 'packages/mermaid/package.json'));
const ELK = mermaidRequire('elkjs/lib/elk.bundled.js') as new () => {
  layout(graph: unknown, options?: { logging?: boolean }): Promise<Record<string, unknown>>;
};

const DIRECTIONS = ['DOWN', 'RIGHT', 'LEFT', 'UP'];
const MODEL_ORDERS = ['NONE', 'NODES_AND_EDGES', 'PREFER_EDGES', 'PREFER_NODES'];
const CYCLE_BREAKERS = ['DEPTH_FIRST', 'GREEDY'];
const LABEL_PLACEMENTS = [
  '[H_CENTER V_TOP, INSIDE]',
  '[H_LEFT V_TOP, INSIDE]',
  '[H_RIGHT V_BOTTOM, INSIDE]',
  '[H_CENTER V_CENTER, INSIDE]',
];
/** The deepest compound level: the root is level 0, its compound children level 1, and so on. */
const MAX_DEPTH = 3;

/** A small deterministic generator (mulberry32), so one seed gives one set of graphs. */
function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

interface Built {
  graph: Record<string, unknown>;
  /** Every node below the root, leaves and compound nodes, with whether it is compound. */
  nodes: { id: string; compound: boolean }[];
}

function makeGraph(random: () => number): Built {
  const between = (low: number, high: number): number => low + Math.floor(random() * (high - low));
  const pick = <T>(values: T[]): T => values[Math.floor(random() * values.length)];
  const nodes: { id: string; compound: boolean }[] = [];
  let counter = 0;

  const withLabel = (node: Record<string, unknown>, text: string, placement: string): void => {
    node.labels = [{ text, width: between(10, 50), height: between(10, 20) }];
    node.layoutOptions = { 'elk.nodeLabels.placement': placement };
  };

  const build = (depth: number): Record<string, unknown>[] => {
    const count = depth === 0 ? between(2, 6) : between(1, 5);
    return Array.from({ length: count }, () => {
      const id = `g${counter++}`;
      const compound = depth < MAX_DEPTH && random() < 0.4;
      nodes.push({ id, compound });
      if (compound) {
        const node: Record<string, unknown> = { id, children: build(depth + 1) };
        if (random() < 0.7) {
          withLabel(node, `Group ${id}`, pick(LABEL_PLACEMENTS));
        }
        return node;
      }
      const node: Record<string, unknown> = {
        id,
        width: between(20, 80),
        height: between(20, 60),
      };
      if (random() < 0.3) {
        withLabel(node, `Leaf ${id}`, pick(LABEL_PLACEMENTS));
      }
      return node;
    });
  };

  const children = build(0);
  const ends = nodes.map((node) => node.id);
  const edgeCount = Math.max(1, Math.floor(nodes.length / 2) + between(0, nodes.length));
  const edges: Record<string, unknown>[] = [];
  for (let index = 0; index < edgeCount; index++) {
    const source = pick(ends);
    let target = pick(ends);
    if (target === source) {
      target = ends[(ends.indexOf(target) + 1) % ends.length];
    }
    const edge: Record<string, unknown> = { id: `e${index}`, sources: [source], targets: [target] };
    if (random() < 0.25) {
      edge.labels = [{ text: `L${index}`, width: between(10, 40), height: between(10, 20) }];
    }
    edges.push(edge);
  }

  const graph = {
    id: 'root',
    layoutOptions: {
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.algorithm': 'elk.layered',
      'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
      'elk.layered.nodePlacement.bk.fixedAlignment': 'BALANCED',
      'elk.direction': pick(DIRECTIONS),
      'spacing.baseValue': 40,
      'elk.layered.unnecessaryBendpoints': true,
      'elk.layered.cycleBreaking.strategy': pick(CYCLE_BREAKERS),
      'elk.layered.layering.strategy': 'NETWORK_SIMPLEX',
      'elk.layered.mergeEdges': false,
      'elk.layered.mergeHierarchyEdges': random() < 0.5,
      'elk.randomSeed': between(1, 1000),
      'elk.layered.considerModelOrder.strategy': pick(MODEL_ORDERS),
      'elk.spacing.portsSurrounding': '[top=12,left=12,bottom=12,right=12]',
    },
    children,
    edges,
  };
  return { graph, nodes };
}

interface Options {
  count: number;
  seed: number;
  out: string;
}

function parseOptions(argv: string[]): Options {
  const options: Options = {
    count: 900,
    seed: 20261017,
    out: join(REPO_ROOT, 'target/ferrigura-compound-graphs'),
  };
  for (let index = 0; index < argv.length; index += 2) {
    const value = argv[index + 1];
    if (value === undefined) {
      throw new Error(`${argv[index]} needs a value`);
    }
    switch (argv[index]) {
      case '--count':
        options.count = Number(value);
        break;
      case '--seed':
        options.seed = Number(value);
        break;
      case '--out':
        options.out = resolve(REPO_ROOT, value);
        break;
      default:
        throw new Error(`unknown option ${argv[index]}`);
    }
  }
  return options;
}

async function main(): Promise<void> {
  const options = parseOptions(process.argv.slice(2));
  const random = seededRandom(options.seed);
  mkdirSync(options.out, { recursive: true });
  const names: string[] = [];
  const failures: string[] = [];
  for (let index = 0; index < options.count; index++) {
    const name = `compound__${String(index).padStart(4, '0')}`;
    const { graph: input } = makeGraph(random);
    try {
      // elkjs writes into the graph it is given, so it gets a copy.
      const result = await new ELK().layout(jsonRoundTrip(input), { logging: true });
      const { logging, ...laidOut } = result;
      const record = {
        version: CORPUS_VERSION,
        name,
        fixture: null,
        algorithm: 'elk.layered',
        input,
        output: stripGwtHashKeys(jsonRoundTrip(laidOut)),
        trace: stripGwtHashKeys(logging),
      };
      writeFileSync(join(options.out, `${name}.json`), `${JSON.stringify(record, null, 1)}\n`);
      names.push(name);
    } catch (error) {
      failures.push(`${name}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  writeFileSync(join(options.out, 'names.json'), `${JSON.stringify(names, null, 1)}\n`);
  console.log(`wrote ${names.length} of ${options.count} compound graphs to ${options.out}`);
  for (const failure of failures) {
    console.log(`elkjs failed on ${failure}`);
  }
}

await main();
