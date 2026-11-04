import type { ArdoMetaFunction } from 'ardo/runtime';

import { ArdoLink as Link } from 'ardo/runtime';
import { Mark, RegistryFacts } from 'ferramenta-family';
import {
  FerraArticle,
  FerraChoices,
  FerraCode,
  FerraColumns,
  FerraEditorialRow,
  FerraFeatureGrid,
  FerraInstrument,
  FerraNotes,
  FerraProductLead,
  FerraProductPage,
  FerraReleaseFacts,
} from 'ferramenta-family/ferra';
import { family, WORKSHOP } from 'ferramenta-family/registry';

import { DiagramExample } from '../components/diagram-example';
import { SiteAction } from '../components/site-action';
import site from '../data/site.json';

// oxlint-disable-next-line react/only-export-components -- Ardo reads route metadata.
export const meta: ArdoMetaFunction = () => [
  { title: 'Ferrigura — Mermaid diagrams, rendered in Rust.' },
  {
    name: 'description',
    content:
      'Render Mermaid diagram text to standalone SVG, without a browser. Bundled fonts, selectable labels and native Rust rendering for your documentation build.',
  },
];
// oxlint-disable-next-line react/only-export-components -- Ardo reads the route layout.
export const handle = { layout: 'bare' };
const member = family.find((entry) => entry.name === 'ferrigura');
if (!member) throw new Error('Ferrigura is missing from the family registry.');
const tool = { ...member, version: site.version };

function Opening() {
  return (
    <FerraProductLead
      id="overview"
      title="Mermaid diagrams, rendered in Rust."
      introduction="Turn diagram text into standalone SVG for your documentation, application or build. Ferrigura runs without a browser process and embeds the fonts it uses, while keeping labels as selectable text. Start with the Rust library or the command-line tool."
      art={<FerraInstrument product="ferrigura" />}
      byline={
        <>
          An open-source project by <a href={WORKSHOP.openSource}>{WORKSHOP.company}</a>.
        </>
      }
    >
      <div className="ferra-actions">
        <SiteAction to="/guide/getting-started/">Render your first diagram</SiteAction>
        <SiteAction variant="secondary" to="#example">
          See the SVG output
        </SiteAction>
      </div>
    </FerraProductLead>
  );
}
function Highlights() {
  return (
    <FerraColumns>
      <FerraArticle
        title="A diagram your build can produce."
        icon={<Mark name="package" size={28} />}
        meta={<FerraReleaseFacts tool={tool} />}
      >
        <p>
          Render in a native process and keep the SVG alongside your other build artifacts. No
          headless browser needs to start, and the result can travel with the page or document that
          uses it.
        </p>
        <Link to="/reference/cli/">Use it from the command line</Link>
      </FerraArticle>
      <FerraArticle
        title="Labels stay text. Fonts travel with them."
        icon={<Mark name="document" size={28} />}
        meta={
          <Link to="/architecture/decisions/0010-text-measurement-and-fonts/">
            How text measurement works
          </Link>
        }
      >
        <p>
          Text is measured with bundled fonts, then the glyphs it needs are embedded in the SVG.
          Labels stay selectable and searchable, with line breaks tied to the same typography the
          viewer receives.
        </p>
        <Link to="/reference/rust/#how-the-output-differs-from-mermaid">
          Understand the output contract
        </Link>
      </FerraArticle>
    </FerraColumns>
  );
}
function Scope() {
  return (
    <FerraEditorialRow
      notes={<FerraNotes label="Inside Ferrigura" notes={tool.editorial?.notes ?? []} />}
    >
      <FerraArticle
        title="Familiar diagram text. Explicit rendering choices."
        introduction="Ferrigura ports Mermaid 12.2.0. Its type reference documents the supported syntax, layout and limits for each diagram."
        meta={<Link to="/diagrams/overview/">Explore diagram coverage</Link>}
      >
        <FerraFeatureGrid
          items={[
            {
              heading: 'Diagrams for different questions',
              text: 'Flowcharts, sequences, classes, states and entity relationships sit alongside the other supported diagram types.',
            },
            {
              heading: 'Geometry checked against Mermaid',
              text: 'The verification harness compares parser models and rendered geometry with the pinned upstream implementation in Chromium.',
            },
            {
              heading: 'One renderer, many inputs',
              text: 'Reuse resolved configuration across diagrams. Give inline SVGs unique IDs so their styles and references stay scoped.',
            },
            {
              heading: 'Repeatable output',
              text: 'Fixed randomness and per-render state support reproducible output. For gantt diagrams that depend on today, Rust callers supply the clock explicitly.',
            },
          ]}
        />
      </FerraArticle>
    </FerraEditorialRow>
  );
}
function Boundaries() {
  return (
    <FerraArticle
      title="Choose it with the boundaries in view."
      meta={<Link to="/comparisons/overview/">Compare rendering approaches</Link>}
    >
      <p>
        Ferrigura produces SVG. Labels use SVG text rather than HTML, and the renderer has no
        browser callbacks or tooltips. Math labels, hand-drawn strokes, classic themes and some font
        coverage are outside the supported scope.
      </p>
      <p>
        Install <code>@ferrigura/core</code> from npm to render in Node.js. npm selects the native
        binary for your platform. Ferromark’s optional adapter renders Mermaid fences in custom
        builds, alongside its Markdown renderer.
      </p>
      <Link to="/reference/rust/#not-supported-yet">Read the specific limitations</Link>
    </FerraArticle>
  );
}
function GettingStarted() {
  return (
    <FerraChoices>
      <FerraArticle
        title="Add rendering to your Rust application."
        introduction="Pass diagram text and configuration to the library. Reuse a renderer when you have several blocks to process."
      >
        <FerraCode caption="Add the library">cargo add ferrigura</FerraCode>
        <SiteAction to="/guide/getting-started/#use-the-rust-library">Start with Rust</SiteAction>
      </FerraArticle>
      <FerraArticle
        title="Give your build an SVG file."
        introduction="Use the native CLI with files or standard input. Render several inputs into an output directory with one command."
      >
        <FerraCode caption="Install the command-line tool">
          cargo install --locked ferrigura-cli
        </FerraCode>
        <SiteAction variant="secondary" to="/guide/getting-started/#render-a-file">
          Start with the CLI
        </SiteAction>
      </FerraArticle>
    </FerraChoices>
  );
}
export default function HomePage() {
  return (
    <RegistryFacts>
      <FerraProductPage>
        <Opening />
        <Highlights />
        <DiagramExample />
        <Scope />
        <Boundaries />
        <GettingStarted />
      </FerraProductPage>
    </RegistryFacts>
  );
}
