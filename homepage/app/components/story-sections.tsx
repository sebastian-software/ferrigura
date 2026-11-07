import { ArdoLink as Link } from 'ardo/runtime';
import { MonoLeadIn, MonoPassage, MonoSection } from 'ferramenta-family/monografie';

import samples from '../data/diagram-samples.json';
import { DiagramGallery, FileContents, ModeSheets, SchemeSheets } from './homepage-specimens';

/* The homepage's first chapters: what Ferrigura is, its text and fonts, the
   flowchart modes and the color schemes. The published anchor #example stays
   where links expect it. */

export function Explainer() {
  return (
    <MonoSection id="example" title="A few lines of text become a drawing.">
      <MonoPassage
        note={{
          term: 'Mermaid',
          children:
            'A language for diagrams in plain text, and the JavaScript library that draws them. GitHub, GitLab and many documentation tools understand it.',
        }}
      >
        <p>
          <MonoLeadIn>A diagram in a document</MonoLeadIn> is usually a picture: drawn in one tool,
          exported, pasted in, and out of date by the next release. Mermaid took another road. You
          write the diagram as text, a box here, an arrow there, right beside the prose that
          explains it. The text lives in the repository, changes in the same review as the code, and
          a renderer turns it into a drawing.
        </p>
      </MonoPassage>
      <MonoPassage
        note={{
          side: 'right',
          term: 'Headless browser',
          children:
            'A full web browser without a window, started by a program to load a page and read the result.',
        }}
      >
        <p>
          That renderer has always needed a browser. Mermaid measures its labels with the
          page&rsquo;s fonts and lays out the drawing inside a web page. In a reader&rsquo;s browser
          that costs a script download. In a documentation build, a server or a test, it means
          starting a headless browser for every batch of diagrams, and the result depends on the
          fonts installed on the machine that drew it.
        </p>
      </MonoPassage>
      <p>
        Ferrigura is Mermaid&rsquo;s drawing, carried over to Rust. It reads the same diagram text,
        lays it out with the same engines and writes a standalone SVG file, with no browser in the
        process. Pick a type and compare the text with the drawing it became.
      </p>
      <DiagramGallery />
      <p>
        Each drawing is an ordinary SVG file. It can be written next to the page that shows it,
        inlined into HTML, or served as an image, and it looks the same wherever it goes.{' '}
        <Link to="/guide/getting-started/">Render your first diagram</Link>
      </p>
    </MonoSection>
  );
}

/** Why labels are SVG text, with the count of HTML elements in the file beside it. */
function LabelsStayText() {
  const { file } = samples;
  return (
    <MonoPassage
      note={{
        side: 'right',
        figure: file.htmlElements === 0 ? 'none' : file.htmlElements,
        children: 'HTML elements inside the drawing: every label is SVG text',
      }}
    >
      <p>
        The labels stay text. By default, Mermaid places its labels as HTML inside the SVG, which
        PDF converters, Inkscape and many other SVG tools do not show at all. Ferrigura writes them
        as SVG text, so they look the same everywhere, and a reader can still select and copy them,
        a search engine index them, and a screen reader read them out.
      </p>
    </MonoPassage>
  );
}

export function Text() {
  return (
    <MonoSection
      id="text"
      title="Labels stay text. Their font comes along."
      standfirst="Ferrigura measures every label with fonts it brings along, then embeds exactly the letters the drawing uses."
    >
      <MonoPassage
        note={{
          term: 'Font subset',
          children:
            'A copy of a font that keeps only the letters a document needs, so it adds kilobytes rather than megabytes.',
        }}
      >
        <p>
          <MonoLeadIn>The size of a box</MonoLeadIn> depends on the words inside it. To place a
          node, an edge or a line break, a renderer must know how wide each label will be, and that
          width depends on the font. Measure with one font and display with another, and labels
          spill out of their boxes or wrap where they should not.
        </p>
      </MonoPassage>
      <p>
        Ferrigura measures with the fonts it ships: Lexica Ultralegible for text and Atkinson
        Hyperlegible Mono for code. It then writes the letters each drawing uses into the file as a
        font subset. The viewer draws the text with the very font it was measured with, whatever is
        installed on the reader&rsquo;s machine.
      </p>
      <LabelsStayText />
      <FileContents />
      <p>
        Each word sits at the position its measured width gives it. Colors are CSS custom
        properties, scoped to the diagram&rsquo;s own id, so several drawings can share one page
        without their styles leaking into each other.{' '}
        <Link to="/architecture/decisions/0010-text-measurement-and-fonts/">
          How the text is measured
        </Link>
      </p>
    </MonoSection>
  );
}

export function Modes() {
  return (
    <MonoSection
      id="flowchart-mode"
      title="Two ways to draw a flowchart."
      standfirst="By default, Ferrigura straightens the small steps Mermaid leaves in simple flowcharts. One option brings Mermaid’s drawing back."
    >
      <MonoPassage
        note={{
          term: 'Port',
          children: 'The point where an arrow meets the edge of a box.',
        }}
      >
        <p>
          <MonoLeadIn>Mermaid spreads</MonoLeadIn> the ports of a box across its side. When two
          boxes of different widths are joined, their ports rarely line up, and the arrow between
          them takes a small step. Its edge labels are also only half opaque, so a line shows
          through the text that sits on it.
        </p>
      </MonoPassage>
      <p>
        Ferrigura&rsquo;s adaptive mode centers the ports and leaves room for the labels, so the
        layout engine can draw those arrows straight. It also gives the labels a solid background.
        Necessary bends remain, and in a complex graph the nodes can move. Where a diagram needs the
        exact drawing of Mermaid, the Mermaid mode keeps it.
      </p>
      <ModeSheets />
      <p>
        The mode is an option of each render, not a setting in the diagram text, so one renderer can
        draw both. It affects flowcharts only.{' '}
        <Link to="/architecture/decisions/0019-adaptive-flowchart-mode/">
          Why adaptive is the default
        </Link>
      </p>
    </MonoSection>
  );
}

export function Schemes() {
  return (
    <MonoSection
      id="schemes"
      title="Light, dark, or the reader’s choice."
      standfirst="Choose the colors when you render, or let each reader’s system choose them."
    >
      <MonoPassage
        note={{
          side: 'right',
          term: 'Color scheme',
          children:
            'The light or dark appearance a reader has chosen for their system, which a page can follow.',
        }}
      >
        <p>
          <MonoLeadIn>A drawing that sits</MonoLeadIn> on a documentation page has to suit the page
          around it. Ferrigura renders in a light or a dark scheme. With the automatic scheme, the
          SVG carries both and follows the reader&rsquo;s system setting, with no script and no
          second file.
        </p>
      </MonoPassage>
      <SchemeSheets />
      <p>
        Every color is a CSS custom property, and each element also carries its color as an
        attribute, for viewers that do not read custom properties.{' '}
        <Link to="/architecture/decisions/0011-themes-as-css-variables/">How themes work</Link>
      </p>
    </MonoSection>
  );
}
