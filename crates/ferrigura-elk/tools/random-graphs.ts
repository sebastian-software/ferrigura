/* eslint-disable no-console -- CLI tool: progress and results go to the console */
/**
 * Random flat graphs, laid out by Node elkjs: the evidence behind the port's claim that it matches
 * elkjs beyond the corpus. Each graph is seeded, so the run is reproducible, and each case is written
 * in the corpus format (`version`, `name`, `input`, `output`, `trace`) to a directory outside the
 * repository's tracked files.
 *
 * The graphs vary the edge direction (DOWN, RIGHT, LEFT, UP), the model order strategy, the cycle
 * breaker (DEPTH_FIRST or GREEDY), the random seed, and edge labels. Parallel edges, cycles and
 * self-loops are possible; the node count is between 4 and 18.
 *
 * Usage, from the repository root: run the tool with the count, seed and output directory options
 * (defaults: 1200 graphs, seed 20261016, target/ferrigura-random-graphs). Then run the ignored test
 * of the port's crate with the environment variable FERRIGURA_RANDOM_GRAPHS set to that directory,
 * and the --ignored and --show-output options of cargo test.
 */

import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { CORPUS_VERSION, jsonRoundTrip, stripGwtHashKeys, type Json } from './corpus-model.js';

const REPO_ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)), '../../..');
const mermaidRequire = createRequire(join(REPO_ROOT, 'packages/mermaid/package.json'));
const ELK = mermaidRequire('elkjs/lib/elk.bundled.js') as new () => {
  layout(graph: unknown, options?: { logging?: boolean }): Promise<Record<string, unknown>>;
};

const DIRECTIONS = ['DOWN', 'RIGHT', 'LEFT', 'UP'];
const MODEL_ORDERS = ['NONE', 'NODES_AND_EDGES', 'PREFER_EDGES', 'PREFER_NODES'];
const CYCLE_BREAKERS = ['DEPTH_FIRST', 'GREEDY'];

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

function makeGraph(random: () => number): Json {
  const between = (low: number, high: number): number => low + Math.floor(random() * (high - low));
  const nodeCount = between(4, 19);
  const edgeCount = nodeCount + Math.floor(random() * nodeCount * 2);
  const pick = (values: string[]): string => values[Math.floor(random() * values.length)];
  const withLabels = random() < 0.5;
  const pairs: [string, string][] = [];
  for (let index = 0; index < edgeCount; index++) {
    const from = Math.floor(random() * nodeCount);
    let to = Math.floor(random() * nodeCount);
    // Self-loops are rare; a node with two of them is a hyper loop, which the port leaves unsupported.
    if (from === to && random() >= 0.05) {
      to = (to + 1) % nodeCount;
    }
    // Mostly forward, so most graphs have few cycles, and some do not.
    pairs.push(
      random() < 0.85
        ? [`n${Math.min(from, to)}`, `n${Math.max(from, to)}`]
        : [`n${Math.max(from, to)}`, `n${Math.min(from, to)}`]
    );
  }
  return {
    id: 'root',
    layoutOptions: {
      'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
      'elk.algorithm': 'elk.layered',
      'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
      'elk.layered.nodePlacement.bk.fixedAlignment': 'BALANCED',
      'elk.direction': pick(DIRECTIONS),
      'spacing.baseValue': 40,
      'elk.layered.unnecessaryBendpoints': true,
      'elk.layered.cycleBreaking.strategy': pick(CYCLE_BREAKERS),
      'elk.layered.layering.strategy': 'NETWORK_SIMPLEX',
      'elk.layered.mergeEdges': false,
      'elk.randomSeed': between(1, 1000),
      'elk.layered.considerModelOrder.strategy': pick(MODEL_ORDERS),
    },
    children: Array.from({ length: nodeCount }, (_, index) => ({
      id: `n${index}`,
      width: between(20, 80),
      height: between(20, 60),
    })),
    edges: pairs.map(([source, target], index) => ({
      id: `e${index}`,
      sources: [source],
      targets: [target],
      ...(withLabels && random() < 0.3
        ? {
            labels: [
              {
                id: `l${index}`,
                text: `L${index}`,
                width: between(10, 40),
                height: between(10, 20),
              },
            ],
          }
        : {}),
    })),
  };
}

interface Options {
  count: number;
  seed: number;
  out: string;
}

function parseOptions(argv: string[]): Options {
  const options: Options = {
    count: 1200,
    seed: 20261016,
    out: join(REPO_ROOT, 'target/ferrigura-random-graphs'),
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
    const name = `random__${String(index).padStart(4, '0')}`;
    const input = makeGraph(random);
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
  console.log(`wrote ${names.length} of ${options.count} random graphs to ${options.out}`);
  for (const failure of failures) {
    console.log(`elkjs failed on ${failure}`);
  }
}

await main();
