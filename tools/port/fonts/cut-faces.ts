/**
 * Cuts the two fallback faces of ferrigura-text from their sources (decision 5 of 2026-10-20 in
 * `docs-port/P3-PLAN.md`):
 *
 * - `TwemojiMozilla-COLRv0-Emoji.ttf`: Twemoji Mozilla 0.7.0 (COLRv0, colour), cut to the code points
 *   that have emoji presentation (`Emoji_Presentation`) and that the font covers. A code point with
 *   emoji presentation that the font lacks is not in the cut, so no bundled face draws it.
 * - `NotoSansSymbols2-Regular-TextSymbols.ttf`: Noto Sans Symbols 2 (OFL), cut to the code points of
 *   U+2600-27BF that are text-default (not `Emoji_Presentation`) and that the font covers. It never
 *   holds an emoji-presentation code point, so such a code point falls to the emoji face.
 *
 * The sources are the files of the two upstream releases, named as they are in their packages
 * (`twemoji-colr/Twemoji.Mozilla.v0.7.0.ttf`, `noto-fonts/NotoSansSymbols2-Regular.ttf`); the
 * licence files are checked too. Each is checked against its SHA-256 before it is read. The cut
 * is done by `ferrigura-text`'s `cut_font` example, with the plan the crate's subsets use.
 *
 * Usage:
 *   pnpm port:cut-fonts -- <dir>          cut the faces into crates/ferrigura-text/fonts
 *   pnpm port:cut-fonts -- <dir> --check  cut again and compare with the bundled files
 *
 * `<dir>` holds the four source files at the paths above. The sources and their checksums are in
 * `crates/ferrigura-text/NOTICE.md`.
 */
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { readCmap } from './cmap.ts';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fontDir = join(root, 'crates/ferrigura-text/fonts');

/** The sources, by their path in `<dir>`, with the SHA-256 each must have. */
const SOURCES = {
  twemoji: {
    path: 'twemoji-colr/Twemoji.Mozilla.v0.7.0.ttf',
    sha256: '6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e',
  },
  symbols2: {
    path: 'noto-fonts/NotoSansSymbols2-Regular.ttf',
    sha256: '882d142b9a1ef3fd7fa4225dbe95c10fab6664206eb4964c8ff705a4f6d02988',
  },
  twemojiLicence: {
    path: 'twemoji-colr/LICENSE.md',
    sha256: '64419edc28e9163204c3be73f835a8dfc34cd6c9b8f7d067bb685f119f839a99',
  },
  notoLicence: {
    path: 'noto-fonts/LICENSE',
    sha256: '0dab92d0544f7b233403f14b84a663bdbfa746982eda629e7f4f9ffe1b036feb',
  },
} as const;

/** The bundled faces: the file each cut is written to, and its source. */
const OUTPUTS = {
  emoji: 'TwemojiMozilla-COLRv0-Emoji.ttf',
  textSymbols: 'NotoSansSymbols2-Regular-TextSymbols.ttf',
} as const;

/** U+2600 to U+27BF, the block of the text symbols. */
const SYMBOLS_FIRST = 0x2600;
const SYMBOLS_LAST = 0x27bf;

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

async function readChecked(directory: string, source: { path: string; sha256: string }) {
  const bytes = new Uint8Array(await readFile(join(directory, source.path)));
  const found = sha256(bytes);
  if (found !== source.sha256) {
    throw new Error(`${source.path}: SHA-256 ${found}, expected ${source.sha256}`);
  }
  return bytes;
}

/** Whether a code point has emoji presentation, as Unicode's `Emoji_Presentation` property says. */
function hasEmojiPresentation(code: number): boolean {
  return /\p{Emoji_Presentation}/u.test(String.fromCodePoint(code));
}

/** The code points of the emoji face: covered by Twemoji and with emoji presentation. */
export function emojiCoverage(twemoji: Set<number>): number[] {
  return [...twemoji].filter(hasEmojiPresentation).sort((a, b) => a - b);
}

/** The code points of the text symbols face: U+2600-27BF, covered by Symbols 2 and text-default. */
export function textSymbolCoverage(symbols2: Set<number>): number[] {
  const cut: number[] = [];
  for (let code = SYMBOLS_FIRST; code <= SYMBOLS_LAST; code++) {
    if (symbols2.has(code) && !hasEmojiPresentation(code)) {
      cut.push(code);
    }
  }
  return cut;
}

/** Cuts `source` to `codes` with the crate's `cut_font` example, and returns the cut bytes. */
async function cut(source: string, codes: readonly number[]): Promise<Uint8Array> {
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-cut-'));
  try {
    const target = join(directory, 'cut.ttf');
    const listed = codes.map((code) => `U+${code.toString(16).toUpperCase().padStart(4, '0')}`);
    execFileSync(
      'cargo',
      ['run', '--quiet', '-p', 'ferrigura-text', '--example', 'cut_font', '--', source, target],
      { cwd: root, input: listed.join('\n'), stdio: ['pipe', 'inherit', 'inherit'] }
    );
    return new Uint8Array(await readFile(target));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** Whether the cut's coverage is exactly the codes asked for. */
function coversExactly(bytes: Uint8Array, codes: readonly number[]): boolean {
  const covered = readCmap(bytes);
  return covered.size === codes.length && codes.every((code) => covered.has(code));
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const check = args.includes('--check');
  const directory = args.find((arg) => !arg.startsWith('--'));
  if (!directory) {
    throw new Error('usage: pnpm port:cut-fonts -- <dir> [--check]');
  }

  const twemoji = await readChecked(directory, SOURCES.twemoji);
  const symbols2 = await readChecked(directory, SOURCES.symbols2);
  await readChecked(directory, SOURCES.twemojiLicence);
  await readChecked(directory, SOURCES.notoLicence);

  const emojiCodes = emojiCoverage(readCmap(twemoji));
  const symbolCodes = textSymbolCoverage(readCmap(symbols2));
  const cuts: { name: string; source: string; codes: readonly number[]; bytes: Uint8Array }[] = [
    {
      name: OUTPUTS.emoji,
      source: join(directory, SOURCES.twemoji.path),
      codes: emojiCodes,
      bytes: new Uint8Array(),
    },
    {
      name: OUTPUTS.textSymbols,
      source: join(directory, SOURCES.symbols2.path),
      codes: symbolCodes,
      bytes: new Uint8Array(),
    },
  ];

  for (const entry of cuts) {
    entry.bytes = await cut(entry.source, entry.codes);
    if (!coversExactly(entry.bytes, entry.codes)) {
      throw new Error(`${entry.name} does not cover exactly the ${entry.codes.length} code points`);
    }
    const line = `${entry.name}: ${entry.codes.length} code points, ${entry.bytes.length} bytes, sha256 ${sha256(entry.bytes)}`;
    if (check) {
      const bundled = new Uint8Array(await readFile(join(fontDir, entry.name)));
      const same = Buffer.compare(Buffer.from(bundled), Buffer.from(entry.bytes)) === 0;
      console.log(`${same ? 'same' : 'DIFFERENT'}  ${line}`);
      if (!same) {
        process.exitCode = 1;
      }
    } else {
      await writeFile(join(fontDir, entry.name), entry.bytes);
      console.log(`wrote ${line}`);
    }
  }
}

await main();
