import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const catalogFile = fileURLToPath(new URL('./catalog.json', import.meta.url));
export const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const PATH = /^fonts\/[a-z0-9-]+\/[0-9.]+\/[0-9a-f]{64}\.(?:ttf|woff2|txt)$/;
const SHA = /^[0-9a-f]{64}$/;
export const PUBLIC_BASE = 'https://assets.sebastian-software.com/ferrigura';
const ZONE = 'sebastian-websites-assets';

// Management access is read-only. The storage password stays in this process and is never logged.
export async function sharedStoragePassword(key, fetchImpl = fetch) {
  if (!key || /[\r\n]/.test(key)) throw new Error('BUNNY_API_KEY is required');
  const get = async (path) => {
    const response = await fetchImpl(`https://api.bunny.net${path}`, {
      method: 'GET',
      headers: { AccessKey: key, Accept: 'application/json' },
      redirect: 'error',
      signal: AbortSignal.timeout(30000),
    });
    if (!response.ok) throw new Error(`Bunny management GET failed: HTTP ${response.status}`);
    try {
      return await response.json();
    } catch {
      throw new Error('Invalid Bunny management response');
    }
  };
  const matches = [];
  for (let page = 1; page <= 100; page += 1) {
    const result = await get(`/pullzone?perPage=1000&page=${page}`);
    const items = Array.isArray(result) ? result : result?.Items;
    if (!Array.isArray(items)) throw new Error('Invalid pull-zone listing');
    matches.push(...items.filter((item) => item.Name === ZONE));
    if (
      Array.isArray(result) ||
      result.HasMoreItems === false ||
      (Number.isInteger(result.TotalItems) && page * 1000 >= result.TotalItems)
    )
      break;
    if (items.length === 0 || page === 100) throw new Error('Incomplete pull-zone listing');
  }
  if (matches.length !== 1 || !Number.isSafeInteger(matches[0].Id) || matches[0].Id < 1)
    throw new Error('Shared pull zone not uniquely found');
  const pull = await get(`/pullzone/${matches[0].Id}`);
  if (
    pull.Id !== matches[0].Id ||
    pull.Name !== ZONE ||
    !Number.isSafeInteger(pull.StorageZoneId) ||
    pull.StorageZoneId < 1 ||
    !pull.Hostnames?.some((host) => host.Value === 'assets.sebastian-software.com')
  )
    throw new Error('Shared pull zone has an unexpected origin or hostname');
  const storage = await get(`/storagezone/${pull.StorageZoneId}`);
  if (
    storage.Id !== pull.StorageZoneId ||
    storage.Name !== ZONE ||
    typeof storage.Password !== 'string' ||
    !storage.Password ||
    /[\r\n]/.test(storage.Password)
  )
    throw new Error('Shared storage zone does not match');
  return storage.Password;
}

export async function readCatalog(input) {
  const bytes = input ?? (await readFile(catalogFile));
  const catalog = JSON.parse(bytes);
  if (
    catalog.schemaVersion !== 1 ||
    !Array.isArray(catalog.families) ||
    catalog.families.length !== 5
  )
    throw new Error('Invalid catalog');
  const sources = new Map([
    [
      'Lexica Ultralegible',
      ['jacobxperez/lexica-ultralegible', '198700e57554026460c7dfd547ac91d910b8b0f5'],
    ],
    [
      'Atkinson Hyperlegible Mono',
      ['googlefonts/atkinson-hyperlegible-next-mono', '154d50362016cc3e873eb21d242cd0772384c8f9'],
    ],
    ['Noto Sans Math', ['notofonts/math', '00e9941d95b2a355399a66f3990ffde6e4985676']],
    ['Noto Sans Symbols 2', ['notofonts/symbols', '25b00f0d3f40873a005287514ba2a48558655314']],
    ['Inter', ['rsms/inter', 'e3a3d4c57d5ecc01453a575621882a384c1995a3']],
  ]);
  const seenFamilies = new Set();
  const paths = new Set();
  const assets = [];
  for (const family of catalog.families) {
    const source = sources.get(family.family);
    if (
      !source ||
      seenFamilies.has(family.family.toLowerCase()) ||
      family.sourceCommit !== source[1] ||
      family.source !== `https://github.com/${source[0]}/tree/${source[1]}` ||
      !/^[0-9]+(?:\.[0-9]+)*$/.test(family.version) ||
      family.license?.spdx !== 'OFL-1.1' ||
      !Array.isArray(family.faces) ||
      family.faces.length === 0
    )
      throw new Error('Invalid family');
    seenFamilies.add(family.family.toLowerCase());
    if (
      family.archive &&
      (!family.archive.url.startsWith(`https://github.com/${source[0]}/releases/download/`) ||
        !SHA.test(family.archive.sha256) ||
        !Number.isSafeInteger(family.archive.size) ||
        family.archive.size < 1 ||
        family.archive.size > 40 * 1024 * 1024)
    )
      throw new Error('Invalid source archive');
    const styles = new Set();
    for (const face of family.faces) {
      const design = `${face.weight}:${face.style}`;
      if (
        !Number.isInteger(face.weight) ||
        face.weight < 100 ||
        face.weight > 900 ||
        !['normal', 'italic'].includes(face.style) ||
        styles.has(design) ||
        !face.renderer?.path.endsWith('.ttf') ||
        !face.web?.path.endsWith('.woff2')
      )
        throw new Error('Invalid face');
      if (
        face.web.encoding &&
        (face.web.encoding !== 'woff2-fonttools-4.60.2-brotli-1.2.1' ||
          face.web.sourcePath !== face.renderer.sourcePath)
      )
        throw new Error('Invalid WOFF2 source');
      styles.add(design);
    }
    if (!family.license.path.endsWith('.txt')) throw new Error('Invalid license');
    for (const asset of [
      family.license,
      ...family.faces.flatMap((face) => [face.renderer, face.web]),
    ]) {
      if (
        !SHA.test(asset.sha256) ||
        !PATH.test(asset.path) ||
        !asset.path.includes(asset.sha256) ||
        !Number.isSafeInteger(asset.size) ||
        asset.size < 1 ||
        asset.size > 8 * 1024 * 1024 ||
        typeof asset.sourcePath !== 'string' ||
        !/^[A-Za-z0-9._/-]+$/.test(asset.sourcePath) ||
        asset.sourcePath.split('/').some((part) => part === '..' || part === '' || part === '.')
      )
        throw new Error(`Invalid asset: ${asset.path}`);
      if (paths.has(asset.path)) throw new Error(`Duplicate asset: ${asset.path}`);
      paths.add(asset.path);
      assets.push({ ...asset, family });
    }
  }
  return {
    catalog,
    families: catalog.families,
    assets,
    bytes,
    manifestPath: `catalogs/fonts/1.0.0/${hash(bytes)}.json`,
  };
}

async function bounded(response, limit) {
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const chunks = [];
  let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > limit) throw new Error('Asset exceeds expected size');
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

export async function prepare(directory, fetchImpl = fetch) {
  const { assets, bytes, manifestPath } = await readCatalog();
  const objects = new Map();
  const archives = new Map();
  const scratch = await mkdtemp(join(tmpdir(), 'ferrigura-font-catalog-'));
  try {
    for (const asset of assets) {
      const family = asset.family;
      let data;
      if (family.archive) {
        if (!archives.has(family.source)) {
          const response = await fetchImpl(family.archive.url, {
            redirect: 'follow',
            signal: AbortSignal.timeout(120000),
          });
          const archive = await bounded(response, family.archive.size);
          if (archive.length !== family.archive.size || hash(archive) !== family.archive.sha256)
            throw new Error(`Archive integrity mismatch: ${family.family}`);
          const file = join(scratch, `archive-${archives.size}.zip`);
          await writeFile(file, archive);
          archives.set(family.source, file);
        }
        data = execFileSync('unzip', ['-p', archives.get(family.source), asset.sourcePath], {
          maxBuffer: 8 * 1024 * 1024,
        });
      } else {
        const repository = new URL(family.source).pathname.split('/').slice(1, 3).join('/');
        const response = await fetchImpl(
          `https://raw.githubusercontent.com/${repository}/${family.sourceCommit}/${asset.sourcePath}`,
          { redirect: 'error', signal: AbortSignal.timeout(60000) }
        );
        data = await bounded(response, asset.size);
      }
      if (asset.encoding) {
        const input = join(scratch, 'input.ttf');
        const output = join(scratch, 'output.woff2');
        await writeFile(input, data);
        execFileSync(
          process.env.FONT_CATALOG_PYTHON || 'python3',
          ['-m', 'fontTools.ttLib.woff2', 'compress', '-q', '-o', output, input],
          { stdio: 'pipe' }
        );
        data = await readFile(output);
      }
      if (data.length !== asset.size || hash(data) !== asset.sha256)
        throw new Error(`Source integrity mismatch: ${asset.sourcePath}`);
      objects.set(asset.path, data);
    }
    objects.set(manifestPath, bytes);
    const legacy = await readFile(new URL('./lexica-1.0.0.json', import.meta.url));
    if (
      hash(legacy) !== 'a0ffba012a6170f3fbb1ecf0949ba8be64f6f30e62c604d330b1c2e707b32825' ||
      JSON.stringify(JSON.parse(legacy).families[0]) !==
        JSON.stringify((await readCatalog()).families[0])
    )
      throw new Error('Published Lexica catalog changed');
    objects.set(`catalogs/lexica-ultralegible/1.0.0/${hash(legacy)}.json`, legacy);
    for (const [path, data] of objects) {
      const file = join(resolve(directory), path);
      await mkdir(dirname(file), { recursive: true });
      await writeFile(file, data);
    }
    return objects;
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

export async function uploadImmutable({ accessKey, objects, fetchImpl = fetch }) {
  if (!accessKey || /[\r\n]/.test(accessKey))
    throw new Error('Storage configuration is incomplete');
  for (const [path, bytes] of objects) {
    const basename = path.split('/').at(-1);
    if (
      !(
        PATH.test(path) ||
        /^catalogs\/(?:fonts|lexica-ultralegible)\/1\.0\.0\/[0-9a-f]{64}\.json$/.test(path)
      ) ||
      !basename.startsWith(hash(bytes))
    )
      throw new Error(`Invalid immutable object: ${path}`);
  }
  for (const [path, bytes] of objects) {
    const url = `https://storage.bunnycdn.com/${ZONE}/ferrigura/${path}`;
    const get = () =>
      fetchImpl(url, {
        headers: { AccessKey: accessKey },
        redirect: 'error',
        signal: AbortSignal.timeout(60000),
      });
    const existing = await get();
    if (existing.status === 200) {
      if (!(await bounded(existing, bytes.length)).equals(bytes))
        throw new Error(`Existing object differs: ${path}`);
      continue;
    }
    await existing.body?.cancel();
    if (existing.status !== 404) throw new Error(`Storage lookup failed: HTTP ${existing.status}`);
    const type = path.endsWith('.woff2')
      ? 'font/woff2'
      : path.endsWith('.ttf')
        ? 'font/ttf'
        : path.endsWith('.json')
          ? 'application/json'
          : 'text/plain';
    const written = await fetchImpl(url, {
      method: 'PUT',
      headers: { AccessKey: accessKey, 'Content-Type': type },
      body: bytes,
      redirect: 'error',
      signal: AbortSignal.timeout(60000),
    });
    await written.body?.cancel();
    if (!written.ok) throw new Error(`Storage upload failed: HTTP ${written.status}`);
    if (!(await bounded(await get(), bytes.length)).equals(bytes))
      throw new Error(`Storage read-back mismatch: ${path}`);
  }
}

export async function verifyPublic(base, objects, fetchImpl = fetch) {
  const url = new URL(base);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash)
    throw new Error('Public base must be an HTTPS URL');
  for (const [path, bytes] of objects) {
    const response = await fetchImpl(`${base.replace(/\/$/, '')}/${path}`, {
      headers: { Origin: 'https://example.org' },
      redirect: 'error',
      signal: AbortSignal.timeout(60000),
    });
    const actual = await bounded(response, bytes.length);
    if (!actual.equals(bytes)) throw new Error(`Public integrity mismatch: ${path}`);
    if (path.endsWith('.woff2') && response.headers.get('access-control-allow-origin') !== '*')
      throw new Error(`Missing public CORS: ${path}`);
    if (
      path.endsWith('.woff2') &&
      (!/\bimmutable\b/.test(response.headers.get('cache-control') || '') ||
        !/\bmax-age=31536000\b/.test(response.headers.get('cache-control') || ''))
    )
      throw new Error(`Missing immutable cache: ${path}`);
    const expected = path.endsWith('.woff2')
      ? 'font/woff2'
      : path.endsWith('.ttf')
        ? 'font/ttf'
        : path.endsWith('.json')
          ? 'application/json'
          : 'text/plain';
    const actualType = (response.headers.get('content-type') || '').split(';')[0];
    if (
      actualType !== expected &&
      !(path.endsWith('.ttf') && actualType === 'application/octet-stream')
    )
      throw new Error(`Incorrect content type: ${path}`);
  }
}
