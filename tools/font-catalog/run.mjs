import { prepare, uploadImmutable, verifyPublic } from './catalog.mjs';
const mode = process.argv[2];
if (!['prepare', 'publish', 'verify'].includes(mode) || process.argv.length !== 3)
  throw new Error('Usage: node tools/font-catalog/run.mjs prepare|publish|verify');
const base = process.env.FONT_CATALOG_BASE_URL;
if (mode !== 'prepare') {
  const url = new URL(base);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash)
    throw new Error('Public base must be an HTTPS URL');
}
if (
  mode === 'publish' &&
  (!process.env.FONT_STORAGE_HOSTNAME ||
    !process.env.FONT_STORAGE_ZONE ||
    !process.env.FONT_STORAGE_ACCESS_KEY)
)
  throw new Error('Storage configuration is incomplete');
const objects = await prepare('target/font-catalog');
if (mode === 'publish')
  await uploadImmutable({
    hostname: process.env.FONT_STORAGE_HOSTNAME,
    zone: process.env.FONT_STORAGE_ZONE,
    accessKey: process.env.FONT_STORAGE_ACCESS_KEY,
    objects,
  });
if (mode !== 'prepare') await verifyPublic(base, objects);
console.log(`${mode}: ${objects.size} verified catalog objects`);
