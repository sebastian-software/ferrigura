// Serves the kerning lab and stores the results the page posts.
//
// Usage: node spikes/text-measure/lab/server.mjs [port] [results-dir]. Open
// http://localhost:<port>/ in any browser; each run is written to <results-dir>/<browser>.json.
import { createServer } from 'node:http';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const port = Number(process.argv[2] ?? 8765);
const spike = join(dirname(fileURLToPath(import.meta.url)), '..');
const results = process.argv[3] ?? join(spike, 'lab', 'results');
mkdirSync(results, { recursive: true });

const types = { '.html': 'text/html', '.ttf': 'font/ttf', '.svg': 'image/svg+xml' };
const files = { '/': 'lab/lab.html', '/lab.html': 'lab/lab.html' };

const browserName = (ua) =>
  /Firefox\/([\d.]+)/.exec(ua)?.[0].replace('/', '-') ??
  /Edg\/([\d.]+)/.exec(ua)?.[0].replace('/', '-') ??
  /Chrome\/([\d.]+)/.exec(ua)?.[0].replace('/', '-') ??
  /Version\/([\d.]+).*Safari/.exec(ua)?.[0].split(' ')[0].replace('Version/', 'Safari-') ??
  'unknown';

createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${port}`);
  if (req.method === 'POST' && url.pathname === '/result') {
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', () => {
      const result = JSON.parse(body);
      const name = `${browserName(result.userAgent)}${url.searchParams.get('tag') ?? ''}`;
      writeFileSync(join(results, `${name}.json`), JSON.stringify(result, null, 1));
      process.stdout.write(`stored ${name}\n`);
      res.end('ok');
    });
    return;
  }
  const path = files[url.pathname] ?? normalize(`.${url.pathname}`);
  if (!path.startsWith('fonts/') && !Object.values(files).includes(path)) {
    res.writeHead(404).end();
    return;
  }
  try {
    res.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' });
    res.end(readFileSync(join(spike, path)));
  } catch {
    res.writeHead(404).end();
  }
}).listen(port, () => process.stdout.write(`lab on http://localhost:${port}/lab.html\n`));
