import { useState } from 'ardo/react';
import {
  MonoCaption,
  MonoFacing,
  MonoSegmented,
  MonoSheet,
  MonoTableView,
} from 'ferramenta-family/monografie';

import samples from '../data/diagram-samples.json';
import { credit, Drawing, Source } from './homepage-specimens';

/* The comparison page's drawings: each sample of the default look beside the same text drawn in
   compat, Mermaid's geometry. Written by scripts/render-diagram-samples.mjs into app/data/. */

const drawings = import.meta.glob<string>('../data/diagrams/design-*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
});
const drawing = (file: string) => {
  const svg = drawings[`../data/diagrams/${file}`];
  if (svg === undefined) throw new Error(`No rendered file ${file}`);
  return svg;
};

/** The same diagram text rendered twice: in compat and by default. */
export function LookComparison() {
  const [first] = samples.design.samples;
  if (first === undefined) throw new Error('No design samples');
  const [selected, setSelected] = useState(first.id);
  const sample = samples.design.samples.find(({ id }) => id === selected) ?? first;
  return (
    <figure
      className={
        sample.stacked
          ? 'mono-wide fgr-pair fgr-design fgr-stacked'
          : 'mono-wide fgr-pair fgr-design'
      }
    >
      <MonoSegmented
        label="Example"
        value={sample.id}
        options={samples.design.samples.map(({ id, label }) => ({ value: id, label }))}
        onChange={setSelected}
      />
      <MonoFacing>
        <MonoSheet label="Mermaid’s geometry" meta="compat: true">
          <Drawing
            svg={drawing(sample.plain)}
            label={`${sample.description} Drawn with Mermaid’s geometry.`}
          />
        </MonoSheet>
        <MonoSheet label="Ferrigura, the default" meta="compat: false">
          <Drawing
            svg={drawing(sample.designed)}
            label={`${sample.description} Drawn in Ferrigura’s own look.`}
          />
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>
        {credit}, the same diagram text rendered twice. Only the option differs.
      </MonoCaption>
      <MonoTableView summary="Show the diagram text">
        <Source label={`${sample.label}: the diagram text for both drawings`}>
          {sample.source}
        </Source>
      </MonoTableView>
    </figure>
  );
}
