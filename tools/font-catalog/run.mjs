import {
  prepare,
  PUBLIC_BASE,
  sharedStoragePassword,
  uploadImmutable,
  verifyPublic,
} from './catalog.mjs';
const mode = process.argv[2];
if (!['prepare', 'publish', 'verify'].includes(mode) || process.argv.length !== 3)
  throw new Error('Usage: node tools/font-catalog/run.mjs prepare|publish|verify');
const password =
  mode === 'publish' ? await sharedStoragePassword(process.env.BUNNY_API_KEY) : undefined;
const objects = await prepare('target/font-catalog');
if (mode === 'publish') await uploadImmutable({ accessKey: password, objects });
if (mode !== 'prepare') await verifyPublic(PUBLIC_BASE, objects);
console.log(`${mode}: ${objects.size} verified catalog objects`);
