import { ArdoLink as Link } from 'ardo/runtime';
import {
  MonoCard,
  MonoCards,
  MonoPassage,
  MonoSection,
  MonoVerdict,
  MonoWide,
} from 'ferramenta-family/monografie';

import coverage from '../data/coverage.json';
import { grouped } from './homepage-data';
import { Size } from './size';

/* The homepage's second chapter: the ten diagram types Ferrigura draws, and why these. */

/* How often each diagram type is written: the Mermaid blocks with a known header in a sample of
   1,000 Markdown files from 705 public repositories on GitHub. */
const usage = { diagrams: 1284, ofTheTen: 1262, flowcharts: 945 };

/** The ten diagram types in the three groups of decision 0024. */
const groups = [
  {
    title: 'Graphs',
    types: 'flowchart · state · class · ER',
    body: 'Boxes and arrows, placed by one layout engine. These are the large diagrams, where the layout decides whether a reader can follow the structure.',
    verdict: 'Flowcharts, state diagrams and class diagrams also draw in the design mode.',
  },
  {
    title: 'A form of their own',
    types: 'sequence · gantt · mindmap',
    body: 'Lifelines and messages, bars on a time axis, a tree around its center. Each has a layout made for it alone.',
    verdict: 'After the flowchart, the sequence diagram is the type people write most.',
  },
  {
    title: 'Charts',
    types: 'XY chart · pie · quadrant chart',
    body: 'Numbers on axes, in slices and in four fields: the figure a page needs beside its text.',
    verdict: 'Drawn with the same fonts and colors as the diagrams around them.',
  },
];

/** Which diagram types Ferrigura draws, and why it draws these and no others. */
export function Focus() {
  const share = Math.round((100 * usage.ofTheTen) / usage.diagrams);
  return (
    <MonoSection
      id="types"
      title="Ten diagram types, each one finished."
      standfirst="Mermaid has more than thirty diagram types. Ferrigura draws the ten that nearly every diagram is written in, and draws each of them completely."
    >
      <MonoPassage
        note={{
          figure: `${share} of 100`,
          children: `Mermaid diagrams in public repositories are of the ten types Ferrigura draws, in a sample of ${grouped(usage.diagrams)}`,
        }}
      >
        <p>
          Mermaid began with the flowchart and has kept growing. It now knows more than thirty
          diagram types, some of them everywhere and some still marked beta. What people write is
          far less spread out. In a sample of {grouped(usage.diagrams)} Mermaid diagrams from public
          repositories on GitHub, {grouped(usage.flowcharts)} are flowcharts, and nine more types
          cover nearly all of the rest.
        </p>
      </MonoPassage>
      <p>Ferrigura draws those ten. They come in three groups.</p>
      <MonoWide className="fgr-gap">
        <MonoCards>
          {groups.map((group) => (
            <MonoCard key={group.title} title={group.title} api={group.types}>
              <p className="fgr-card-text">{group.body}</p>
              <MonoVerdict>{group.verdict}</MonoVerdict>
            </MonoCard>
          ))}
        </MonoCards>
      </MonoWide>
      <p>
        A type Ferrigura draws is ported whole and checked whole: what it reads and what it draws
        are compared with Mermaid {coverage.mermaid}, input by input. No type is half supported. A
        text of another type is not drawn with parts missing; it gets an error that names the type
        and lists the ten.
      </p>
      <p>
        Ten types instead of thirty is what makes room for the rest of this page: labels that are
        measured, drawings that are checked to a twentieth of a pixel, and a design mode that goes
        beyond Mermaid&rsquo;s own drawing. It keeps the engine small, too: the Node addon with all
        ten types is <Size of="all" />. <Link to="/architecture/size/">What each type adds</Link>
        {' · '}
        <Link to="/architecture/decisions/0024-a-fixed-list-of-diagram-types/">Why ten types</Link>
      </p>
    </MonoSection>
  );
}
