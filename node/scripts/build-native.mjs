import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runBuild, stageAddon } from 'ferramenta-native/build';
import config from '../native-npm.config.mjs';
await import('./write-notices.mjs');

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const outputDir = path.join(root, 'target/node-addon', process.env.FERRIGURA_RUST_TARGET ?? 'host');
const target = process.env.FERRIGURA_RUST_TARGET;
runBuild({
  cwd: config.packageDir,
  manifestPath: path.join(root, 'crates/ferrigura-node/Cargo.toml'),
  profile: 'release-node',
  outputDir,
  target,
});
await stageAddon(config, { outputDir, target });
