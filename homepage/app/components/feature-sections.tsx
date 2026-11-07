import { ArdoLink as Link } from 'ardo/runtime';
import {
  MonoCard,
  MonoCards,
  MonoLeadIn,
  MonoPassage,
  MonoSection,
  MonoTerms,
  MonoVerdict,
  MonoWide,
} from 'ferramenta-family/monografie';
import { displayName, family, STATUS_MEANING } from 'ferramenta-family/registry';

import coverage from '../data/coverage.json';
import { grouped, tool } from './homepage-data';
import { CoverageProofs, StartCards } from './homepage-specimens';

/* The homepage's later chapters: the checks against Mermaid, the alternatives,
   the limits, the three ways in and the family. */

export function Checked() {
  return (
    <MonoSection
      id="checked"
      title="Checked against Mermaid itself."
      standfirst={`Every diagram type is compared with Mermaid ${coverage.mermaid}, run unchanged in a browser: first what it reads, then what it draws.`}
    >
      <MonoPassage
        note={{
          term: 'Reference',
          children: `Mermaid ${coverage.mermaid}, built from a pinned commit and run in Chromium, the browser engine of Chrome.`,
        }}
      >
        <p>
          <MonoLeadIn>A port is only useful</MonoLeadIn> if it behaves like the original. So
          Ferrigura does not take Mermaid&rsquo;s word for it, and does not ask you to take ours. A
          test harness runs the original next to the port and compares them, type by type.
        </p>
      </MonoPassage>
      <MonoPassage
        note={{
          side: 'right',
          figure: grouped(coverage.parsed),
          children: 'inputs read the way Mermaid reads them, or rejected by both',
        }}
      >
        <p>
          The first comparison is about reading. Fixtures, Mermaid&rsquo;s own test cases and the
          examples from its documentation go through both parsers, and the structures they produce
          must agree. The second is about drawing. Both sides draw the same diagrams in Chromium,
          and every box, position, edge point and text has to land within 0.05 pixels of
          Mermaid&rsquo;s.
        </p>
      </MonoPassage>
      <CoverageProofs />
      <p>
        {grouped(coverage.notCompared)} drawings are left out of the comparison, each for a stated
        reason: a hand-drawn look Ferrigura does not draw, a script no bundled font covers, or an
        input on which Mermaid itself fails.{' '}
        <Link to="/diagrams/overview/">The checks for each type</Link>
      </p>
    </MonoSection>
  );
}

const alternatives = [
  {
    name: 'Mermaid',
    href: 'https://mermaid.js.org/',
    runs: 'TypeScript, in a browser',
    fit: 'The original, and the right choice when diagrams are drawn in the reader’s browser, with click callbacks, tooltips and the newest diagram types first.',
    differs:
      'On a server it needs a headless browser, through mermaid-cli. Labels use the page’s fonts.',
  },
  {
    name: 'Pele',
    href: 'https://pele.run/',
    runs: 'TypeScript, any JavaScript runtime',
    fit: 'An independent renderer of Mermaid’s syntax, built for Obsidian, when speed matters most.',
    differs: 'It draws a layout and style of its own rather than Mermaid’s, and faster.',
  },
  {
    name: 'merman',
    href: 'https://github.com/Latias94/merman',
    runs: 'Rust, with many bindings',
    fit: 'A headless Rust implementation aimed at Mermaid’s output, with PNG, PDF and ASCII as well as SVG, and bindings such as WebAssembly and Python.',
    differs:
      'By default it estimates text widths rather than measuring bundled fonts, and leaves the fonts to the viewer.',
  },
];

export function Alternatives() {
  return (
    <MonoSection
      id="alternatives"
      title="Other ways to draw Mermaid diagrams."
      standfirst="Ferrigura is one of several renderers for Mermaid’s syntax. Which one fits depends on where the diagrams are drawn and how close to Mermaid they must look."
    >
      <p>
        <MonoLeadIn>Choose Ferrigura</MonoLeadIn> when you want Mermaid&rsquo;s own drawing without
        a browser, the same on every machine, with its fonts inside the file: in a Rust program, on
        the command line or in Node.js. Choose one of these when something else matters more.
      </p>
      <MonoWide className="fgr-gap">
        <MonoCards>
          {alternatives.map((entry) => (
            <MonoCard key={entry.name} title={entry.name} api={entry.runs}>
              <p className="fgr-card-text">{entry.fit}</p>
              <MonoVerdict>{entry.differs}</MonoVerdict>
              <p className="fgr-card-link">
                <a href={entry.href}>{entry.href.replace(/^https:\/\//, '').replace(/\/$/, '')}</a>
              </p>
            </MonoCard>
          ))}
        </MonoCards>
      </MonoWide>
      <p>
        The comparison page sets these and mermaid-rs-renderer side by side, with measured sizes and
        render times, and says what each does better.{' '}
        <Link to="/comparisons/overview/">Compare the renderers</Link>
      </p>
    </MonoSection>
  );
}

const limits = [
  {
    term: 'No interaction',
    description:
      'Click callbacks and tooltips need a browser at run time, so they have no meaning in a static file.',
  },
  {
    term: 'No hand-drawn look',
    description: 'Shapes take the classic geometry, without the sketched strokes.',
  },
  {
    term: 'No math in labels',
    description: 'Formulas between $$ marks are not typeset.',
  },
  {
    term: 'Two themes',
    description: 'Mermaid’s classic themes (default, dark, forest, neutral) are not shipped.',
  },
  {
    term: 'No fallback font',
    description:
      'Text in Chinese, Japanese, Korean or Arabic, and emoji the bundled fonts lack, cannot be drawn yet.',
  },
  {
    term: 'No images or icon packs',
    description: 'An image shape needs its size given in the text; icon packs are not read.',
  },
];

export function Boundaries() {
  return (
    <MonoSection
      id="boundaries"
      title="Early, and clear about its limits."
      standfirst={`Ferrigura is an early member of its family. ${STATUS_MEANING[tool.status]}`}
    >
      <p>
        <MonoLeadIn>Every diagram type</MonoLeadIn> of Mermaid {coverage.mermaid} renders, and the
        checks above run in the repository&rsquo;s continuous integration. What is young is
        everything around them: the interface may still change between releases, and a few parts of
        Mermaid are left out on purpose or not done yet.
      </p>
      <MonoTerms items={limits} columns={3} prose className="fgr-gap" />
      <p>
        The output is SVG only. For PNG or PDF, hand the file to a converter of your choice.{' '}
        <Link to="/reference/rust/#not-supported-yet">The full list of limits</Link>
      </p>
    </MonoSection>
  );
}

export function Runtimes() {
  return (
    <MonoSection
      id="runtimes"
      title="Three ways in, one engine."
      standfirst="Render from Node.js, from a Rust program or from the shell. All three run the same engine."
    >
      <MonoPassage
        note={{
          side: 'right',
          term: 'Native addon',
          children:
            'A compiled binary for each platform that npm installs beside the package. No build step on your side.',
        }}
      >
        <p>
          <MonoLeadIn>The engine is written</MonoLeadIn> in Rust. The crate gives a Rust program the
          renderer directly. The command-line tool renders files in a build script. The npm package
          brings the same engine to Node.js as a native addon, with a small JavaScript API and no
          WebAssembly. The library and the package render synchronously and return the SVG; the tool
          writes it to a file or to standard output.
        </p>
      </MonoPassage>
      <StartCards />
    </MonoSection>
  );
}

export function Family() {
  const notes = tool.editorial?.notes ?? [];
  const planned = family.filter((entry) => entry.plannedRunsOn?.includes(tool.name));
  return (
    <MonoSection
      id="family"
      title="Part of a workshop of engines."
      standfirst="Ferrigura works on its own. These are the places where it meets the rest of the Ferramenta family."
    >
      <p>
        Each Ferramenta engine has one job and its own interface, and none needs another beside it.
        {planned.length > 0 && (
          <>
            {' '}
            {planned.map((entry) => displayName(entry)).join(' and ')} plans to draw its diagrams
            with Ferrigura.
          </>
        )}
      </p>
      <MonoWide>
        <MonoCards layout="two">
          {notes.map((note) => (
            <MonoCard key={note.title} title={note.title}>
              <p className="fgr-card-text">{note.body}</p>
              <MonoVerdict>
                <a href={note.href}>Read more</a>
              </MonoVerdict>
            </MonoCard>
          ))}
        </MonoCards>
      </MonoWide>
    </MonoSection>
  );
}
