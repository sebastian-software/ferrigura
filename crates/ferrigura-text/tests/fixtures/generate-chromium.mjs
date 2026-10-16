// Regenerates chromium.json, the browser reference for the ferrigura-text tests.
//
// Run from the repository root (Playwright and its Chromium must be installed):
//
//   node crates/ferrigura-text/tests/fixtures/generate-chromium.mjs \
//     > crates/ferrigura-text/tests/fixtures/chromium.json
//
// Widths are the drawn extent of a text element: from the start of its first character to the
// end of its last one, read with getStartPositionOfChar and getEndPositionOfChar. The spike's
// getComputedTextLength and getBBox are not used, because WebKit reports unkerned widths through
// them. Text box ascent and descent are the font bounding box that canvas reports, which Chromium
// rounds to whole pixels. Wrapping is Mermaid's splitLineToFitWidth (rendering-util/splitText.ts),
// ported line for line and measured in the browser.

import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';

const fontDir = new URL('../../fonts/', import.meta.url);
const FONT_FILES = {
  'sans-regular': 'LexicaUltralegible-Regular.ttf',
  'sans-italic': 'LexicaUltralegible-Italic.ttf',
  'sans-bold': 'LexicaUltralegible-Bold.ttf',
  'sans-bolditalic': 'LexicaUltralegible-BoldItalic.ttf',
  'mono-regular': 'AtkinsonHyperlegibleMono-Regular.ttf',
  'mono-italic': 'AtkinsonHyperlegibleMono-Italic.ttf',
  'mono-bold': 'AtkinsonHyperlegibleMono-Bold.ttf',
  'mono-bolditalic': 'AtkinsonHyperlegibleMono-BoldItalic.ttf',
};
const STYLES = Object.keys(FONT_FILES);

const LABELS = [
  'office',
  'fluffy',
  'fjord',
  'affinity',
  'AVATAR',
  'Type',
  'WAVE',
  'Yo',
  'Te',
  'LT',
  'Größenänderung Übermaß Ärger öffnen',
  '0123456789 10.5% $1,000.00',
  ' leading and trailing spaces ',
  'Supercalifragilisticexpialidocious',
  'a',
  'CamelCaseIdentifierWithoutSpaces',
  'Mixed: punctuation, (brackets) [and] {braces}!?',
  'Données français: «guillemets» — tirets – et …',
];

const WRAP_TEXTS = [
  'Supercalifragilisticexpialidocious',
  'CamelCaseIdentifierWithoutSpaces',
  'The quick brown fox jumps over the lazy dog',
  'Data flow: user submits form, system validates input',
  'Größenänderung Übermaß Ärger öffnen',
  'a b c d e f g h i j k l m n o p',
  'office fluffy fjord affinity',
  'Données français: «guillemets» — tirets – et …',
  ' leading   and double  spaces ',
];
const WRAP_WIDTHS = [80, 140, 200];

// Each run is [text, style]; the line is measured at the given size.
const RUN_LINES = [
  [
    ['A', 'sans-regular'],
    ['V', 'sans-bold'],
  ],
  [
    ['Type ', 'sans-regular'],
    ['WAVE', 'sans-bold'],
  ],
  [
    ['Te', 'sans-regular'],
    ['st', 'sans-italic'],
    ['LT', 'sans-bolditalic'],
  ],
  [
    ['office ', 'sans-regular'],
    ['fluffy', 'sans-bold'],
  ],
  [
    ['Mixed ', 'sans-regular'],
    ['bold', 'sans-bold'],
    [' and ', 'sans-regular'],
    ['italic', 'sans-italic'],
    [' end', 'sans-regular'],
  ],
  [
    ['AVATAR ', 'mono-regular'],
    ['Type', 'mono-bold'],
  ],
];

const fontFace = (style) =>
  `@font-face { font-family: "t-${style}"; src: url(data:font/ttf;base64,${readFileSync(
    new URL(FONT_FILES[style], fontDir)
  ).toString('base64')}); }`;

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setContent(
  `<!doctype html><html><head><meta charset="utf-8"><style>${STYLES.map(fontFace).join('\n')}</style></head><body></body></html>`
);

const requests = {
  widths: [],
  wraps: [],
  runs: RUN_LINES,
  boxes: [],
};
for (const text of LABELS) {
  for (const style of STYLES) {
    requests.widths.push({ text, style, size: 16 });
  }
  requests.widths.push({ text, style: 'sans-regular', size: 14 });
  requests.widths.push({ text, style: 'sans-regular', size: 24 });
}
for (const text of WRAP_TEXTS) {
  for (const wrapWidth of WRAP_WIDTHS) {
    requests.wraps.push({ text, style: 'sans-regular', size: 16, wrapWidth });
  }
}
for (const style of STYLES) {
  for (const size of [14, 16, 24]) {
    requests.boxes.push({ style, size });
  }
}

const results = await page.evaluate(async (requests) => {
  const svgNs = 'http://www.w3.org/2000/svg';
  const family = (style) => `"t-${style}"`;
  for (const style of Object.keys(requests.fontsFor)) {
    await document.fonts.load(`16px ${family(style)}`, 'A');
  }
  await document.fonts.ready;

  const svg = document.createElementNS(svgNs, 'svg');
  svg.setAttribute('width', '2000');
  svg.setAttribute('height', '200');
  document.body.append(svg);
  const text = document.createElementNS(svgNs, 'text');
  text.setAttribute('x', '0');
  text.setAttribute('y', '50');
  text.style.whiteSpace = 'pre';
  svg.append(text);
  const ctx = document.createElement('canvas').getContext('2d');

  // Drawn extent of the text element: first character's start to last character's end.
  const extent = (el) => {
    const n = el.getNumberOfChars();
    if (n === 0) {
      return 0;
    }
    return el.getEndPositionOfChar(n - 1).x - el.getStartPositionOfChar(0).x;
  };
  const setText = (parts) => {
    text.replaceChildren();
    for (const [content, style] of parts) {
      const tspan = document.createElementNS(svgNs, 'tspan');
      tspan.style.fontFamily = family(style);
      tspan.style.fontSize = `${requests.size}px`;
      tspan.textContent = content;
      text.append(tspan);
    }
  };

  // Mermaid's splitText.ts, ported without types. Words are the markdown words of one line.
  const splitTextToChars = (value) =>
    [...new Intl.Segmenter().segment(value)].map((s) => s.segment);
  const splitWordToFitWidth = (checkFit, word) =>
    splitWordToFitWidthRecursion(checkFit, [], splitTextToChars(word.content), word.type);
  const splitWordToFitWidthRecursion = (checkFit, usedChars, remainingChars, type) => {
    if (remainingChars.length === 0) {
      return [{ content: usedChars.join(''), type }, { content: '', type }];
    }
    const [nextChar, ...rest] = remainingChars;
    const newWord = [...usedChars, nextChar];
    if (checkFit([{ content: newWord.join(''), type }])) {
      return splitWordToFitWidthRecursion(checkFit, newWord, rest, type);
    }
    if (usedChars.length === 0 && nextChar) {
      usedChars.push(nextChar);
      remainingChars.shift();
    }
    return [
      { content: usedChars.join(''), type },
      { content: remainingChars.join(''), type },
    ];
  };
  const splitLineToFitWidth = (line, checkFit) =>
    splitLineToFitWidthRecursion([...line], checkFit);
  const splitLineToFitWidthRecursion = (words, checkFit, lines = [], newLine = []) => {
    if (words.length === 0) {
      if (newLine.length > 0) {
        lines.push(newLine);
      }
      return lines.length > 0 ? lines : [];
    }
    let joiner = '';
    if (words[0].content === ' ') {
      joiner = ' ';
      words.shift();
    }
    const nextWord = words.shift() ?? { content: ' ', type: 'normal' };
    const lineWithNextWord = [...newLine];
    if (joiner !== '') {
      lineWithNextWord.push({ content: joiner, type: 'normal' });
    }
    lineWithNextWord.push(nextWord);
    if (checkFit(lineWithNextWord)) {
      return splitLineToFitWidthRecursion(words, checkFit, lines, lineWithNextWord);
    }
    if (newLine.length > 0) {
      lines.push(newLine);
      words.unshift(nextWord);
    } else if (nextWord.content) {
      const [line, rest] = splitWordToFitWidth(checkFit, nextWord);
      lines.push([line]);
      if (rest.content) {
        words.unshift(rest);
      }
    }
    return splitLineToFitWidthRecursion(words, checkFit, lines);
  };

  // createText.ts: one tspan per word, with a leading space on every word after the first.
  const wordsAsParts = (words) =>
    words.map((word, index) => [index === 0 ? word.content : ` ${word.content}`, requests.wrapStyle]);
  const measureWords = (words) => {
    setText(wordsAsParts(words));
    return extent(text);
  };

  const widths = requests.widths.map((r) => {
    requests.size = r.size;
    setText([[r.text, r.style]]);
    return { ...r, width: extent(text) };
  });

  const runs = requests.runs.map((line) => {
    requests.size = 16;
    setText(line.map(([content, style]) => [content, style]));
    return {
      runs: line.map(([content, style]) => ({ text: content, style })),
      size: 16,
      width: extent(text),
    };
  });

  const wraps = requests.wraps.map((r) => {
    requests.size = r.size;
    requests.wrapStyle = r.style;
    const words = r.text
      .split(' ')
      .filter((w) => w)
      .map((content) => ({ content, type: 'normal' }));
    const checkFit = (line) => measureWords(line) <= r.wrapWidth;
    const linesUnderWidth = checkFit(words) ? [words] : splitLineToFitWidth(words, checkFit);
    return {
      ...r,
      lines: linesUnderWidth.map((line) => line.map((w) => w.content).join(' ')),
    };
  });

  const boxes = requests.boxes.map((r) => {
    ctx.font = `${r.size}px ${family(r.style)}`;
    const metrics = ctx.measureText('Hg');
    return {
      ...r,
      ascent: metrics.fontBoundingBoxAscent,
      descent: metrics.fontBoundingBoxDescent,
    };
  });

  return { widths, runs, wraps, boxes };
}, { ...requests, fontsFor: Object.fromEntries(STYLES.map((s) => [s, true])) });

await browser.close();

const out = {
  generatedBy: 'crates/ferrigura-text/tests/fixtures/generate-chromium.mjs',
  ...results,
};
process.stdout.write(`${JSON.stringify(out, null, 1)}\n`);
