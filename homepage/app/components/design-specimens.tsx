import { useState } from 'ardo/react';
import { ArdoLink as Link } from 'ardo/runtime';
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
import firstDesigned from '../data/diagrams/design-pipeline.svg?raw';
import { credit, Drawing, Source } from './homepage-specimens';

/* The drawings of Ferrigura's default look, written by scripts/render-diagram-samples.mjs into
   app/data/ like every other drawing of the page. Mermaid's geometry for the same samples stands on
   the comparison page (look-comparison.tsx), not here. */

// The other samples load when a visitor asks for them; the ones on the page
// from the start are imported above.
const drawings = import.meta.glob<string>(
  [
    '../data/diagrams/design-*.svg',
    '!../data/diagrams/design-*-compat.svg',
    '!../data/diagrams/design-pipeline.svg',
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

/** The chosen sample and its drawing, loaded when it is chosen first. */
function useDesignSample() {
  const [selected, setSelected] = useState(first.id);
  const [loaded, setLoaded] = useState<Record<string, string>>({
    [first.designed]: firstDesigned,
    'design-decision.svg': designLight,
  });
  const sample = samples.design.samples.find(({ id }) => id === selected) ?? first;
  const choose = (id: string) => {
    const next = samples.design.samples.find((entry) => entry.id === id);
    if (next === undefined) return;
    if (loaded[next.designed] !== undefined) {
      setSelected(id);
      return;
    }
    const load = drawings[`../data/diagrams/${next.designed}`];
    if (load === undefined) throw new Error(`No rendered file ${next.designed}`);
    void load().then((drawing) => {
      setLoaded((current) => ({ ...current, [next.designed]: drawing }));
      setSelected(id);
    });
  };
  return { sample, choose, designed: loaded[sample.designed] ?? firstDesigned };
}

/** Three flowcharts, a state diagram and a class diagram in the default rendering. */
export function DesignSheets() {
  const { sample, choose, designed } = useDesignSample();
  return (
    <figure className="mono-wide fgr-pair fgr-design">
      <MonoSegmented
        label="Example"
        value={sample.id}
        options={samples.design.samples.map(({ id, label }) => ({ value: id, label }))}
        onChange={choose}
      />
      <div className="fgr-single fgr-design-sheet">
        <MonoSheet label="Ferrigura, the default" meta={sample.designed}>
          <Drawing svg={designed} label={sample.description} />
        </MonoSheet>
      </div>
      <MonoCaption>
        {credit}, rendered without options. The same samples beside Mermaid&rsquo;s geometry stand
        on the <Link to="/comparisons/default-look/">comparison page</Link>.
      </MonoCaption>
      <MonoTableView summary="Show the diagram text">
        <Source label={`${sample.label}: the diagram text`}>{sample.source}</Source>
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
