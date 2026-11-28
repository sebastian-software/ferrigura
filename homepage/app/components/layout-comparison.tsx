import { ArdoLink as Link } from 'ardo/runtime';
import {
  MonoCaption,
  MonoFacing,
  MonoSection,
  MonoSheet,
  MonoTableView,
} from 'ferramenta-family/monografie';

import comparison from '../data/layout-comparison.json';
import { Source } from './homepage-specimens';

type Sample = (typeof comparison.examples)[number];

const asset = (id: string, suffix: string) => `/layout-comparison/${id}${suffix}`;

function LayoutPair({ sample }: { sample: Sample }) {
  const label = sample.native ? 'TALA in Rust' : 'TALA, pinned Go reference';
  return (
    <figure className="mono-wide fgr-pair fgr-layout-pair">
      <MonoFacing>
        {[
          { engine: 'elk', label: 'ELK Layered', fileLabel: 'ELK', size: sample.elk },
          { engine: 'tala', label, fileLabel: 'TALA', size: sample.tala },
        ].map(({ engine, label: name, fileLabel, size }) => {
          const svg = asset(sample.id, `-${engine}.svg`);
          return (
            <MonoSheet key={engine} label={name}>
              <img
                className="fgr-layout-drawing"
                src={svg}
                alt={`${sample.title}, laid out with ${name}`}
                width={size.width}
                height={size.height}
                loading="lazy"
              />
              <p className="fgr-open">
                <a href={svg}>Open the {fileLabel} SVG</a>
              </p>
            </MonoSheet>
          );
        })}
      </MonoFacing>
      <MonoTableView summary="Show the Mermaid source">
        <Source label={`${sample.title} Mermaid source`}>{sample.source}</Source>
        <p className="fgr-open">
          <a href={asset(sample.id, '.mmd')}>Open the Mermaid source file</a>
        </p>
      </MonoTableView>
      <MonoCaption>
        {sample.title}. {sample.nodes} {sample.nodes === 1 ? 'node' : 'nodes'}, {sample.edges}{' '}
        {sample.edges === 1 ? 'edge' : 'edges'}
        {sample.groups === 0
          ? ''
          : `, ${sample.groups} ${sample.groups === 1 ? 'group' : 'groups'}`}
        . Ferrigura paints both SVGs from the same Mermaid source.
      </MonoCaption>
    </figure>
  );
}

const metrics = [
  ['bends', 'Route bends'],
  ['crossings', 'Route crossings'],
  ['length', 'Total route length'],
  ['nodeOverlaps', 'Node overlaps'],
  ['labelOverlaps', 'Edge-label overlaps'],
  ['resizedNodes', 'Resized nodes'],
] as const;

type ViewBox = { width: number; height: number };

const dimensions = ({ width, height }: ViewBox) => `${width} × ${height}`;
const area = ({ width, height }: ViewBox) => (width * height).toLocaleString('en-US');

function LayoutMetrics({ sample }: { sample: Sample }) {
  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- A scrollable table needs focus.
    <div
      className="fgr-table fgr-layout-metrics"
      role="region"
      aria-label={`${sample.title} layout measurements`}
      tabIndex={0}
    >
      <table className="mono-table">
        <thead>
          <tr>
            <th scope="col">Geometry</th>
            <th scope="col" className="mono-num">
              ELK
            </th>
            <th scope="col" className="mono-num">
              TALA
            </th>
          </tr>
        </thead>
        <tbody>
          {metrics.map(([key, label]) => (
            <tr key={key}>
              <th scope="row">{label}</th>
              <td className="mono-num">{sample.elk[key].toLocaleString('en-US')}</td>
              <td className="mono-num">{sample.tala[key].toLocaleString('en-US')}</td>
            </tr>
          ))}
          <tr>
            <th scope="row">SVG view box</th>
            <td className="mono-num">{dimensions(sample.elk)}</td>
            <td className="mono-num">{dimensions(sample.tala)}</td>
          </tr>
          <tr>
            <th scope="row">Approximate view-box area</th>
            <td className="mono-num">{area(sample.elk)}</td>
            <td className="mono-num">{area(sample.tala)}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

export function LayoutExample({ id }: { id: string }) {
  const sample = comparison.examples.find((example) => example.id === id);
  if (!sample) throw new Error(`No layout comparison for ${id}`);
  return (
    <>
      <LayoutPair sample={sample} />
      <LayoutMetrics sample={sample} />
    </>
  );
}

export function HomepageLayouts() {
  const sample = comparison.examples.find(({ id }) => id === 'library-architecture');
  if (!sample) throw new Error('No grouped layout comparison');
  return (
    <MonoSection
      id="layout-engines"
      title="Two ways to lay out a flowchart."
      standfirst="ELK Layered is Ferrigura’s default. The Go reference for TALA shows another way to place groups and route their links."
    >
      <p>
        In this service map, the TALA reference uses fewer turns and trades width for height. ELK
        keeps the map shallower and draws the directional workflow with shorter connections.
      </p>
      <LayoutPair sample={sample} />
      <p>
        <Link to="/comparisons/elk-and-tala/">Compare ELK and TALA</Link>
        {' · '}
        <Link to="/guide/layout-engines/">Choose a layout engine</Link>
      </p>
    </MonoSection>
  );
}
