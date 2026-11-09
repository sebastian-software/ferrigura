import charts from '../data/size-charts.json';
import size from '../data/size.json';

/* The numbers come from tools/size/measure.mjs (app/data/size.json); the
   charts are Mermaid text drawn by the published package, written by
   scripts/render-size.mjs. Nothing here is typed by hand. */

const fileUrls = import.meta.glob<string>('../data/size/*.svg', {
  query: '?url',
  import: 'default',
  eager: true,
});

const markup = import.meta.glob<string>('../data/size/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
});

const mb = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;

function chart(id: string) {
  const entry = charts.charts.find((candidate) => candidate.id === id);
  if (entry === undefined) throw new Error(`No size chart ${id}`);
  const url = fileUrls[`../data/size/${entry.file}`];
  if (url === undefined) throw new Error(`No rendered file ${entry.file}`);
  return { ...entry, url };
}

/** The drawing's size from its viewBox, so the image keeps its proportions. */
function drawingSize(file: string) {
  const svg = markup[`../data/size/${file}`] ?? '';
  const viewBox = /viewBox="([^"]+)"/.exec(svg)?.[1]?.split(/\s+/).map(Number);
  if (viewBox?.length !== 4) throw new Error(`The size chart ${file} has no viewBox`);
  return { width: Math.round(viewBox[2] ?? 0), height: Math.round(viewBox[3] ?? 0) };
}

/** One chart: the drawing, what it shows, and the Mermaid text it was drawn from. */
export function SizeChart({ id }: { id: string }) {
  const entry = chart(id);
  const { width, height } = drawingSize(entry.file);
  return (
    <figure className="fgr-size">
      <figcaption>{entry.description}</figcaption>
      <img
        className="fgr-size-drawing"
        src={entry.url}
        width={width}
        height={height}
        loading="lazy"
        alt={`${entry.title}: ${entry.description}`}
      />
      <details className="fgr-side-source">
        <summary>Diagram text</summary>
        {/* oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- A scrollable region needs focus. */}
        <pre tabIndex={0} aria-label={`${entry.title}: diagram text`}>
          <code>{entry.source.replace(/\n$/, '')}</code>
        </pre>
      </details>
    </figure>
  );
}

/** The artifacts as measured: the command-line tool and the addon builds. */
export function SizeTable() {
  const rows = [
    { label: 'Command-line tool, every type', ...size.cli },
    { label: 'Addon, core only (the info diagram)', ...size.core },
    ...size.sets.map(({ label, bytes, gzip }) => ({
      label: `Addon, ${label.toLowerCase()}`,
      bytes,
      gzip,
    })),
  ];
  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- A scrollable region needs focus.
    <div className="mono-table-region" role="region" tabIndex={0} aria-label="Sizes of the builds">
      <table>
        <thead>
          <tr>
            <th>Build</th>
            <th>Size</th>
            <th>gzip</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ label, bytes, gzip }) => (
            <tr key={label}>
              <td>{label}</td>
              <td>{mb(bytes)}</td>
              <td>{mb(gzip)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Where and with what the sizes were measured, and which release drew the charts. */
export function Measured() {
  const { commit, rustc, platform } = size.measuredWith;
  return (
    <>
      commit <code>{commit}</code>, {rustc.replace(/^rustc /, 'Rust ')} on <code>{platform}</code>,
      charts drawn by <code>@ferrigura/core</code> {charts.renderedWith['@ferrigura/core']}
    </>
  );
}

/** One measured number, by name, for the prose. */
export function Size({ of }: { of: 'all' | 'cli' | 'core' | 'sequence' }) {
  const bytes = {
    core: size.core.bytes,
    all: size.sets.at(-1)?.bytes ?? 0,
    sequence: size.sets[0]?.bytes ?? 0,
    cli: size.cli.bytes,
  }[of];
  return <>{mb(bytes)}</>;
}
