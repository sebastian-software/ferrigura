import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { Renderer } from '../../node/ferrigura/index.mjs';

const root = new URL('./', import.meta.url);
const data = new URL('../../homepage/app/data/', root);
const manifest = JSON.parse(await readFile(new URL('diagram-samples.json', data), 'utf8'));
const request = manifest.samples.find((sample) => sample.id === 'flowchart');
const renderer = new Renderer();
const samples = [
  {
    id: 'request',
    title: 'A request takes two paths',
    description: request.description,
    source: request.source,
  },
  {
    id: 'review',
    title: 'A review can return to the start',
    description: manifest.modes.description,
    source: manifest.modes.source,
  },
  {
    id: 'styles',
    title: 'Line styles stay intact',
    description:
      'A bidirectional connection, dotted return, colored line, and linked node inside a group.',
    source:
      '---\nconfig:\n  look: classic\n---\nflowchart LR\n subgraph Work\n A[Start] edge@-->|go| B{Review}\n B -.-> A\n end\n B <--> C[Done]\n click A href "https://example.com"\n linkStyle 0 stroke:#f00,stroke-width:4px\n',
  },
  {
    id: 'gantt',
    title: 'A release takes shape',
    description:
      'Tasks grow along the timeline. Milestones arrive after research, design, and delivery.',
    source:
      'gantt\n title Release plan\n dateFormat YYYY-MM-DD\n axisFormat %b %d\n todayMarker off\n section Discovery\n Research :done, research, 2026-01-05, 3d\n Design :active, design, after research, 4d\n section Delivery\n Build :crit, build, after design, 5d\n Launch :milestone, launch, after build, 0d\n',
  },
  {
    id: 'gitgraph',
    title: 'Commits, branches, and a merge',
    description: 'Connections trace the commit history; commit markers and tags appear on arrival.',
    source:
      'gitGraph\n commit id: "Start" tag: "v1"\n branch docs\n commit id: "Guide"\n commit id: "Examples"\n checkout main\n commit id: "Fix" type: HIGHLIGHT\n merge docs id: "Merge"\n commit id: "Ship" tag: "v2"\n',
  },
  {
    id: 'mindmap',
    title: 'An idea branches outward',
    description:
      'A release starts at its root and grows through research, implementation, and delivery.',
    source:
      '---\nconfig:\n  layout: tidy-tree\n---\nmindmap\n root((Release))\n  Research\n   Interviews\n   Prototypes\n  Build\n   Parser\n   Layout\n   SVG\n  Ship\n   Tests\n   Documentation\n',
  },
  {
    id: 'pie',
    title: 'A circle, one sector at a time',
    description:
      'Each sector draws clockwise with its own soft start and finish, paced by its arc length.',
    source:
      'pie showData\n title Rendering work\n "Layout" : 45\n "Text" : 30\n "SVG" : 20\n "Other" : 5\n',
  },
  {
    id: 'donut',
    title: 'The same reveal with a center hole',
    description: 'Each sector draws around the ring and brings in its matching legend entry.',
    source:
      '---\nconfig:\n  pie:\n    donutHole: 0.55\n---\npie showData\n title Build time\n "Compile" : 60\n "Link" : 25\n "Package" : 15\n',
  },
  {
    id: 'xychart',
    title: 'Bars rise, then a line traces the trend',
    description:
      'Each bar grows from its baseline. The line follows its points while the axes stay visible.',
    source:
      'xychart-beta\n title "Weekly output"\n x-axis [Mon, Tue, Wed, Thu, Fri]\n y-axis "Builds" 0 --> 100\n bar "Builds" [32, 54, 46, 78, 92]\n line "Trend" [28, 42, 55, 70, 88]\n',
  },
  {
    id: 'xy-horizontal',
    title: 'Horizontal bars grow along the value axis',
    description:
      'The reveal follows the chart orientation, keeping category labels and the scale in place.',
    source:
      'xychart-beta horizontal\n title "Work in progress"\n x-axis [Planned, Active, Done]\n y-axis "Tasks" 0 --> 50\n bar "Tasks" [40, 26, 18]\n',
  },
].map((sample) => ({
  ...sample,
  svg: renderer.render(sample.source, { id: `animation-${sample.id}` }),
}));

const files = new Map([
  ['/', ['index.html', 'text/html']],
  ['/style.css', ['style.css', 'text/css']],
  ['/app.mjs', ['app.mjs', 'text/javascript']],
]);
const requestedPort = Number(process.argv[2] ?? 4174);
if (!Number.isInteger(requestedPort) || requestedPort < 0 || requestedPort > 65_535) {
  throw new Error('Pass a TCP port between 0 and 65535.');
}
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost');
    res.setHeader('Cache-Control', 'no-store');
    if (url.pathname === '/samples.json') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ samples }));
    } else if (url.pathname === '/render') {
      const sample = samples.find((sample) => sample.id === url.searchParams.get('sample'));
      if (!sample) {
        res.writeHead(400);
        res.end('Unknown diagram.');
        return;
      }
      try {
        const svg = renderer.render(sample.source, {
          id: `animation-${sample.id}`,
          animation: {
            speed: Number(url.searchParams.get('speed') ?? 1),
            order: url.searchParams.get('order') ?? 'source',
          },
        });
        res.writeHead(200, { 'Content-Type': 'image/svg+xml' });
        res.end(svg);
      } catch (error) {
        res.writeHead(400, { 'Content-Type': 'text/plain' });
        res.end(error.message);
      }
    } else if (files.has(url.pathname)) {
      const [file, type] = files.get(url.pathname);
      res.writeHead(200, { 'Content-Type': `${type}; charset=utf-8` });
      res.end(await readFile(new URL(file, root)));
    } else {
      res.writeHead(url.pathname === '/favicon.ico' ? 204 : 404);
      res.end();
    }
  } catch (error) {
    console.error(error);
    res.writeHead(500);
    res.end('The animation preview could not be loaded.');
  }
});
server.listen(requestedPort, '127.0.0.1', () => {
  console.log(`Animation preview: http://127.0.0.1:${server.address().port}`);
});
