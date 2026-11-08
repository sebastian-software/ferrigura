import type { ArdoMetaFunction } from 'ardo/runtime';

import { Fragment } from 'ardo/react';
import { ArdoLink as Link } from 'ardo/runtime';
import {
  MonoAction,
  MonoClosing,
  MonoColophon,
  MonoEngraving,
  MonoOpening,
} from 'ferramenta-family/monografie';
import { displayName, WORKSHOP } from 'ferramenta-family/registry';

import { Alternatives, Checked, Family, Runtimes } from '../components/feature-sections';
import { grouped, tool } from '../components/homepage-data';
import { Explainer, Modes, Schemes, Text } from '../components/story-sections';
import coverage from '../data/coverage.json';
import { nodeVersion, releaseLine, rustVersion } from '../version';

// oxlint-disable-next-line react/only-export-components -- Ardo reads route metadata.
export const meta: ArdoMetaFunction = () => [
  { title: 'Ferrigura — Mermaid diagrams, drawn without a browser' },
  {
    name: 'description',
    content:
      'Ferrigura renders Mermaid diagram text to standalone SVG in Rust, without a browser: bundled fonts, labels that stay text, and every diagram type checked against Mermaid. For Rust, Node.js and the command line.',
  },
];
// oxlint-disable-next-line react/only-export-components -- Ardo reads the route layout.
export const handle = { layout: 'bare' };

function Actions() {
  return (
    <>
      <MonoAction
        href="/guide/getting-started/#use-the-rust-library"
        primary
        detail="cargo add ferrigura"
      >
        Start with Rust
      </MonoAction>
      <MonoAction href="/reference/npm/" detail="npm install @ferrigura/core">
        Start with Node.js
      </MonoAction>
    </>
  );
}

function Opening() {
  return (
    <div id="overview">
      <MonoOpening
        product="ferrigura"
        instrumentLabel="Ferrigura's instrument: three steel roller conveyors meet in a Y at a switch plate with a rust-brown dial. Square and round steel pieces ride the lanes, one lane leading in and two branching off."
        kicker={`${displayName(tool)} ${releaseLine}, ${tool.what.toLowerCase()} for Rust, Node.js and the command line`}
        title="Mermaid diagrams, drawn without a browser."
        subtitle="Fonts included, labels still text."
        standfirst={`Ferrigura turns Mermaid diagram text into standalone SVG files for documentation sites, build tools and servers. It is a port of Mermaid ${coverage.mermaid} to Rust: no browser starts, the fonts the labels were measured with travel inside the file, and every label stays real text.`}
        actions={<Actions />}
        facts={[
          {
            value: (
              <>
                <em>{coverage.types.length}</em> of {coverage.mermaidTypes}
              </>
            ),
            label: `diagram types of Mermaid ${coverage.mermaid}`,
          },
          {
            value: (
              <>
                <em>{grouped(coverage.agreed)}</em> of {grouped(coverage.compared)}
              </>
            ),
            label: 'drawings within 0.05 px of Mermaid’s',
          },
          {
            value: <em>{grouped(coverage.parsed)}</em>,
            label: 'inputs read the way Mermaid reads them',
          },
          { value: `Rust ${rustVersion}+`, label: `Node.js ${nodeVersion}+, MIT license` },
        ]}
      />
    </div>
  );
}

// Keys are written out: minified builds do not keep function names.
const chapters = [
  { key: 'explainer', Chapter: Explainer },
  { key: 'text', Chapter: Text },
  { key: 'modes', Chapter: Modes },
  { key: 'schemes', Chapter: Schemes },
  { key: 'checked', Chapter: Checked },
  { key: 'alternatives', Chapter: Alternatives },
  { key: 'runtimes', Chapter: Runtimes },
  { key: 'family', Chapter: Family },
];

function Origin() {
  return (
    <section aria-label="Origin">
      <div className="mono-axis">
        <MonoColophon>
          {tool.proof} Mermaid was created by Knut Sveidqvist and is maintained by the Mermaid
          contributors; its license and copyright notice stay with the port. Ferrigura is one of the
          Ferramenta engines, made by {WORKSHOP.name} in {WORKSHOP.place}.
        </MonoColophon>
      </div>
    </section>
  );
}

/** The actions again, under the published anchor #start. */
function Closing() {
  return (
    <div id="start">
      <MonoClosing
        title="Draw your next diagram in the build."
        standfirst="Add the crate or the npm package, hand it your diagram text and keep the SVG with the rest of your site."
        actions={<Actions />}
        links={
          <>
            <a href="https://crates.io/crates/ferrigura">crates.io</a>
            <a href="https://www.npmjs.com/package/@ferrigura/core">npm</a>
            <a href="https://github.com/sebastian-software/ferrigura">GitHub repository</a>
            <Link to="/guide/getting-started/">Documentation</Link>
            <a href={WORKSHOP.consulting}>Integration and support</a>
          </>
        }
      />
    </div>
  );
}

export default function HomePage() {
  return (
    <>
      <Opening />
      {chapters.map(({ key, Chapter }, index) => (
        <Fragment key={key}>
          <MonoEngraving seed={index + 1} />
          <Chapter />
        </Fragment>
      ))}
      <MonoEngraving seed={chapters.length + 1} />
      <Origin />
      <Closing />
    </>
  );
}
