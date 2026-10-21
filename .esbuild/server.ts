import chokidar from 'chokidar';
import cors from 'cors';
import { createHash } from 'crypto';
import { context } from 'esbuild';
import type { Request, Response } from 'express';
import express from 'express';
import path from 'path';
import { packageOptions } from '../.build/common.js';
import { generateLangium } from '../.build/generateLangium.js';
import { defaultOptions, getBuildConfig } from './util.js';
import 'dotenv/config';

// Derive a deterministic dev-server port from the worktree root so multiple
// checkouts (git worktrees, sibling clones) can run `pnpm dev` in parallel
// without colliding on port 9000. Override with MERMAID_DEV_PORT.
function resolveDevPort(): number {
  const override = process.env.MERMAID_DEV_PORT;
  if (override) {
    const parsed = Number.parseInt(override, 10);
    if (Number.isFinite(parsed) && parsed > 0 && parsed < 65536) {
      return parsed;
    }
    console.warn(`[dev] Ignoring invalid MERMAID_DEV_PORT=${override}`);
  }
  const worktreeRoot = process.cwd();
  // The canonical checkout keeps port 9000 so existing tooling/bookmarks work.
  if (path.basename(worktreeRoot) === 'alana-mermaid') {
    return 9000;
  }
  const hash = createHash('sha1').update(worktreeRoot).digest();
  return 9000 + (hash.readUInt16BE(0) % 100);
}

const devPort = resolveDevPort();

// Print the URL up-front, before the build noise, so the operator knows where
// to point their browser even if they scroll past the later "Listening on…"
// confirmation line.
console.log('');
console.log('================================================================');
console.log(
  `  Dev server starting on http://localhost:${devPort}` +
    `  (worktree: ${path.basename(process.cwd())})`
);
console.log('================================================================');
console.log('');

// The dev server is never published, so always compile the render profiler in.
// It stays a no-op until toggled on at runtime: `__mermaidProfiler.enable()`.
const configs = Object.values(packageOptions).map(({ packageName }) =>
  getBuildConfig({
    ...defaultOptions,
    minify: false,
    core: false,
    profiling: true,
    options: packageOptions[packageName],
  })
);
const mermaidIIFEConfig = getBuildConfig({
  ...defaultOptions,
  minify: false,
  core: false,
  profiling: true,
  options: packageOptions.mermaid,
  format: 'iife',
});
configs.push(mermaidIIFEConfig);

const contexts = await Promise.all(
  configs.map(async (config) => ({ config, context: await context(config) }))
);

let rebuildCounter = 1;
const rebuildAll = async () => {
  const buildNumber = rebuildCounter++;
  const timeLabel = `Rebuild ${buildNumber} Time (total)`;
  console.time(timeLabel);

  await Promise.all(
    contexts.map(async ({ config, context }) => {
      const buildVariant = `Rebuild ${buildNumber} Time (${Object.keys(config.entryPoints!)[0]} ${config.format})`;
      console.time(buildVariant);
      await context.rebuild();
      console.timeEnd(buildVariant);
    })
  ).catch((e) => console.error(e));
  console.timeEnd(timeLabel);
};

let clients: { id: number; response: Response }[] = [];
function eventsHandler(request: Request, response: Response) {
  const headers = {
    'Content-Type': 'text/event-stream',
    Connection: 'keep-alive',
    'Cache-Control': 'no-cache',
  };
  response.writeHead(200, headers);
  const clientId = Date.now();
  clients.push({
    id: clientId,
    response,
  });
  request.on('close', () => {
    clients = clients.filter((client) => client.id !== clientId);
  });
}

let timeoutID: NodeJS.Timeout | undefined = undefined;

/**
 * Debounce file change events to avoid rebuilding multiple times.
 */
function handleFileChange() {
  if (timeoutID !== undefined) {
    clearTimeout(timeoutID);
  }
  // eslint-disable-next-line @typescript-eslint/no-misused-promises
  timeoutID = setTimeout(async () => {
    await rebuildAll();
    sendEventsToAll();
    timeoutID = undefined;
  }, 100);
}

function sendEventsToAll() {
  clients.forEach(({ response }) => response.write(`data: ${Date.now()}\n\n`));
}

async function createServer() {
  await generateLangium();
  handleFileChange();
  const app = express();
  chokidar
    .watch('**/src/**/*.{js,ts,langium,yaml,json}', {
      ignoreInitial: true,
      ignored: [/node_modules/, /dist/, /docs/, /coverage/],
    })
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    .on('all', async (event, path) => {
      // Ignore other events.
      if (!['add', 'change'].includes(event)) {
        return;
      }
      console.log(`${path} changed. Rebuilding...`);
      if (path.endsWith('.langium')) {
        await generateLangium();
      }
      handleFileChange();
    });

  app.use(cors());
  app.get('/events', eventsHandler);

  for (const { packageName } of Object.values(packageOptions)) {
    app.use(express.static(`./packages/${packageName}/dist`));
  }
  app.use(express.static('e2e/platform'));

  app.listen(devPort, () => {
    console.log(
      `Listening on http://localhost:${devPort} (worktree: ${path.basename(process.cwd())})`
    );
  });
}

void createServer();
