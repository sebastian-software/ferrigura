import { NATIVE_TARGETS } from 'ferramenta-native/targets';
import path from 'node:path';
const root = import.meta.dirname;
export default {
  name: 'ferrigura',
  packageDir: path.join(root, 'ferrigura'),
  platformDir: path.join(root, 'platforms'),
  packagePattern: '@ferrigura/{target}',
  binaryPattern: 'ferrigura.node',
  runtimeFile: 'native-runtime.mjs',
  repositoryPlatformDir: 'node/platforms',
  licenses: ['LICENSE', 'NOTICE.md'],
  workspaceProtocol: true,
  pack: {
    required: ['index.mjs', 'index.d.mts', 'native-runtime.mjs'],
    maxUnpackedSize: 200_000,
    // Each platform package, a few megabytes above its addon under the release profile and below
    // the 0.4.0 packages, built without LTO and unstripped (docs/decisions/0016-npm-packages-via-napi.md).
    // A Windows DLL keeps its symbols in a separate PDB, so stripping does not shrink it, and x86-64
    // code is larger than arm64: win32-x64 is 18.9 MB (0.4.0: 23.8 MB), the others are under 16 MB
    // (darwin-arm64 12.0 MB, 0.4.0: 19.8 MB).
    platformMaxUnpackedSize: { default: 16_000_000, 'win32-x64-msvc': 22_000_000 },
  },
  consumer: {
    smoke: `import { render } from '@ferrigura/core'; const result = render('graph TD; A-->B'); if (!result.includes('<svg')) throw new Error('SVG mismatch');`,
  },
  release: {
    repository: 'https://github.com/sebastian-software/ferrigura',
    tagPrefix: 'v',
    ci: {
      requiredJobs: ['lint', 'test', 'rust', 'snapshots', 'port-geometry'],
      platformJobs: NATIVE_TARGETS.map(({ id }) => ({
        pattern: `^native-npm \\(.*${id}`,
        label: `native-npm ${id}`,
      })),
    },
    cargoFiles: ['Cargo.toml', 'crates/ferrigura-node/Cargo.toml'],
  },
};
