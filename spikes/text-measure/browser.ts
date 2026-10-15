/**
 * Measure the same requests in Chromium, with the same font files, two ways:
 * SVG `<text>` (getComputedTextLength) and canvas measureText. For requests with a
 * wrapWidth, also record where CSS line breaking wraps the text in a div of that width.
 *
 * Usage: `tsx browser.ts requests.json`, writes the results as JSON to stdout.
 */
import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';

const fonts: Record<string, string> = {
  regular: 'fonts/LexicaUltralegible-Regular.ttf',
  italic: 'fonts/LexicaUltralegible-Italic.ttf',
  bold: 'fonts/LexicaUltralegible-Bold.ttf',
  bolditalic: 'fonts/LexicaUltralegible-BoldItalic.ttf',
  mono: 'fonts/AtkinsonHyperlegibleMono-Regular.ttf',
};

const requests = JSON.parse(readFileSync(process.argv[2], 'utf8'));
const faces = Object.entries(fonts)
  .map(([style, path]) => {
    const data = readFileSync(path).toString('base64');
    return `@font-face { font-family: "spike-${style}"; src: url(data:font/ttf;base64,${data}); }`;
  })
  .join('\n');

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setContent(
  `<!doctype html><html><head><style>${faces}</style></head><body></body></html>`
);
// tsx/esbuild wraps named functions in `__name(...)`, which the page does not define.
await page.evaluate('window.__name = (f) => f');

const results = await page.evaluate(
  async ({ requests, styles }) => {
    for (const style of styles) {
      await document.fonts.load(`16px "spike-${style}"`);
    }
    await document.fonts.ready;

    const svgNs = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(svgNs, 'svg');
    document.body.append(svg);
    const textEl = document.createElementNS(svgNs, 'text');
    textEl.setAttribute('xml:space', 'preserve');
    textEl.style.whiteSpace = 'pre';
    textEl.setAttribute('y', '0');
    svg.append(textEl);
    const ctx = document.createElement('canvas').getContext('2d')!;
    const div = document.createElement('div');
    div.style.whiteSpace = 'normal';
    document.body.append(div);

    return requests.map((r: any) => {
      const family = `"spike-${r.style}"`;
      textEl.style.fontFamily = family;
      textEl.style.fontSize = `${r.size}px`;
      textEl.textContent = r.text;
      const svgWidth = textEl.getComputedTextLength();
      const box = textEl.getBBox();
      ctx.font = `${r.size}px ${family}`;
      const canvasWidth = ctx.measureText(r.text).width;

      let lines: string[] | undefined;
      if (r.wrapWidth) {
        div.style.fontFamily = family;
        div.style.fontSize = `${r.size}px`;
        div.style.width = `${r.wrapWidth}px`;
        div.textContent = r.text.trim();
        const node = div.firstChild as Text;
        const range = document.createRange();
        lines = [];
        let current = '';
        let top: number | undefined;
        for (let i = 0; i < node.length; i++) {
          range.setStart(node, i);
          range.setEnd(node, i + 1);
          const rect = range.getClientRects()[0];
          if (rect && top !== undefined && rect.top > top + 1) {
            lines.push(current.trimEnd());
            current = '';
          }
          if (rect) {
            top = rect.top;
          }
          current += node.data[i];
        }
        lines.push(current.trimEnd());
      }
      let mermaidLines: string[] | undefined;
      if (r.wrapWidth) {
        // Verbatim logic of Mermaid's createFormattedText + splitLineToFitWidth for SVG
        // labels: one tspan per word, ' ' prefix after the first, getComputedTextLength.
        const measure = (words: string[]) => {
          const t = document.createElementNS(svgNs, 'text');
          t.style.fontFamily = family;
          t.style.fontSize = `${r.size}px`;
          const outer = document.createElementNS(svgNs, 'tspan');
          words.forEach((w, i) => {
            const inner = document.createElementNS(svgNs, 'tspan');
            inner.textContent = i === 0 ? w : ' ' + w;
            outer.append(inner);
          });
          t.append(outer);
          svg.append(t);
          const length = outer.getComputedTextLength();
          t.remove();
          return length;
        };
        const fits = (words: string[]) => measure(words) <= r.wrapWidth;
        const graphemes = (s: string) => [...new Intl.Segmenter().segment(s)].map((g) => g.segment);
        const words = r.text.split(' ').filter((w: string) => w);
        mermaidLines = [];
        let current: string[] = [];
        while (words.length > 0) {
          const word = words.shift()!;
          if (fits([...current, word])) {
            current.push(word);
          } else if (current.length > 0) {
            mermaidLines.push(current.join(' '));
            current = [];
            words.unshift(word);
          } else {
            const chars = graphemes(word);
            const used: string[] = [];
            while (chars.length > 0 && fits([[...used, chars[0]].join('')])) {
              used.push(chars.shift()!);
            }
            if (used.length === 0 && chars.length > 0) {
              used.push(chars.shift()!);
            }
            mermaidLines.push(used.join(''));
            if (chars.length > 0) {
              words.unshift(chars.join(''));
            }
          }
        }
        if (current.length > 0) {
          mermaidLines.push(current.join(' '));
        }
      }
      return { svgWidth, canvasWidth, lines, mermaidLines, bboxHeight: box.height, bboxTop: box.y };
    });
  },
  { requests, styles: Object.keys(fonts) }
);

await browser.close();
process.stdout.write(JSON.stringify(results));
