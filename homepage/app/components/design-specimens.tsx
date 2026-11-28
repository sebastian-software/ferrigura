import { useState } from 'ardo/react';
import {
  MonoCaption,
  MonoFacing,
  MonoSegmented,
  MonoSheet,
  MonoTableView,
} from 'ferramenta-family/monografie';

import samples from '../data/diagram-samples.json';
import designDark from '../data/diagrams/design-decision-dark.svg?raw';
import designLight from '../data/diagrams/design-decision.svg?raw';
import firstPlain from '../data/diagrams/design-pipeline-compat.svg?raw';
import firstDesigned from '../data/diagrams/design-pipeline.svg?raw';
import { credit, Drawing, Source } from './homepage-specimens';

/* The drawings of Ferrigura's own look beside compat's, written by scripts/render-diagram-samples.mjs
   into app/data/ like every other drawing of the page. */

// The other samples load when a visitor asks for them; the ones on the page
// from the start are imported above.
const drawings = import.meta.glob<string>(
  [
    '../data/diagrams/design-*.svg',
    '!../data/diagrams/design-pipeline*.svg',
    '!../data/diagrams/design-decision.svg',
    '!../data/diagrams/design-decision-dark.svg',
  ],
  { query: '?raw', import: 'default' }
);

/** The first sample, the one the page is prerendered with. */
function firstSample() {
  const [entry] = samples.design.samples;
  if (entry?.designed !== 'design-pipeline.svg') {
    throw new Error('The design samples start with the pipeline');
  }
  return entry;
}
const first = firstSample();

/** The chosen sample and its two drawings, loaded when it is chosen first. */
function useDesignSample() {
  const [selected, setSelected] = useState(first.id);
  const [loaded, setLoaded] = useState<Record<string, string>>({
    [first.plain]: firstPlain,
    [first.designed]: firstDesigned,
    'design-decision.svg': designLight,
  });
  const sample = samples.design.samples.find(({ id }) => id === selected) ?? first;
  const choose = (id: string) => {
    const next = samples.design.samples.find((entry) => entry.id === id);
    if (next === undefined) return;
    const missing = [next.plain, next.designed].filter((file) => loaded[file] === undefined);
    const loads = missing.map(async (file) => {
      const load = drawings[`../data/diagrams/${file}`];
      if (load === undefined) throw new Error(`No rendered file ${file}`);
      return [file, await load()] as const;
    });
    void Promise.all(loads).then((entries) => {
      setLoaded((current) => ({ ...current, ...Object.fromEntries(entries) }));
      setSelected(id);
    });
  };
  return {
    sample,
    choose,
    plain: loaded[sample.plain] ?? firstPlain,
    designed: loaded[sample.designed] ?? firstDesigned,
  };
}

/**
     Three flowcharts, a state diagram and a class diagram, each in compat, Mermaid's drawing, and in
    the default rendering.
 */
export function DesignSheets() {
  const { sample, choose, plain, designed } = useDesignSample();
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
        onChange={choose}
      />
      <MonoFacing>
        <MonoSheet label="Mermaid’s drawing" meta="compat: true">
          <Drawing svg={plain} label={`${sample.description} Drawn with Mermaid’s geometry.`} />
        </MonoSheet>
        <MonoSheet label="Ferrigura, the default" meta="compat: false">
          <Drawing svg={designed} label={`${sample.description} Drawn in Ferrigura’s own look.`} />
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

/** The decision sample of the default rendering in its light and its dark set. */
export function DesignSchemeSheets() {
  const sample = samples.design.samples.find(({ id }) => id === 'decision') ?? first;
  return (
    <figure className="mono-wide fgr-pair">
      <MonoFacing>
        <MonoSheet label="Light" meta="colorScheme: 'light'">
          <Drawing svg={designLight} label={`${sample.description} Light colors.`} />
        </MonoSheet>
        <MonoSheet label="Dark" meta="colorScheme: 'dark'" className="fgr-dark">
          <Drawing svg={designDark} label={`${sample.description} Dark colors.`} />
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>
        {credit}. Each set is checked for contrast: 4.5 to 1 for text, 3 to 1 for lines.
      </MonoCaption>
    </figure>
  );
}
